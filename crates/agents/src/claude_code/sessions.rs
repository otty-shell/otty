//! The sessions of a Claude Code profile, read from the session files in
//! `<config>/projects/`.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::session_file::read_session;
use crate::error::SourceError;
use crate::profile::Profile;
use crate::session::{AgentSession, SessionSource};
use crate::source_id::SourceId;

/// Lists the sessions of a profile from its session files.
///
/// Each folder in `<config>/projects/` is one project, and each `*.jsonl`
/// file directly inside it is one session. The sessions of helper agents
/// are kept deeper, in `<session id>/subagents/`, so they are not listed.
pub(super) struct SessionFiles;

impl SessionSource for SessionFiles {
    fn id(&self) -> SourceId {
        SourceId::SessionFiles
    }

    /// The sessions of the profile. A missing `projects` folder means no
    /// sessions yet; a project folder or file that can not be read is
    /// skipped.
    fn sessions(
        &self,
        profile: &Profile,
    ) -> Result<Vec<AgentSession>, SourceError> {
        let projects_dir = profile.config_dir().join("projects");

        let project_dirs = match std::fs::read_dir(&projects_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Vec::new());
            },
            Err(error) => {
                let shown = projects_dir.display();
                return Err(SourceError::NotAvailable(format!(
                    "can not read {shown}: {error}"
                )));
            },
        };

        let sessions = project_dirs
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .flat_map(|project_dir| project_sessions(&project_dir))
            .collect();

        Ok(sessions)
    }
}

/// The sessions in one project folder, or none when it can not be read.
fn project_sessions(project_dir: &Path) -> Vec<AgentSession> {
    let Ok(entries) = std::fs::read_dir(project_dir) else {
        return Vec::new();
    };
    let fallback_folder = decoded_folder(project_dir);

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_session_file(path))
        .filter_map(|path| read_session(&path, &fallback_folder))
        .collect()
}

fn is_session_file(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|ext| ext == "jsonl")
}

