//! The agent part `codex`: Codex, OpenAI's coding agent, and the commands
//! that start it.

use otty_plugin_sdk::{
    Agent, Answers, Command, Login, Profile, ProfilePlace, ProfileStatus,
    ProjectStats, ReadError, Session, SessionStats, SpendItem, SpendKind,
    SpendPlace,
};

use crate::glob::escaped;
use crate::profiles::{CONFIG_VARIABLE, profiles};
use crate::sessions::session_file_globs;
use crate::{OpenAi, detect, sessions, spend_file, stats, version};

/// The program, looked up on `PATH`.
const PROGRAM: &str = "codex";

impl Agent for OpenAi {
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        profiles()
    }

    fn detect(profile: &Profile) -> ProfileStatus {
        detect::status(&profile.folder)
    }

    /// The version the newest session file names in its first line.
    fn version(profile: &Profile) -> Option<String> {
        version::version(&profile.folder)
    }

    /// None yet: the logins of `auth.json` come in a later release.
    fn logins(_profile: &Profile) -> Vec<Login> {
        Vec::new()
    }

    fn sessions(profile: &Profile) -> Result<Vec<Session>, ReadError> {
        sessions::sessions(&profile.folder)
    }

    /// Every session file, plain or compressed, in both session folders.
    /// The files of helper agents count too: they are session files like
    /// the others.
    fn spend_places(profile: &Profile) -> Vec<SpendPlace> {
        session_file_globs(&escaped(&profile.folder), "*")
            .into_iter()
            .map(|glob| SpendPlace {
                glob,
                kind: SpendKind::Files,
            })
            .collect()
    }

    fn read_answers(
        _profile: &Profile,
        item: &SpendItem,
    ) -> Result<Answers, ReadError> {
        spend_file::read_answers(&item.path)
    }

    /// `codex` for a new session, `codex resume <id>` to continue one.
    fn launch(profile: &Profile, session: Option<&str>) -> Command {
        let args = match session {
            Some(session) => vec!["resume".to_string(), session.to_string()],
            None => Vec::new(),
        };

        start_command(profile, args)
    }

    /// `codex login`.
    fn login_command(profile: &Profile) -> Command {
        start_command(profile, vec!["login".to_string()])
    }

    fn read_session_stats(
        profile: &Profile,
        session: &str,
    ) -> Result<SessionStats, ReadError> {
        stats::session_stats(&profile.folder, session)
    }

    fn read_project_stats(
        profile: &Profile,
        folder: &str,
    ) -> Result<ProjectStats, ReadError> {
        stats::project_stats(&profile.folder, folder)
    }
}

/// `codex` with `args` for `profile`, with `CODEX_HOME` when its folder
/// is not the default one.
fn start_command(profile: &Profile, args: Vec<String>) -> Command {
    let env = if profile.is_default {
        Vec::new()
    } else {
        vec![(CONFIG_VARIABLE.to_string(), profile.folder.clone())]
    };

    Command {
        program: PROGRAM.to_string(),
        args,
        env,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{BLOG_SESSION, as_part, host, path, profile};

    const ID: &str = "019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b";

    fn default_profile() -> Profile {
        Profile {
            agent: "codex".to_string(),
            folder: "/home/user/.codex".to_string(),
            is_default: true,
        }
    }

    fn added_profile() -> Profile {
        Profile {
            agent: "codex".to_string(),
            folder: "/home/user/.codex-work".to_string(),
            is_default: false,
        }
    }

    fn command(args: &[&str], env: &[(&str, &str)]) -> Command {
        Command {
            program: PROGRAM.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            env: env
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        }
    }

    #[test]
    fn the_default_profile_starts_codex_with_no_variables() {
        let profile = default_profile();

        assert_eq!(OpenAi::launch(&profile, None), command(&[], &[]));
        assert_eq!(
            OpenAi::launch(&profile, Some(ID)),
            command(&["resume", ID], &[])
        );
    }

    #[test]
    fn an_added_profile_starts_codex_with_its_config_folder() {
        let profile = added_profile();
        let env = [(CONFIG_VARIABLE, "/home/user/.codex-work")];

        assert_eq!(OpenAi::launch(&profile, None), command(&[], &env));
        assert_eq!(
            OpenAi::launch(&profile, Some(ID)),
            command(&["resume", ID], &env)
        );
    }

    #[test]
    fn login_runs_codex_login_in_the_config_folder_of_the_profile() {
        assert_eq!(
            OpenAi::login_command(&default_profile()),
            command(&["login"], &[])
        );
        assert_eq!(
            OpenAi::login_command(&added_profile()),
            command(&["login"], &[(CONFIG_VARIABLE, "/home/user/.codex-work")])
        );
    }

    /// The host walks the spend places: every session file, plain or
    /// compressed, archived or not, helper agents included, and nothing
    /// else.
    #[test]
    fn the_spend_places_hold_every_session_file() {
        let host = host();
        let profile = profile(&host, ".codex", true);
        let places = OpenAi::spend_places(&profile);

        let mut names: Vec<String> = as_part(&host, || {
            places
                .iter()
                .flat_map(|place| {
                    otty_plugin_sdk::files::list(&place.glob)
                        .expect("the listing")
                })
                .map(|file| file.path.replace(&path(&host, ".codex/"), ""))
                .collect()
        });
        names.sort();
        assert_eq!(
            names,
            [
                format!(
                    "archived_sessions/rollout-2026-11-15T09-00-00-{BLOG_SESSION}.jsonl"
                ),
                "sessions/2026/11/13/rollout-2026-11-13T08-00-00-019a7ffe-0000-7000-8000-000000000000.jsonl.zst".to_string(),
                "sessions/2026/11/14/rollout-2026-11-14T08-00-00-019a7fff-0000-7000-8000-000000000000.jsonl".to_string(),
                format!(
                    "sessions/2026/11/15/rollout-2026-11-15T09-00-00-{BLOG_SESSION}.jsonl.zst"
                ),
                "sessions/2026/11/16/rollout-2026-11-16T10-00-00-019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b.jsonl".to_string(),
                "sessions/2026/11/16/rollout-2026-11-16T10-05-00-019a9001-2222-7333-8444-555566667777.jsonl".to_string(),
            ]
        );
        assert!(places.iter().all(|place| place.kind == SpendKind::Files));
    }

    /// The tokens of a session on a provider of the user's own config are
    /// not OpenAI's.
    #[test]
    fn answers_are_read_with_the_provider_of_the_session() {
        let host = host();
        let profile = profile(&host, ".codex-work", false);
        let item = SpendItem {
            path: path(
                &host,
                ".codex-work/sessions/2026/11/17/rollout-2026-11-17T08-00-00-019aa000-3333-7444-8555-666677778888.jsonl",
            ),
            kind: SpendKind::Files,
        };

        let answers = as_part(&host, || OpenAi::read_answers(&profile, &item))
            .expect("the answers");

        assert_eq!(answers.answers.len(), 1);
        assert_eq!(answers.answers[0].provider, None);
        assert_eq!(answers.answers[0].model.as_deref(), Some("qwen3-coder"));
        assert_eq!(
            answers.answers[0].project.as_deref(),
            Some("/home/dev/work")
        );
    }
}
