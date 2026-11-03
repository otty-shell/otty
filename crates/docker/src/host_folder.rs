//! Whether a folder on this computer exists, asked off the UI thread: a hung
//! network mount can block this check for a long time.

use std::path::Path;

/// Whether `path` exists on this computer. A path that cannot be checked
/// counts as missing, the same meaning as [`Path::exists`].
///
/// Needs a tokio runtime.
///
/// ```
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// use std::path::Path;
///
/// assert!(!otty_docker::host_folder_exists(Path::new("/otty/none")).await);
/// # }
/// ```
pub async fn host_folder_exists(path: &Path) -> bool {
    tokio::fs::try_exists(path).await.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[tokio::test]
    async fn a_folder_on_disk_exists() {
        let dir = TempDir::new();

        assert!(host_folder_exists(dir.path()).await);
    }

    #[tokio::test]
    async fn a_missing_path_does_not_exist() {
        let dir = TempDir::new();

        assert!(!host_folder_exists(&dir.path().join("missing")).await);
    }

    #[tokio::test]
    async fn a_path_that_cannot_be_checked_does_not_exist() {
        assert!(!host_folder_exists(Path::new("bad\0path")).await);
    }
}
