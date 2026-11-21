//! The sessions of one config folder over the files of its session places
//! (issue 25), with what each file told kept between refreshes in a
//! [`FileMemory`], as the tokens are counted.
//!
//! A file with the same stamp as on the last read is not read again; a
//! file that is gone drops its sessions. What several files tell about one
//! session id is joined into one session, so a file may tell only part of
//! it, such as an index of the sessions' titles.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::SourceError;
use crate::file_memory::{FileMemory, ReadFiles, try_read_if_changed};
use crate::file_stamp::FileStamp;
use crate::session::{AgentSession, SessionId};

/// What one file told about one session: its id, and the folder and the
/// title when the file names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundSession {
    id: String,
    folder: Option<PathBuf>,
    title: Option<String>,
    changed_at: SystemTime,
}

impl FoundSession {
    /// The session `id`, which changed at `changed_at` as the file tells.
    pub(crate) fn new(id: impl Into<String>, changed_at: SystemTime) -> Self {
        Self {
            id: id.into(),
            folder: None,
            title: None,
            changed_at,
        }
    }

    /// The same session, which ran in `folder`.
    pub(crate) fn with_folder(self, folder: Option<PathBuf>) -> Self {
        Self { folder, ..self }
    }

    /// The same session, titled `title`.
    pub(crate) fn with_title(self, title: Option<String>) -> Self {
        Self { title, ..self }
    }
}

/// What the files tell about one session id so far: the latest time, and
/// the folder and the title of the latest record that names them, each
/// with the time of its record.
struct JoinedSession {
    changed_at: SystemTime,
    folder: Option<(SystemTime, PathBuf)>,
    title: Option<(SystemTime, String)>,
}

impl JoinedSession {
    /// What `found` tells, joined to what was told before.
    fn add(&mut self, found: &FoundSession) {
        self.changed_at = self.changed_at.max(found.changed_at);

        if let Some(folder) = &found.folder {
            keep_latest(&mut self.folder, found.changed_at, folder);
        }
        if let Some(title) = &found.title {
            keep_latest(&mut self.title, found.changed_at, title);
        }
    }
}

/// The sessions the files `paths` of `config_dir` tell about, newest
/// first.
///
/// A file is read with `read_file` only when it is new or changed since
/// the last read kept in `memory`; a file that is gone is skipped and
/// forgotten. When `read_file` fails, so does the read: the files read
/// before it are kept, and the one that failed is read again next time.
pub(crate) fn read_session_files(
    memory: &FileMemory<Vec<FoundSession>>,
    config_dir: &Path,
    paths: Vec<PathBuf>,
    mut read_file: impl FnMut(&Path) -> Result<Vec<FoundSession>, SourceError>,
) -> Result<Vec<AgentSession>, SourceError> {
    let mut previous = memory.take(config_dir);

    let mut read = ReadFiles::new();
    for path in paths {
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        let stamp = FileStamp::of(&metadata);

        match try_read_if_changed(&mut previous, &path, stamp, &mut read_file) {
            Ok(file) => {
                read.insert(path, file);
            },
            Err(error) => {
                read.extend(previous);
                memory.keep(config_dir, read);
                return Err(error);
            },
        }
    }

    let sessions = joined(read.values().map(|file| file.content()));
    memory.keep(config_dir, read);

    Ok(sessions)
}

/// The sessions `files` tell about, one for each id, newest first; of two
/// that changed at the same time, the one with the smaller id first. A
/// session no file names a folder for is left out: it can not be
/// continued. One no file titles is titled with its id.
fn joined<'a>(
    files: impl Iterator<Item = &'a Vec<FoundSession>>,
) -> Vec<AgentSession> {
    let mut by_id: BTreeMap<&str, JoinedSession> = BTreeMap::new();
    for found in files.flatten() {
        by_id
            .entry(found.id.as_str())
            .or_insert(JoinedSession {
                changed_at: found.changed_at,
                folder: None,
                title: None,
            })
            .add(found);
    }

    let mut sessions: Vec<AgentSession> = by_id
        .into_iter()
        .filter_map(|(id, session)| {
            let (_, folder) = session.folder?;
            let title = session.title.map_or_else(|| id.to_string(), |t| t.1);

            Some(AgentSession::new(
                SessionId::new(id),
                folder,
                title,
                session.changed_at,
            ))
        })
        .collect();
    sessions.sort_by(|one, other| {
        other
            .changed_at()
            .cmp(&one.changed_at())
            .then_with(|| one.id().as_str().cmp(other.id().as_str()))
    });

    sessions
}

