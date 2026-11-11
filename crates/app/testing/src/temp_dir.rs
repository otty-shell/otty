//! A folder for one test under the system temp folder, for the tests that
//! read and write files: the tests of the binary and the drawing tests that
//! keep a reference picture.

#![allow(
    clippy::disallowed_methods,
    reason = "the tests prepare and remove the folder they work in"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

/// A fresh folder under the system temp folder for one test, removed when the
/// test ends, also when it fails.
///
/// Each one has its own path, so the end of one test can not take away what
/// another test running at the same time saved.
///
/// # Example
///
/// ```
/// use otty_app_testing::temp_dir::TempDir;
///
/// let folder = TempDir::new("example");
/// let path = folder.path().to_path_buf();
/// assert!(path.is_dir());
///
/// drop(folder);
/// assert!(!path.exists());
/// ```
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Return the path of the folder.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a folder with `name` in its path, to tell where it comes from.
    pub fn new(name: &str) -> Self {
        let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let path = std::env::temp_dir()
            .join(format!("otty-app-test-{name}-{process}-{number}"));

        std::fs::create_dir_all(&path).expect("create test dir");

        Self { path }
    }

    /// Remove the folder before the test ends, for a test about a folder
    /// that went away.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_app_testing::temp_dir::TempDir;
    ///
    /// let folder = TempDir::new("gone");
    /// folder.remove_now();
    ///
    /// assert!(!folder.path().exists());
    /// ```
    pub fn remove_now(&self) {
        std::fs::remove_dir_all(&self.path).expect("remove test dir");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
