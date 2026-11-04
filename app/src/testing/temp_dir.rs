//! A folder for one test under the system temp folder.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

/// A fresh folder under the system temp folder for one test, removed when the
/// test ends, also when it fails.
///
/// Each one has its own path, so the end of one test can not take away what
/// another test running at the same time saved.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Creates a folder with `name` in its path, to tell where it comes from.
    pub(crate) fn new(name: &str) -> Self {
        let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let path = std::env::temp_dir()
            .join(format!("otty-test-{name}-{process}-{number}"));

        std::fs::create_dir_all(&path).expect("create test dir");

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_two_folders_then_they_have_their_own_paths() {
        let first = TempDir::new("same");
        let second = TempDir::new("same");

        assert_ne!(first.path(), second.path());
        assert!(first.path().is_dir());
        assert!(second.path().is_dir());
    }

    #[test]
    fn given_a_folder_with_a_file_when_dropped_then_it_is_gone() {
        let folder = TempDir::new("dropped");
        let path = folder.path().to_path_buf();
        std::fs::write(path.join("file"), "x").expect("write");

        drop(folder);

        assert!(!path.exists());
    }

    #[test]
    fn given_a_test_that_fails_then_its_folder_is_gone() {
        let mut path = None;

        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let folder = TempDir::new("failing");
                path = Some(folder.path().to_path_buf());
                panic!("the test fails here");
            }));

        assert!(outcome.is_err());
        assert!(!path.expect("the folder was made").exists());
    }
}
