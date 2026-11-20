//! What was read from the files of each config folder, kept between
//! refreshes so that a file that did not change is not read again.
//!
//! What a file told is kept with the stamp the file had then: its size and
//! change time. A file with the same stamp is taken as not changed. It
//! lives in memory only, for as long as the app keeps its agent.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use crate::file_stamp::FileStamp;

/// The files read in one config folder, by their path.
pub(crate) type ReadFiles<T> = HashMap<PathBuf, ReadFile<T>>;

/// What was read from one file, and the stamp the file had then.
#[derive(Debug, Clone)]
pub(crate) struct ReadFile<T> {
    stamp: FileStamp,
    content: T,
}

impl<T> ReadFile<T> {
    /// The stamp the file had when it was read.
    pub(crate) fn stamp(&self) -> FileStamp {
        self.stamp
    }

    /// What was read from the file.
    pub(crate) fn content(&self) -> &T {
        &self.content
    }
}

/// The files read in each config folder on its last refresh, with what
/// each told, `T`.
///
/// An agent keeps one, shared with the sources it gives, behind a lock, so
/// it outlives each source.
#[derive(Debug)]
pub(crate) struct FileMemory<T> {
    folders: Mutex<HashMap<PathBuf, ReadFiles<T>>>,
}

impl<T> FileMemory<T> {
    /// Takes out what was read in `config_dir` on its last refresh. Empty
    /// when nothing was.
    pub(crate) fn take(&self, config_dir: &Path) -> ReadFiles<T> {
        let mut folders =
            self.folders.lock().unwrap_or_else(PoisonError::into_inner);

        folders.remove(config_dir).unwrap_or_default()
    }

    /// Keeps what was read in `config_dir` for its next refresh, in place
    /// of what was kept before.
    pub(crate) fn keep(&self, config_dir: &Path, files: ReadFiles<T>) {
        let mut folders =
            self.folders.lock().unwrap_or_else(PoisonError::into_inner);

        folders.insert(config_dir.to_path_buf(), files);
    }
}

impl<T: Clone> FileMemory<T> {
    /// What the file at `path` in `config_dir` tells while it has the
    /// stamp `stamp`: what the last refresh kept when the file did not
    /// change since, else what `read` reads now. What is kept stays in
    /// place, and what is read now is not kept.
    pub(crate) fn kept_or_read(
        &self,
        config_dir: &Path,
        path: &Path,
        stamp: FileStamp,
        read: impl FnOnce(&Path) -> T,
    ) -> T {
        let kept = {
            let folders =
                self.folders.lock().unwrap_or_else(PoisonError::into_inner);
            folders
                .get(config_dir)
                .and_then(|files| files.get(path))
                .filter(|file| file.stamp == stamp)
                .map(|file| file.content.clone())
        };

        match kept {
            Some(content) => content,
            None => read(path),
        }
    }
}

impl<T> Default for FileMemory<T> {
    fn default() -> Self {
        Self {
            folders: Mutex::new(HashMap::new()),
        }
    }
}

/// The file at `path`, which has the stamp `stamp` now: taken out of
/// `previous` when it had the same stamp there, else read now with `read`.
pub(crate) fn read_if_changed<T>(
    previous: &mut ReadFiles<T>,
    path: &Path,
    stamp: FileStamp,
    read: impl FnOnce(&Path) -> T,
) -> ReadFile<T> {
    match previous.remove(path) {
        Some(file) if file.stamp == stamp => file,
        _ => ReadFile {
            stamp,
            content: read(path),
        },
    }
}

/// The file at `path`, which has the stamp `stamp` now: taken out of
/// `previous` when it had the same stamp there, else read now with
/// `read`, which may fail.
pub(crate) fn try_read_if_changed<T, E>(
    previous: &mut ReadFiles<T>,
    path: &Path,
    stamp: FileStamp,
    read: impl FnOnce(&Path) -> Result<T, E>,
) -> Result<ReadFile<T>, E> {
    match previous.remove(path) {
        Some(file) if file.stamp == stamp => Ok(file),
        _ => Ok(ReadFile {
            stamp,
            content: read(path)?,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, write_file};

    /// The stamp of the file at `path` now.
    fn stamp_of(path: &Path) -> FileStamp {
        FileStamp::of(&std::fs::metadata(path).expect("read the file stamp"))
    }

    #[test]
    fn a_file_with_the_same_stamp_is_taken_from_what_was_kept() {
        let temp = TempDir::new();
        let path = temp.path().join("a.jsonl");
        write_file(&path, "first");
        let stamp = stamp_of(&path);
        let mut previous = ReadFiles::new();
        previous.insert(
            path.clone(),
            ReadFile {
                stamp,
                content: "kept",
            },
        );

        let same = read_if_changed(&mut previous, &path, stamp, |_| "read");
        write_file(&path, "longer now");
        let changed =
            read_if_changed(&mut previous, &path, stamp_of(&path), |_| "read");

        assert_eq!(same.content(), &"kept");
        assert_eq!(changed.content(), &"read");
    }

    #[test]
    fn what_was_kept_is_used_only_while_the_file_has_its_stamp() {
        let temp = TempDir::new();
        let config_dir = temp.path();
        let path = config_dir.join("a.jsonl");
        write_file(&path, "first");
        let stamp = stamp_of(&path);
        let memory = FileMemory::default();
        let mut files = ReadFiles::new();
        files.insert(
            path.clone(),
            ReadFile {
                stamp,
                content: "kept",
            },
        );
        memory.keep(config_dir, files);

        let kept = memory.kept_or_read(config_dir, &path, stamp, |_| "read");
        write_file(&path, "longer now");
        let read =
            memory.kept_or_read(config_dir, &path, stamp_of(&path), |_| "read");
        let other_folder = memory.kept_or_read(&path, &path, stamp, |_| "read");

        assert_eq!((kept, read, other_folder), ("kept", "read", "read"));
        assert_eq!(memory.take(config_dir).len(), 1);
        assert!(memory.take(config_dir).is_empty());
    }
}
