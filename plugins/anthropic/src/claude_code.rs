//! The agent part `claude-code`: Claude Code, Anthropic's coding agent,
//! and the commands that start it.

use otty_plugin_sdk::{
    Agent, Answers, Command, Limits, Login, LoginRef, Profile, ProfilePlace,
    ProfileStatus, ProjectStats, ReadError, Session, SessionStats, SpendItem,
    SpendKind, SpendPlace,
};

use crate::glob::escaped;
use crate::profiles::{CONFIG_VARIABLE, profiles};
use crate::project_folder::projects_folder;
use crate::provider::profile_provider;
use crate::{
    Anthropic, detect, logins, sessions, spend_file, stats, usage_probe,
    version,
};

/// The program, looked up on `PATH`.
const PROGRAM: &str = "claude";

impl Agent for Anthropic {
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        profiles()
    }

    fn detect(profile: &Profile) -> ProfileStatus {
        detect::status(&profile.folder)
    }

    /// The version the records of the newest session file name.
    fn version(profile: &Profile) -> Option<String> {
        version::version(&profile.folder)
    }

    fn logins(profile: &Profile) -> Vec<Login> {
        logins::logins(&profile.folder)
    }

    fn sessions(profile: &Profile) -> Result<Vec<Session>, ReadError> {
        sessions::sessions(&profile.folder)
    }

    /// Every `*.jsonl` file in the project folders, at any depth, so the
    /// files of helper agents in `<session id>/subagents/` count too.
    fn spend_places(profile: &Profile) -> Vec<SpendPlace> {
        let projects = escaped(&projects_folder(&profile.folder));

        vec![SpendPlace {
            glob: format!("{projects}/*/**/*.jsonl"),
            kind: SpendKind::Files,
        }]
    }

    /// The answers of one session file, under the server the settings
    /// send Claude Code to: Claude Code does not write it.
    fn read_answers(
        profile: &Profile,
        item: &SpendItem,
    ) -> Result<Answers, ReadError> {
        let provider = profile_provider(&profile.folder);

        spend_file::read_answers(&profile.folder, &item.path, provider.as_ref())
    }

    /// `claude` for a new session, `claude --resume <id>` to continue one.
    fn launch(profile: &Profile, session: Option<&str>) -> Command {
        let args = match session {
            Some(session) => vec!["--resume".to_string(), session.to_string()],
            None => Vec::new(),
        };

        start_command(profile, args)
    }

    /// `claude`, which asks to log in on its first start in a config
    /// folder.
    fn login_command(profile: &Profile) -> Command {
        start_command(profile, Vec::new())
    }

    /// The `/usage` panel of `claude`, for any login of the profile: it
    /// shows the limits of the login Claude Code runs with.
    fn read_limits(
        profile: &Profile,
        _login: &LoginRef,
    ) -> Result<Limits, ReadError> {
        usage_probe::read_limits(profile)
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

/// `claude` with `args` for `profile`, with `CLAUDE_CONFIG_DIR` when its
/// folder is not the default one. A profile in the default folder sets
/// nothing: with the variable set, Claude Code would look for its
/// `.claude.json` inside the folder instead of next to it, and lose the
/// login of the default profile.
pub(crate) fn start_command(profile: &Profile, args: Vec<String>) -> Command {
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
    use crate::testing::{AGENT, as_part, host, path, profile};

    const SESSION: &str = "0b6c4e8a-1f2d-4c3b-9a7e-5d1f2e3c4b5a";

    fn default_profile() -> Profile {
        Profile {
            agent: AGENT.to_string(),
            folder: "/home/dev/.claude".to_string(),
            is_default: true,
        }
    }

    fn added_profile() -> Profile {
        Profile {
            agent: AGENT.to_string(),
            folder: "/home/dev/.claude-work".to_string(),
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
    fn the_default_profile_starts_claude_with_no_variables() {
        let profile = default_profile();

        assert_eq!(Anthropic::launch(&profile, None), command(&[], &[]));
        assert_eq!(
            Anthropic::launch(&profile, Some(SESSION)),
            command(&["--resume", SESSION], &[])
        );
    }

    #[test]
    fn an_added_profile_starts_claude_with_its_config_folder() {
        let profile = added_profile();
        let env = [(CONFIG_VARIABLE, "/home/dev/.claude-work")];

        assert_eq!(Anthropic::launch(&profile, None), command(&[], &env));
        assert_eq!(
            Anthropic::launch(&profile, Some(SESSION)),
            command(&["--resume", SESSION], &env)
        );
    }

    #[test]
    fn login_starts_claude_in_the_config_folder_of_the_profile() {
        assert_eq!(
            Anthropic::login_command(&default_profile()),
            command(&[], &[])
        );
        assert_eq!(
            Anthropic::login_command(&added_profile()),
            command(&[], &[(CONFIG_VARIABLE, "/home/dev/.claude-work")])
        );
    }

    /// The host walks the spend place: every session file of a project
    /// folder, helper agents included, and nothing outside them.
    #[test]
    fn the_spend_place_holds_every_session_file_of_the_project_folders() {
        let host = host();
        let profile = profile(&host, ".claude", true);
        let places = Anthropic::spend_places(&profile);

        let found =
            as_part(&host, || otty_plugin_sdk::files::list(&places[0].glob))
                .expect("the listing");

        let mut names: Vec<String> = found
            .iter()
            .map(|file| {
                file.path.replace(&path(&host, ".claude/projects/"), "")
            })
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "-home-dev-blog/2a1d3c8f-9e5b-4f70-8c1d-2b3e4f5a6b7c.jsonl",
                "-home-dev-shop/1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b.jsonl",
                "-home-dev-shop/1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b/subagents/agent-a1b2c3.jsonl",
                "-home-dev-shop/3b2e4d9a-af6c-4081-9d2e-3c4f5a6b7c8d.jsonl",
            ]
        );
        assert_eq!(places[0].kind, SpendKind::Files);
    }

    /// The tokens of a profile whose settings send Claude Code to DeepSeek
    /// are DeepSeek's.
    #[test]
    fn answers_are_counted_under_the_server_of_the_settings() {
        let host = host();
        let profile = profile(&host, ".claude-deepseek", false);
        let item = SpendItem {
            path: path(
                &host,
                ".claude-deepseek/projects/-home-dev-shop/5d4a6fbc-c18e-42a3-bf4a-5e6b7c8d9eaf.jsonl",
            ),
            kind: SpendKind::Files,
        };

        let answers =
            as_part(&host, || Anthropic::read_answers(&profile, &item))
                .expect("the answers");

        assert_eq!(answers.answers.len(), 1);
        assert_eq!(
            answers.answers[0].provider,
            Some(otty_plugin_sdk::ProviderRef::Host(
                "api.deepseek.com".to_string()
            ))
        );
        assert_eq!(answers.answers[0].model.as_deref(), Some("deepseek-chat"));
    }
}
