//! A folder for one test under the system temp folder, for the tests of the
//! effects that read and write files.

#![allow(
    clippy::disallowed_methods,
    reason = "the tests of effects/ prepare the files the effects read"
)]

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
            .join(format!("otty-app-test-{name}-{process}-{number}"));

        std::fs::create_dir_all(&path).expect("create test dir");

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
