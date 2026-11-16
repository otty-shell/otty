//! What was read from the Codex session files, kept between refreshes so
//! that a file that did not change is not read again.
//!
//! It lives in memory only, for as long as the app keeps its `Codex`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use super::session_file::SessionMeta;
use crate::file_stamp::FileStamp;

/// The session files read in one config folder, by their path.
pub(super) type ReadFiles = HashMap<PathBuf, ReadFile>;

/// What was read from one session file, and the stamp the file had then.
#[derive(Debug, Clone)]
pub(super) struct ReadFile {
    stamp: FileStamp,
    meta: Option<SessionMeta>,
}

impl ReadFile {
    /// The stamp the file had when it was read.
    pub(super) fn stamp(&self) -> FileStamp {
        self.stamp
    }

    /// What the file told about its session. `None`: the file gives no
    /// session, because it is broken or belongs to a helper agent.
    pub(super) fn meta(&self) -> Option<&SessionMeta> {
        self.meta.as_ref()
    }

    /// What was read from a file with the stamp `stamp`.
    pub(super) fn new(stamp: FileStamp, meta: Option<SessionMeta>) -> Self {
        Self { stamp, meta }
    }
}

/// The session files read in each config folder on its last refresh.
///
/// It is shared by `Codex` and the sources it gives, behind a lock, so it
/// outlives each source.
#[derive(Debug, Default)]
pub(super) struct SessionCache {
    folders: Mutex<HashMap<PathBuf, ReadFiles>>,
}

impl SessionCache {
    /// Takes out what was read in `config_dir` on its last refresh. Empty
    /// when nothing was.
    pub(super) fn take(&self, config_dir: &Path) -> ReadFiles {
        let mut folders =
            self.folders.lock().unwrap_or_else(PoisonError::into_inner);

        folders.remove(config_dir).unwrap_or_default()
    }

    /// Keeps what was read in `config_dir` for its next refresh, in place
    /// of what was kept before.
    pub(super) fn keep(&self, config_dir: &Path, files: ReadFiles) {
        let mut folders =
            self.folders.lock().unwrap_or_else(PoisonError::into_inner);

        folders.insert(config_dir.to_path_buf(), files);
    }
}
