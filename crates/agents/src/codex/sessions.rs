//! The sessions of a Codex profile, read from the session files in
//! `<config>/sessions/` and `<config>/archived_sessions/`.

use std::collections::HashMap;
use std::fs::ReadDir;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use super::cache::{ReadFile, ReadFiles, SessionCache};
use super::session_file::read_session_meta;
use super::session_index::{IndexEntry, read_index};
use crate::error::SourceError;
use crate::file_stamp::FileStamp;
use crate::profile::Profile;
use crate::session::{AgentSession, SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::title::title_line;

/// The folders of the config folder that hold session files.
const SESSION_FOLDERS: [&str; 2] = ["sessions", "archived_sessions"];

/// Lists the sessions of a profile from its session files.
///
/// Each `rollout-*.jsonl` or `rollout-*.jsonl.zst` file in the two session
/// folders, at any depth, is one session. Only its first line is read, and
/// only when the file changed since the last refresh. The names come from
/// `session_index.jsonl`.
pub(super) struct SessionFiles {
    cache: Arc<SessionCache>,
}

impl SessionFiles {
    /// A source that remembers what it read in `cache`.
    pub(super) fn new(cache: Arc<SessionCache>) -> Self {
        Self { cache }
    }
}

impl SessionSource for SessionFiles {
    fn id(&self) -> SourceId {
        SourceId::SessionFiles
    }

    /// The sessions of the profile. A missing session folder means no
    /// sessions there; a folder inside it or a file that can not be read
    /// is skipped.
    fn sessions(
        &self,
        profile: &Profile,
    ) -> Result<Vec<AgentSession>, SourceError> {
        let config_dir = profile.config_dir();

        let paths = session_files(config_dir)?;
        let index = read_index(config_dir);

        let previous = self.cache.take(config_dir);
        let read_files: ReadFiles = paths
            .into_iter()
            .filter_map(|path| read_file(path, &previous))
            .collect();

        let sessions = newest_by_id(
            read_files.values().filter_map(|file| session(file, &index)),
        );

        self.cache.keep(config_dir, read_files);

        Ok(sessions)
    }
}

/// The session files of the config folder `config_dir`, in both session
/// folders and at any depth.
///
/// A missing session folder has none. A session folder that can not be
/// read fails the source; a folder inside it that can not be read is
/// skipped.
pub(super) fn session_files(
    config_dir: &Path,
) -> Result<Vec<PathBuf>, SourceError> {
    let mut paths = Vec::new();

    for folder in SESSION_FOLDERS {
        paths.extend(session_files_in(&config_dir.join(folder))?);
    }

    Ok(paths)
}

/// The session files in `folder` and in the folders inside it.
///
/// A missing folder has none. A folder that can not be read fails the
/// source; a folder inside it that can not be read is skipped.
fn session_files_in(folder: &Path) -> Result<Vec<PathBuf>, SourceError> {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(Vec::new());
        },
        Err(error) => {
            let shown = folder.display();
            return Err(SourceError::NotAvailable(format!(
                "can not read {shown}: {error}"
            )));
        },
    };

    let mut files = Vec::new();
    add_session_files(entries, &mut files);

    Ok(files)
}

/// Adds the session files among `entries`, and in the folders among them,
/// to `files`. Links are not followed, so a link can not lead in a loop.
fn add_session_files(entries: ReadDir, files: &mut Vec<PathBuf>) {
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();

        if file_type.is_dir() {
            if let Ok(inner) = std::fs::read_dir(&path) {
                add_session_files(inner, files);
            }
        } else if file_type.is_file() && is_session_file(&path) {
            files.push(path);
        }
    }
}

/// Whether `path` is named like a session file: `rollout-*.jsonl`, or
/// `rollout-*.jsonl.zst` when it is compressed.
fn is_session_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let plain_name = name.strip_suffix(".zst").unwrap_or(name);

    plain_name.starts_with("rollout-") && plain_name.ends_with(".jsonl")
}