/// The project folder guessed from the name of its folder in `projects`.
///
/// Claude Code names that folder after the project path with `/` and `.`
/// replaced by `-`, so the guess turns every `-` back into `/`. It is used
/// only when no record of a session tells the real folder.
pub(super) fn decoded_folder(project_dir: &Path) -> PathBuf {
    let name = project_dir
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();

    PathBuf::from(name.replace('-', "/"))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::session::SessionId;
    use crate::testing::{TempDir, at, claude_profile, write_file};

    const ID: &str = "3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b";

    const USER: &str = r#"{"type":"user","cwd":"/home/user/otty","sessionId":"3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b","gitBranch":"main","timestamp":"2026-11-16T10:00:00.000Z","message":{"role":"user","content":"Fix the tab bar\nIt flickers on resize"}}"#;
    const ASSISTANT: &str = r#"{"type":"assistant","cwd":"/home/user/otty","sessionId":"3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b","gitBranch":"main","timestamp":"2026-11-16T10:05:00.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Done."}]}}"#;
    const AI_TITLE: &str = r#"{"type":"ai-title","aiTitle":"Stop the tab bar flicker","sessionId":"3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b"}"#;
    const LAST_PROMPT: &str = r#"{"type":"last-prompt","lastPrompt":"Run the tests","sessionId":"3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b"}"#;

    /// A made-up config folder with a `projects` folder inside.
    struct ConfigDir {
        temp: TempDir,
    }

    impl ConfigDir {
        fn path(&self) -> PathBuf {
            self.temp.path().join(".claude-work")
        }

        fn new() -> Self {
            let config = Self {
                temp: TempDir::new(),
            };
            std::fs::create_dir_all(config.path().join("projects"))
                .expect("create projects folder");
            config
        }

        fn session_file(&self, project: &str, id: &str) -> PathBuf {
            self.path()
                .join("projects")
                .join(project)
                .join(format!("{id}.jsonl"))
        }

        fn write_session(&self, project: &str, id: &str, lines: &[&str]) {
            let mut content = lines.join("\n");
            content.push('\n');
            write_file(&self.session_file(project, id), &content);
        }

        fn sessions(&self) -> Vec<AgentSession> {
            let profile = claude_profile(&self.path());

            let mut sessions = SessionFiles
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

    #[test]
    fn a_normal_session_has_its_folder_title_time_and_branch() {
        let config = ConfigDir::new();
        config.write_session(
            "-home-user-otty",
            ID,
            &[USER, ASSISTANT, AI_TITLE, LAST_PROMPT],
        );

        let sessions = config.sessions();

        let expected = AgentSession::new(
            SessionId::new(ID),
            PathBuf::from("/home/user/otty"),
            "Stop the tab bar flicker",
            at(1_794_823_500),
        )
        .with_branch("main");
        assert_eq!(sessions, [expected]);
    }

    #[test]
    fn a_session_with_no_title_shows_its_first_prompt() {
        let config = ConfigDir::new();
        config.write_session("-home-user-otty", ID, &[USER, ASSISTANT]);

        let sessions = config.sessions();

        assert_eq!(sessions[0].title(), "Fix the tab bar");
    }

    #[test]
    fn a_session_with_no_title_and_no_prompt_shows_its_id() {
        let config = ConfigDir::new();
        config.write_session("-home-user-otty", ID, &[ASSISTANT]);

        let sessions = config.sessions();

        assert_eq!(sessions[0].title(), ID);
    }

    #[test]
    fn broken_lines_in_the_middle_are_skipped() {
        let config = ConfigDir::new();
        let wrong_field_type = r#"{"type":"user","cwd":42,"timestamp":"2026-12-01T00:00:00.000Z"}"#;
        config.write_session(
            "-home-user-otty",
            ID,
            &[
                USER,
                "{\"type\":\"assistant\",\"cwd\":",
                "",
                "not json at all",
                wrong_field_type,
                r#"{"type":"some-future-record","payload":{"x":1}}"#,
                ASSISTANT,
                AI_TITLE,
            ],
        );

        let sessions = config.sessions();

        assert_eq!(sessions[0].folder(), Path::new("/home/user/otty"));
        assert_eq!(sessions[0].title(), "Stop the tab bar flicker");
        assert_eq!(sessions[0].changed_at(), at(1_794_823_500));
        assert_eq!(sessions[0].branch(), Some("main"));
    }

    #[test]
    fn an_empty_file_gives_a_session_with_its_id_and_the_decoded_folder() {
        let config = ConfigDir::new();
        let path = config.session_file("-home-user-otty", ID);
        write_file(&path, "");
        let file_time = std::fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .expect("file time");

        let sessions = config.sessions();

        let expected = AgentSession::new(
            SessionId::new(ID),
            PathBuf::from("/home/user/otty"),
            ID,
            file_time,
        );
        assert_eq!(sessions, [expected]);
    }

    #[test]
    fn helper_sessions_and_other_files_are_not_listed() {
        let config = ConfigDir::new();
        config.write_session("-home-user-otty", ID, &[USER]);
        config.write_session(
            "-home-user-otty",
            &format!("{ID}/subagents/agent-a1b2c3"),
            &[USER],
        );
        write_file(
            &config.path().join("projects/-home-user-otty/notes.txt"),
            "made-up notes",
        );
        write_file(&config.path().join("projects/stray.jsonl"), USER);

        let sessions = config.sessions();

        assert_eq!(ids(&sessions), [ID]);
    }

    #[test]
    fn one_unreadable_file_does_not_hide_the_others() {
        let config = ConfigDir::new();
        config.write_session("-home-user-otty", ID, &[USER]);
        write_file(
            &config.session_file("-home-user-docs", "a-binary-file"),
            "\u{0}\u{1}\u{fffd}garbage with no line end",
        );
        std::fs::create_dir_all(config.session_file("-home-user-x", "a-dir"))
            .expect("create a folder that looks like a session file");

        let sessions = config.sessions();

        assert_eq!(ids(&sessions), [ID, "a-binary-file"]);
        assert_eq!(sessions[1].folder(), Path::new("/home/user/docs"));
    }

    #[test]
    fn a_missing_projects_folder_gives_an_empty_list() {
        let temp = TempDir::new();
        let no_projects = temp.path().join(".claude-work");
        std::fs::create_dir_all(&no_projects).expect("create config folder");
        let missing = temp.path().join(".claude-gone");

        let in_empty_folder =
            SessionFiles.sessions(&claude_profile(&no_projects));
        let in_missing_folder =
            SessionFiles.sessions(&claude_profile(&missing));

        assert_eq!(in_empty_folder, Ok(Vec::new()));
        assert_eq!(in_missing_folder, Ok(Vec::new()));
    }

    #[test]
    fn a_projects_path_that_is_not_a_folder_is_not_available() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".claude-work");
        write_file(&config_dir.join("projects"), "not a folder");

        let result = SessionFiles.sessions(&claude_profile(&config_dir));

        assert!(matches!(result, Err(SourceError::NotAvailable(_))));
    }
}
