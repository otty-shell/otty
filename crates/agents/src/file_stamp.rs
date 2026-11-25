//! The stamp of a file: what tells that it changed since it was last read.

use std::fs::Metadata;
use std::time::SystemTime;

/// The size of a file and the time it last changed. A file with the same
/// stamp as before is taken as not changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStamp {
    size: u64,
    changed_at: Option<SystemTime>,
}

impl FileStamp {
    /// The time the file last changed, when the system tells it.
    pub(crate) fn changed_at(&self) -> Option<SystemTime> {
        self.changed_at
    }

    /// The stamp of a file from its metadata.
    pub(crate) fn of(metadata: &Metadata) -> Self {
        Self {
            size: metadata.len(),
            changed_at: metadata.modified().ok(),
        }
    }
}