/// What the file at `path` tells, taken from `previous` when the file did
/// not change since, else read now. `None` when the file is gone.
fn read_file(
    path: PathBuf,
    previous: &ReadFiles,
) -> Option<(PathBuf, ReadFile)> {
    let metadata = std::fs::metadata(&path).ok()?;
    let stamp = FileStamp::of(&metadata);

    let file = match previous.get(&path) {
        Some(file) if file.stamp() == stamp => file.clone(),
        _ => ReadFile::new(stamp, read_session_meta(&path)),
    };

    Some((path, file))
}

/// One session for each id: the one that changed last.
///
/// The same session can be found twice, in both session folders, or as a
/// plain and a compressed file while Codex compresses it.
fn newest_by_id(
    sessions: impl Iterator<Item = AgentSession>,
) -> Vec<AgentSession> {
    let mut by_id: HashMap<SessionId, AgentSession> = HashMap::new();

    for session in sessions {
        let is_newer = by_id
            .get(session.id())
            .is_none_or(|kept| session.changed_at() > kept.changed_at());
        if is_newer {
            by_id.insert(session.id().clone(), session);
        }
    }

    by_id.into_values().collect()
}

/// The session `file` tells about, named from `index`.
///
/// The title is the name in the index, else the id. The time is the newer
/// of the time in the index and the time the file last changed.
fn session(
    file: &ReadFile,
    index: &HashMap<String, IndexEntry>,
) -> Option<AgentSession> {
    let meta = file.meta()?;
    let entry = index.get(meta.id());

    let title = entry
        .and_then(IndexEntry::name)
        .and_then(title_line)
        .unwrap_or_else(|| meta.id().to_owned());
    let changed_at = entry
        .and_then(IndexEntry::updated_at)
        .max(file.stamp().changed_at())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let session = AgentSession::new(
        SessionId::new(meta.id()),
        meta.folder().to_path_buf(),
        title,
        changed_at,
    );

    match meta.branch() {
        Some(branch) => Some(session.with_branch(branch)),
        None => Some(session),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::testing::{
        TempDir, at, codex_profile, set_file_time, write_compressed, write_file,
    };

    const ID: &str = "019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b";
    const OTHER_ID: &str = "019a9000-1111-7222-8333-444455556666";
    const FILE_TIME: u64 = 1_794_000_000;
    const MESSAGE: &str = r#"{"timestamp":"2026-11-16T10:00:05.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Fix the tabs"}]}}"#;

    fn meta_line(id: &str, folder: &str) -> String {
        format!(
            r#"{{"timestamp":"2026-11-16T10:00:00.000Z","type":"session_meta","payload":{{"id":"{id}","cwd":"{folder}","source":"cli","git":{{"branch":"main"}}}}}}"#
        )
    }

    fn helper_meta_line(id: &str) -> String {
        format!(
            r#"{{"type":"session_meta","payload":{{"id":"{id}","cwd":"/home/user/otty","source":{{"subagent":{{"thread_spawn":{{"depth":1}}}}}}}}}}"#
        )
    }

    /// A made-up Codex config folder.
    struct ConfigDir {
        temp: TempDir,
        source: SessionFiles,
    }

    impl ConfigDir {
        fn path(&self) -> PathBuf {
            self.temp.path().join(".codex-work")
        }

        fn new() -> Self {
            let config = Self {
                temp: TempDir::new(),
                source: SessionFiles::new(Arc::default()),
            };
            std::fs::create_dir_all(config.path().join("sessions"))
                .expect("create sessions folder");
            config
        }

        fn session_file(&self, folder: &str, id: &str, ext: &str) -> PathBuf {
            self.path()
                .join(folder)
                .join(format!("rollout-2026-11-16T10-00-00-{id}.{ext}"))
        }

        /// Writes a plain session file with `first_line` and a message.
        fn write_plain(&self, folder: &str, id: &str, first_line: &str) {
            let path = self.session_file(folder, id, "jsonl");
            write_file(&path, &format!("{first_line}\n{MESSAGE}\n"));
            set_file_time(&path, at(FILE_TIME));
        }

        /// Writes a compressed session file with `first_line` and a
        /// message.
        fn write_compressed(&self, folder: &str, id: &str, first_line: &str) {
            let path = self.session_file(folder, id, "jsonl.zst");
            write_compressed(&path, &format!("{first_line}\n{MESSAGE}\n"));
            set_file_time(&path, at(FILE_TIME));
        }

        fn write_index(&self, lines: &[&str]) {
            write_file(
                &self.path().join("session_index.jsonl"),
                &lines.join("\n"),
            );
        }

        fn sessions(&self) -> Vec<AgentSession> {
            let profile = codex_profile(&self.path());

            let mut sessions = self
                .source
                .sessions(&profile)
                .expect("the session files are read");
            sessions.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str()));
            sessions
        }
    }

    fn ids(sessions: &[AgentSession]) -> Vec<&str> {
        sessions
            .iter()
            .map(|session| session.id().as_str())
            .collect()
    }

    fn plain_session(id: &str) -> AgentSession {
        AgentSession::new(
            SessionId::new(id),
            PathBuf::from("/home/user/otty"),
            id,
            at(FILE_TIME),
        )
        .with_branch("main")
    }

    #[test]
    fn a_plain_session_has_its_folder_branch_and_file_time() {
        let config = ConfigDir::new();
        config.write_plain(
            "sessions/2026/11/16",
            ID,
            &meta_line(ID, "/home/user/otty"),
        );

        assert_eq!(config.sessions(), [plain_session(ID)]);
    }

    #[test]
    fn a_compressed_session_is_listed() {
        let config = ConfigDir::new();
        config.write_compressed(
            "sessions/2025/01/02",
            ID,
            &meta_line(ID, "/home/user/otty"),
        );

        assert_eq!(config.sessions(), [plain_session(ID)]);
    }

    #[test]
    fn archived_sessions_are_listed() {
        let config = ConfigDir::new();
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/a"));
        config.write_compressed(
            "archived_sessions",
            OTHER_ID,
            &meta_line(OTHER_ID, "/b"),
        );

        let sessions = config.sessions();

        assert_eq!(ids(&sessions), [ID, OTHER_ID]);
        assert_eq!(sessions[1].folder(), Path::new("/b"));
    }

    #[test]
    fn a_helper_agents_session_is_not_listed() {
        let config = ConfigDir::new();
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/a"));
        config.write_plain(
            "sessions/2026/11/16",
            OTHER_ID,
            &helper_meta_line(OTHER_ID),
        );

        assert_eq!(ids(&config.sessions()), [ID]);
    }

    #[test]
    fn the_name_and_the_time_come_from_the_index() {
        let config = ConfigDir::new();
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/a"));
        config.write_plain(
            "sessions/2026/11/16",
            OTHER_ID,
            &meta_line(OTHER_ID, "/b"),
        );
        let blank_name = format!(
            r#"{{"id":"{OTHER_ID}","thread_name":"  ","updated_at":"2026-11-16T10:00:00Z"}}"#
        );
        config.write_index(&[
            &format!(
                r#"{{"id":"{ID}","thread_name":"Fix the tabs","updated_at":"2026-11-16T10:00:00Z"}}"#
            ),
            &format!(
                r#"{{"id":"{ID}","thread_name":"Stop the tab flicker","updated_at":"2026-11-16T10:05:00Z"}}"#
            ),
            &blank_name,
        ]);

        let sessions = config.sessions();

        assert_eq!(sessions[0].title(), "Stop the tab flicker");
        assert_eq!(sessions[0].changed_at(), at(1_794_823_500));
        assert_eq!(sessions[1].title(), OTHER_ID);
        assert_eq!(sessions[1].changed_at(), at(1_794_823_200));
    }

    #[test]
    fn the_file_time_wins_when_it_is_newer_than_the_index() {
        let config = ConfigDir::new();
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/a"));
        config.write_index(&[&format!(
            r#"{{"id":"{ID}","thread_name":"Fix the tabs","updated_at":"2026-01-01T00:00:00Z"}}"#
        )]);

        let sessions = config.sessions();

        assert_eq!(sessions[0].title(), "Fix the tabs");
        assert_eq!(sessions[0].changed_at(), at(FILE_TIME));
    }

    #[test]
    fn a_session_found_twice_is_listed_once_with_its_newest_file() {
        let config = ConfigDir::new();
        let newer = config.session_file("archived_sessions", ID, "jsonl.zst");
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/a"));
        config.write_compressed(
            "sessions/2026/11/16",
            ID,
            &meta_line(ID, "/a"),
        );
        config.write_compressed("archived_sessions", ID, &meta_line(ID, "/b"));
        set_file_time(&newer, at(FILE_TIME + 60));
        config.write_plain(
            "sessions/2026/11/16",
            OTHER_ID,
            &meta_line(OTHER_ID, "/c"),
        );

        let sessions = config.sessions();

        assert_eq!(ids(&sessions), [ID, OTHER_ID]);
        assert_eq!(sessions[0].folder(), Path::new("/b"));
        assert_eq!(sessions[0].changed_at(), at(FILE_TIME + 60));
    }

    #[test]
    fn a_file_with_a_broken_first_line_is_skipped() {
        let config = ConfigDir::new();
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/a"));
        config.write_plain(
            "sessions/2026/11/16",
            OTHER_ID,
            "{\"type\":\"session_meta\",\"payload\":",
        );
        write_file(
            &config.session_file("sessions/2026/11/15", "x", "jsonl.zst"),
            "not compressed at all",
        );

        assert_eq!(ids(&config.sessions()), [ID]);
    }

    #[test]
    fn other_files_and_folders_are_not_read() {
        let config = ConfigDir::new();
        let line = meta_line(ID, "/a");
        write_file(&config.path().join("sessions/notes.txt"), &line);
        write_file(&config.path().join("sessions/2026/history.jsonl"), &line);
        write_file(&config.path().join("history.jsonl"), &line);
        std::fs::create_dir_all(config.session_file(
            "sessions/2026/11/16",
            ID,
            "jsonl",
        ))
        .expect("create a folder that looks like a session file");

        assert_eq!(config.sessions(), []);
    }

    #[test]
    fn a_missing_sessions_folder_gives_an_empty_list() {
        let temp = TempDir::new();
        let no_sessions = temp.path().join(".codex-work");
        std::fs::create_dir_all(&no_sessions).expect("create config folder");
        let missing = temp.path().join(".codex-gone");
        let source = SessionFiles::new(Arc::default());

        let in_empty_folder = source.sessions(&codex_profile(&no_sessions));
        let in_missing_folder = source.sessions(&codex_profile(&missing));

        assert_eq!(in_empty_folder, Ok(Vec::new()));
        assert_eq!(in_missing_folder, Ok(Vec::new()));
    }

    #[test]
    fn a_sessions_path_that_is_not_a_folder_is_not_available() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".codex-work");
        write_file(&config_dir.join("archived_sessions"), "not a folder");
        let source = SessionFiles::new(Arc::default());

        let result = source.sessions(&codex_profile(&config_dir));

        assert!(matches!(result, Err(SourceError::NotAvailable(_))));
    }

    #[test]
    fn a_file_that_did_not_change_is_not_read_twice() {
        let config = ConfigDir::new();
        config.write_plain(
            "sessions/2026/11/16",
            ID,
            &meta_line(ID, "/home/user/aaaa"),
        );

        let first = config.sessions();
        config.write_plain(
            "sessions/2026/11/16",
            ID,
            &meta_line(ID, "/home/user/bbbb"),
        );
        let second = config.sessions();

        assert_eq!(first[0].folder(), Path::new("/home/user/aaaa"));
        assert_eq!(second[0].folder(), Path::new("/home/user/aaaa"));
    }

    #[test]
    fn a_file_that_changed_is_read_again() {
        let config = ConfigDir::new();
        let path = config.session_file("sessions/2026/11/16", ID, "jsonl");
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/a"));

        let first = config.sessions();
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/bb"));
        let grown = config.sessions();
        config.write_plain("sessions/2026/11/16", ID, &meta_line(ID, "/cc"));
        set_file_time(&path, at(FILE_TIME + 60));
        let touched = config.sessions();
        std::fs::remove_file(&path).expect("remove the session file");
        let removed = config.sessions();

        assert_eq!(first[0].folder(), Path::new("/a"));
        assert_eq!(grown[0].folder(), Path::new("/bb"));
        assert_eq!(touched[0].folder(), Path::new("/cc"));
        assert_eq!(touched[0].changed_at(), at(FILE_TIME + 60));
        assert_eq!(removed, []);
    }
}
