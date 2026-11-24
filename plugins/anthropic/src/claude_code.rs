//! The agent part `claude-code`: Claude Code, Anthropic's coding agent,
//! and the commands that start it.

use otty_plugin_sdk::Command;
use otty_plugin_sdk::agents::{
    Agent, Answers, Place, PlaceItem, PlaceKind, Profile, ProfilePlace,
    ProfileStatus, ReadError, Session,
};

use crate::glob::escaped;
use crate::profiles::{CONFIG_VARIABLE, default_folder, profiles};
use crate::project_folder::projects_folder;
use crate::provider::profile_provider;
use crate::{Anthropic, detect, sessions, spend_file, version};

/// The program, looked up on `PATH`.
pub(crate) const PROGRAM: &str = "claude";

impl Agent for Anthropic {
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        profiles()
    }

    /// A profile nobody is logged in with, such as a new one with an
    /// empty folder, needs `claude auth login --claudeai` in its folder.
    fn detect(profile: &Profile) -> ProfileStatus {
        let login = start_command(profile, login_args());

        detect::status(&profile.folder, login)
    }

    /// The version the records of the newest session file name.
    fn version(profile: &Profile) -> Option<String> {
        version::version(&profile.folder)
    }

    /// Every `*.jsonl` file in the project folders: the files of helper
    /// agents, deeper, name the session that ran them.
    fn session_places(profile: &Profile) -> Vec<Place> {
        sessions::session_places(&profile.folder)
    }

    fn read_sessions(
        profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Vec<Session>, ReadError> {
        Ok(sessions::read_sessions(&profile.folder, item))
    }

    /// Every `*.jsonl` file in the project folders, at any depth, so the
    /// files of helper agents in `<session id>/subagents/` count too.
    fn spend_places(profile: &Profile) -> Vec<Place> {
        vec![Place {
            glob: session_files_glob(&profile.folder),
            kind: PlaceKind::Files,
        }]
    }

    /// The answers of one session file, under the server the settings
    /// send Claude Code to: Claude Code does not write it.
    fn read_answers(
        profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Answers, ReadError> {
        let provider = profile_provider(&profile.folder);

        spend_file::read_answers(
            &profile.folder,
            &item.file.path,
            provider.as_ref(),
        )
    }

    /// `claude` for a new session, `claude --resume <id>` to continue one.
    fn launch(profile: &Profile, session: Option<&str>) -> Command {
        let args = match session {
            Some(session) => vec!["--resume".to_string(), session.to_string()],
            None => Vec::new(),
        };

        start_command(profile, args)
    }
}

/// Every session file of the config folder `folder`: each `*.jsonl` file
/// in its project folders, at any depth, so the files of helper agents in
/// `<session id>/subagents/` count too.
pub(crate) fn session_files_glob(folder: &str) -> String {
    let projects = escaped(&projects_folder(folder));

    format!("{projects}/*/**/*.jsonl")
}

/// `claude` with `args` for `profile`, as [`command_in`] gives it.
pub(crate) fn start_command(profile: &Profile, args: Vec<String>) -> Command {
    command_in(&profile.folder, profile.is_default, args)
}

/// `claude` with no arguments in the config folder `folder`, as the agent
/// part starts it in a profile there. The provider part runs it so, for
/// the `/usage` panel or to renew a login, and knows the default folder
/// by where it is.
pub(crate) fn claude_in_folder(folder: &str) -> Command {
    let is_default = folder == default_folder();

    command_in(folder, is_default, Vec::new())
}

/// `claude auth login --claudeai` with `CLAUDE_CONFIG_DIR` set to OTTY's
/// own login folder `folder`, which is never the default one: Log in with
/// Claude Code (spec 2026-11-23-accounts-apart-from-profiles, Decision 6,
/// the command the owner chose on 2026-11-23).
pub(crate) fn claude_login_in(folder: &str) -> Command {
    command_in(folder, false, login_args())
}

/// The arguments of Claude Code's login with a Claude account: `auth login
/// --claudeai`.
fn login_args() -> Vec<String> {
    ["auth", "login", "--claudeai"].map(String::from).to_vec()
}

/// `claude` with `args` in the config folder `folder`, with
/// `CLAUDE_CONFIG_DIR` when the folder is not the default one. The
/// default folder sets nothing: with the variable set, Claude Code would
/// look for its `.claude.json` inside the folder instead of next to it,
/// and lose the login of the default profile.
fn command_in(folder: &str, is_default: bool, args: Vec<String>) -> Command {
    let env = if is_default {
        Vec::new()
    } else {
        vec![(CONFIG_VARIABLE.to_string(), folder.to_string())]
    };

    Command {
        program: PROGRAM.to_string(),
        args,
        env,
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::NeedAction;
    use otty_plugin_sdk::agents::Provider;

    use super::*;
    use crate::testing::{
        AGENT, PROVIDER, as_part, as_provider_in, host, login_folder, path,
        profile,
    };

    /// A time after the made-up tokens of the fixtures expired, and after
    /// the one of `.claude-expired`.
    const AFTER_EXPIRY_MS: i64 = 1_800_000_000_000;

    const SESSION: &str = "0b6c4e8a-1f2d-4c3b-9a7e-5d1f2e3c4b5a";

    /// The arguments of Claude Code's login.
    const LOGIN_ARGS: [&str; 3] = ["auth", "login", "--claudeai"];

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

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6: Log in
    /// with Claude Code runs `claude auth login --claudeai` with
    /// `CLAUDE_CONFIG_DIR` set to OTTY's own folder (owner, 2026-11-23).
    #[test]
    fn log_in_runs_claude_auth_login_in_the_own_folder() {
        let folder = "/home/dev/.local/share/otty/accounts/6f0c1e2a";

        let command_of = Anthropic::log_in(PROVIDER, folder);

        assert_eq!(
            command_of,
            Some(command(
                &["auth", "login", "--claudeai"],
                &[(CONFIG_VARIABLE, folder)]
            ))
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 2: a profile
    /// with a login is found, and the agent part says nothing of whose it
    /// is: the provider part finds the account.
    #[test]
    fn a_profile_with_a_login_is_found() {
        let host = host();
        let work = profile(&host, ".claude-work", false);

        let status = as_part(&host, || Anthropic::detect(&work));

        assert_eq!(status, ProfileStatus::Found);
    }

    /// "No login" for a folder with no login of Claude Code, in English.
    #[test]
    fn a_folder_with_no_login_needs_a_login() {
        let host = host();
        let nothing = profile(&host, ".claude-nothing", false);

        let status = as_part(&host, || Anthropic::detect(&nothing));

        let ProfileStatus::NotLoggedIn(need) = status else {
            panic!("not logged in: {status:?}");
        };
        assert_eq!(need.title, "Needs login");
    }

    /// The need of a profile nobody is logged in with runs `claude auth
    /// login --claudeai` in the profile's config folder, in the language
    /// of OTTY (issue 12 of spec 2026-11-23-accounts-apart-from-profiles).
    #[test]
    fn a_profile_nobody_is_logged_in_with_needs_claude_to_log_in() {
        let host = host().with_language("ru");
        let added = profile(&host, ".claude-nothing", false);
        let default = profile(&host, ".claude-nothing", true);

        let added_status = as_part(&host, || Anthropic::detect(&added));
        let default_status = as_part(&host, || Anthropic::detect(&default));

        let ProfileStatus::NotLoggedIn(need) = added_status else {
            panic!("not logged in: {added_status:?}");
        };
        assert_eq!(need.id, "not-logged-in");
        assert_eq!(need.title, "Нужен вход");
        assert!(need.text.contains("Claude Code"), "{}", need.text);
        assert_eq!(
            need.action,
            NeedAction::Run(command(
                &LOGIN_ARGS,
                &[(CONFIG_VARIABLE, added.folder.as_str())]
            ))
        );
        let ProfileStatus::NotLoggedIn(need) = default_status else {
            panic!("not logged in: {default_status:?}");
        };
        assert_eq!(need.action, NeedAction::Run(command(&LOGIN_ARGS, &[])));
    }

    /// Issue 12 of spec 2026-11-23-accounts-apart-from-profiles: an added
    /// config folder with no file in it yet needs `claude auth login
    /// --claudeai` there, with `CLAUDE_CONFIG_DIR` set to it.
    #[test]
    fn an_added_empty_folder_needs_claude_auth_login_in_it() {
        let host = host();
        let added = profile(&host, ".claude-new", false);

        let status = as_part(&host, || Anthropic::detect(&added));

        let ProfileStatus::NotLoggedIn(need) = status else {
            panic!("not logged in: {status:?}");
        };
        assert_eq!(need.id, "not-logged-in");
        assert_eq!(need.title, "Needs login");
        assert_eq!(
            need.action,
            NeedAction::Run(command(
                &LOGIN_ARGS,
                &[(CONFIG_VARIABLE, added.folder.as_str())]
            ))
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 9: the files
    /// tell that the Claude login expired for good, a token past its time
    /// with no refresh token, so the profile needs the login again, with
    /// the very need the provider part gives on the account found in that
    /// folder: one title, text and command, so the two read as one need.
    #[test]
    fn an_expired_login_needs_the_same_login_as_its_account() {
        let host = host().with_now_ms(AFTER_EXPIRY_MS).with_language("ru");
        let expired = profile(&host, ".claude-expired", false);
        let folders = [login_folder(&host, ".claude-expired")];

        let status = as_part(&host, || Anthropic::detect(&expired));
        let found = as_provider_in(&host, &folders, || {
            <Anthropic as Provider>::accounts(PROVIDER, &folders)
        });

        let ProfileStatus::NotLoggedIn(need) = status else {
            panic!("not logged in: {status:?}");
        };
        assert_eq!(need.id, "login-expired");
        assert_eq!(need.title, "Вход истёк");
        assert_eq!(found.accounts[0].needs, [need]);
    }

    /// Claude Code renews a token past its time with its refresh token, so
    /// such a login is found.
    #[test]
    fn a_login_with_a_refresh_token_is_found_past_its_time() {
        let host = host().with_now_ms(AFTER_EXPIRY_MS);
        let work = profile(&host, ".claude-work", false);

        let status = as_part(&host, || Anthropic::detect(&work));

        assert_eq!(status, ProfileStatus::Found);
    }

    /// The provider part knows the default folder by where it is: there
    /// `claude` sets no variable, anywhere else it gets the folder.
    #[test]
    fn claude_in_a_folder_sets_the_variable_only_outside_the_default_one() {
        let host = host();
        let default = path(&host, ".claude");
        let work = path(&host, ".claude-work");

        let (in_default, in_work) = host.as_provider(PROVIDER, || {
            (claude_in_folder(&default), claude_in_folder(&work))
        });

        assert_eq!(in_default, command(&[], &[]));
        assert_eq!(in_work, command(&[], &[(CONFIG_VARIABLE, work.as_str())]));
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
        assert_eq!(places[0].kind, PlaceKind::Files);
    }

    /// The tokens of a profile whose settings send Claude Code to DeepSeek
    /// are DeepSeek's.
    #[test]
    fn answers_are_counted_under_the_server_of_the_settings() {
        let host = host();
        let profile = profile(&host, ".claude-deepseek", false);
        let items = host.place_items(AGENT, &Anthropic::spend_places(&profile));
        let item = items
            .iter()
            .find(|item| item.file.path.ends_with("5e6b7c8d9eaf.jsonl"))
            .expect("the session file");

        let answers =
            as_part(&host, || Anthropic::read_answers(&profile, item))
                .expect("the answers");

        assert_eq!(answers.answers.len(), 1);
        assert_eq!(
            answers.answers[0].provider,
            Some(otty_plugin_sdk::agents::ProviderRef::Host(
                "api.deepseek.com".to_string()
            ))
        );
        assert_eq!(answers.answers[0].model.as_deref(), Some("deepseek-chat"));
    }
}