/// Keeps `value`, told by a record of `at`, in `kept` when no value was
/// kept or the kept one was told earlier. Of two told at the same time,
/// the greater value is kept, so the order the files are read in does not
/// change the list.
fn keep_latest<T: Ord + Clone>(
    kept: &mut Option<(SystemTime, T)>,
    at: SystemTime,
    value: &T,
) {
    let is_later = kept.as_ref().is_none_or(|(kept_at, kept_value)| {
        (at, value) > (*kept_at, kept_value)
    });

    if is_later {
        *kept = Some((at, value.clone()));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::error::Unavailable;
    use crate::testing::{NOW, TempDir, at, set_file_time, write_file};

    /// A made-up config folder whose session files each hold lines of
    /// `id folder title seconds`, where `-` stands for "not named".
    struct Folder {
        temp: TempDir,
        memory: FileMemory<Vec<FoundSession>>,
        opened: RefCell<Vec<String>>,
    }

    impl Folder {
        fn new() -> Self {
            Self {
                temp: TempDir::new(),
                memory: FileMemory::default(),
                opened: RefCell::new(Vec::new()),
            }
        }

        fn path(&self, name: &str) -> PathBuf {
            self.temp.path().join(name)
        }

        fn write(&self, name: &str, lines: &[&str], changed: u64) {
            write_file(&self.path(name), &lines.join("\n"));
            set_file_time(&self.path(name), at(changed));
        }

        /// The sessions of the files `names`, read with what this folder
        /// kept, and the files that were opened.
        fn read(&self, names: &[&str]) -> (Vec<AgentSession>, Vec<String>) {
            let sessions = self
                .try_read(&self.memory, names)
                .expect("the files are read");

            (sessions, self.opened.borrow().clone())
        }

        /// The sessions of the files `names`, read as the first time.
        fn read_whole(&self, names: &[&str]) -> Vec<AgentSession> {
            self.try_read(&FileMemory::default(), names)
                .expect("the files are read")
        }

        fn try_read(
            &self,
            memory: &FileMemory<Vec<FoundSession>>,
            names: &[&str],
        ) -> Result<Vec<AgentSession>, SourceError> {
            let paths = names.iter().map(|name| self.path(name)).collect();
            self.opened.borrow_mut().clear();

            read_session_files(memory, self.temp.path(), paths, |path| {
                self.read_file(path)
            })
        }

        fn read_file(
            &self,
            path: &Path,
        ) -> Result<Vec<FoundSession>, SourceError> {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.opened.borrow_mut().push(name);

            let content = std::fs::read_to_string(path).unwrap_or_default();
            if content.trim() == "broken" {
                return Err(SourceError::NotAvailable(
                    Unavailable::NoSessionFiles,
                ));
            }

            let named =
                |field: &str| (field != "-").then(|| field.replace('_', " "));
            let found = content
                .lines()
                .filter_map(|line| {
                    let fields: Vec<&str> = line.split(' ').collect();
                    let [id, folder, title, seconds] = fields[..] else {
                        return None;
                    };
                    let seconds = seconds.parse().ok()?;

                    Some(
                        FoundSession::new(id, at(seconds))
                            .with_folder(named(folder).map(PathBuf::from))
                            .with_title(named(title)),
                    )
                })
                .collect();
            Ok(found)
        }
    }

    fn ids(sessions: &[AgentSession]) -> Vec<&str> {
        sessions
            .iter()
            .map(|session| session.id().as_str())
            .collect()
    }

    #[test]
    fn a_second_read_with_no_changed_file_reads_no_file() {
        let folder = Folder::new();
        folder.write("a.jsonl", &["a /shop Fix_the_cart 10"], NOW);
        folder.write("b.jsonl", &["b /blog Notes 20"], NOW);

        let (first, opened_first) = folder.read(&["a.jsonl", "b.jsonl"]);
        let (second, opened_second) = folder.read(&["a.jsonl", "b.jsonl"]);

        assert_eq!(opened_first.len(), 2);
        assert!(opened_second.is_empty(), "{opened_second:?}");
        assert_eq!(second, first);
        assert_eq!(ids(&second), ["b", "a"]);
        assert_eq!(second[1].title(), "Fix the cart");
        assert_eq!(second[1].folder(), Path::new("/shop"));
        assert_eq!(second[1].changed_at(), at(10));
    }

    #[test]
    fn a_changed_file_is_read_again_alone() {
        let folder = Folder::new();
        folder.write("a.jsonl", &["a /shop Old 10"], NOW);
        folder.write("b.jsonl", &["b /blog Notes 20"], NOW);

        let _ = folder.read(&["a.jsonl", "b.jsonl"]);
        folder.write("a.jsonl", &["a /shop New_title 30"], NOW);
        let (sessions, opened) = folder.read(&["a.jsonl", "b.jsonl"]);

        assert_eq!(opened, ["a.jsonl"]);
        assert_eq!(ids(&sessions), ["a", "b"]);
        assert_eq!(sessions[0].title(), "New title");
    }

    #[test]
    fn a_new_file_adds_its_sessions_and_a_deleted_one_removes_them() {
        let folder = Folder::new();
        folder.write("a.jsonl", &["a /shop A 10"], NOW);
        folder.write("b.jsonl", &["b /blog B 20"], NOW);

        let _ = folder.read(&["a.jsonl", "b.jsonl"]);
        folder.write("c.jsonl", &["c /site C 30"], NOW);
        std::fs::remove_file(folder.path("b.jsonl")).expect("remove b");
        let (sessions, opened) =
            folder.read(&["a.jsonl", "b.jsonl", "c.jsonl"]);
        let (unlisted, _) = folder.read(&["a.jsonl"]);

        assert_eq!(opened, ["c.jsonl"]);
        assert_eq!(ids(&sessions), ["c", "a"]);
        assert_eq!(ids(&unlisted), ["a"]);
    }

    #[test]
    fn the_list_after_changes_is_the_same_as_a_whole_read() {
        let folder = Folder::new();
        let names = ["a.jsonl", "b.jsonl", "index.jsonl"];
        folder.write("a.jsonl", &["a /shop - 10"], NOW);
        folder.write("b.jsonl", &["b /blog - 20"], NOW);
        folder.write("index.jsonl", &["a - Fix_the_cart 15"], NOW);

        let _ = folder.read(&names);
        folder.write("b.jsonl", &["b /blog - 40"], NOW + 1);
        folder.write(
            "index.jsonl",
            &["a - Fix_the_cart 15", "b - Notes 25"],
            NOW + 1,
        );
        let (sessions, _) = folder.read(&names);

        assert_eq!(sessions, folder.read_whole(&names));
        assert_eq!(ids(&sessions), ["b", "a"]);
        assert_eq!(sessions[0].title(), "Notes");
    }

    /// What two files tell about one id joins: the latest time, and the
    /// folder and the title of the latest record that names them.
    #[test]
    fn what_several_files_tell_about_one_session_is_joined() {
        let folder = Folder::new();
        folder.write("old.jsonl", &["a /old - 10"], NOW);
        folder.write("new.jsonl", &["a /new - 20"], NOW);
        folder.write("index.jsonl", &["a - Fix_the_cart 30"], NOW);

        let (sessions, _) =
            folder.read(&["old.jsonl", "new.jsonl", "index.jsonl"]);

        assert_eq!(
            sessions,
            [AgentSession::new(
                SessionId::new("a"),
                PathBuf::from("/new"),
                "Fix the cart",
                at(30),
            )]
        );
    }

    #[test]
    fn a_session_with_no_folder_is_left_out_and_one_with_no_title_has_its_id() {
        let folder = Folder::new();
        folder.write("a.jsonl", &["a /shop - 10", "b - Gone 20"], NOW);

        let (sessions, _) = folder.read(&["a.jsonl"]);

        assert_eq!(ids(&sessions), ["a"]);
        assert_eq!(sessions[0].title(), "a");
    }

    #[test]
    fn a_read_that_failed_fails_and_the_file_is_read_again() {
        let folder = Folder::new();
        folder.write("a.jsonl", &["a /shop A 10"], NOW);
        folder.write("b.jsonl", &["broken"], NOW);

        let failed = folder.try_read(&folder.memory, &["a.jsonl", "b.jsonl"]);
        folder.write("b.jsonl", &["b /blog B 20"], NOW);
        let (sessions, opened) = folder.read(&["a.jsonl", "b.jsonl"]);

        assert!(failed.is_err());
        assert_eq!(opened, ["b.jsonl"]);
        assert_eq!(ids(&sessions), ["b", "a"]);
    }
}
