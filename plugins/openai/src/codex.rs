//! The agent part `codex`: Codex, OpenAI's coding agent, and the commands
//! that start it.

use otty_plugin_sdk::Command;
use otty_plugin_sdk::agents::{
    Agent, Answers, Place, PlaceItem, PlaceKind, Profile, ProfilePlace,
    ProfileStatus, ReadError, Session,
};

use crate::glob::escaped;
use crate::profiles::{CONFIG_VARIABLE, profiles};
use crate::sessions::session_file_globs;
use crate::{OpenAi, detect, sessions, spend_file, version};

/// The program, looked up on `PATH`.
pub(crate) const PROGRAM: &str = "codex";

impl Agent for OpenAi {
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        profiles()
    }

    /// A profile nobody is logged in with needs `codex login`.
    fn detect(profile: &Profile) -> ProfileStatus {
        let login = start_command(profile, vec!["login".to_string()]);

        detect::status(&profile.folder, login)
    }

    /// The version the newest session file names in its first line.
    fn version(profile: &Profile) -> Option<String> {
        version::version(&profile.folder)
    }

    /// Every session file, plain or compressed, in both session folders,
    /// and the index of their names.
    fn session_places(profile: &Profile) -> Vec<Place> {
        sessions::session_places(&profile.folder)
    }

    fn read_sessions(
        _profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Vec<Session>, ReadError> {
        Ok(sessions::read_sessions(item))
    }

    /// Every session file, plain or compressed, in both session folders.
    /// The files of helper agents count too: they are session files like
    /// the others.
    fn spend_places(profile: &Profile) -> Vec<Place> {
        session_file_globs(&escaped(&profile.folder), "*")
            .into_iter()
            .map(|glob| Place {
                glob,
                kind: PlaceKind::Files,
            })
            .collect()
    }

    fn read_answers(
        _profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Answers, ReadError> {
        spend_file::read_profile_answers(&item.file.path)
    }

    /// `codex` for a new session, `codex resume <id>` to continue one.
    fn launch(profile: &Profile, session: Option<&str>) -> Command {
        let args = match session {
            Some(session) => vec!["resume".to_string(), session.to_string()],
            None => Vec::new(),
        };

        start_command(profile, args)
    }
}

/// `codex` with `args` for `profile`.
fn start_command(profile: &Profile, args: Vec<String>) -> Command {
    Command {
        program: PROGRAM.to_string(),
        args,
        env: profile_env(profile),
    }
}

/// The variables that point `codex` at the config folder of `profile`:
/// `CODEX_HOME` when its folder is not the default one.
fn profile_env(profile: &Profile) -> Vec<(String, String)> {
    if profile.is_default {
        return Vec::new();
    }

    vec![(CONFIG_VARIABLE.to_string(), profile.folder.clone())]
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::NeedAction;

    use super::*;
    use crate::testing::{
        AGENT, BLOG_SESSION, TEN, as_part, as_provider_in, codex_folder, host,
        path, profile,
    };

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

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 2: a profile
    /// with a login is found, and the agent part says nothing of whose it
    /// is: the provider part finds the account.
    #[test]
    fn a_profile_with_a_login_is_found() {
        let host = host().with_now_ms(TEN);
        let profile = profile(&host, ".codex-plus", false);

        let status = as_part(&host, || OpenAi::detect(&profile));

        assert_eq!(status, ProfileStatus::Found);
    }

    /// "No login" for a folder whose `auth.json` holds no login, in
    /// English.
    #[test]
    fn a_folder_with_no_login_needs_a_login() {
        let host = host();
        let profile = profile(&host, ".codex-emptyauth", false);

        let status = as_part(&host, || OpenAi::detect(&profile));

        let ProfileStatus::NotLoggedIn(need) = status else {
            panic!("not logged in: {status:?}");
        };
        assert_eq!(need.title, "Needs login");
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

    /// The need of a profile nobody is logged in with runs `codex login`
    /// in the profile's config folder, in the language of OTTY.
    #[test]
    fn a_profile_nobody_is_logged_in_with_needs_codex_login() {
        let host = host().with_language("ru");
        let added = profile(&host, ".codex-loggedout", false);
        let default = profile(&host, ".codex-loggedout", true);

        let added_status = as_part(&host, || OpenAi::detect(&added));
        let default_status = as_part(&host, || OpenAi::detect(&default));

        let ProfileStatus::NotLoggedIn(need) = added_status else {
            panic!("not logged in: {added_status:?}");
        };
        assert_eq!(need.id, "not-logged-in");
        assert_eq!(need.title, "Нужен вход");
        assert!(need.text.contains("Codex"), "{}", need.text);
        assert_eq!(
            need.action,
            NeedAction::Run(command(
                &["login"],
                &[(CONFIG_VARIABLE, added.folder.as_str())]
            ))
        );
        let ProfileStatus::NotLoggedIn(need) = default_status else {
            panic!("not logged in: {default_status:?}");
        };
        assert_eq!(need.action, NeedAction::Run(command(&["login"], &[])));
    }

    /// Issue 12 of spec 2026-11-23-accounts-apart-from-profiles: an added
    /// config folder with no file in it yet needs `codex login` there,
    /// with `CODEX_HOME` set to it.
    #[test]
    fn an_added_empty_folder_needs_codex_login_in_it() {
        let host = host();
        let added = profile(&host, ".codex-new", false);

        let status = as_part(&host, || OpenAi::detect(&added));

        let ProfileStatus::NotLoggedIn(need) = status else {
            panic!("not logged in: {status:?}");
        };
        assert_eq!(need.id, "not-logged-in");
        assert_eq!(need.title, "Needs login");
        assert_eq!(
            need.action,
            NeedAction::Run(command(
                &["login"],
                &[(CONFIG_VARIABLE, added.folder.as_str())]
            ))
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 9: the files
    /// tell that the ChatGPT login expired, so the profile needs the login
    /// again, with the very need the provider part gives on the account
    /// found in that folder: one title, text and command, so the two read
    /// as one need, and acting on either fixes both.
    #[test]
    fn an_expired_login_needs_the_same_login_as_its_account() {
        let host = host().with_now_ms(TEN).with_language("ru");
        let profile = profile(&host, ".codex-expired", true);

        let status = as_part(&host, || OpenAi::detect(&profile));
        let account = as_provider_in(&host, ".codex-expired", || {
            crate::accounts::accounts(&[codex_folder(&host, ".codex-expired")])
        });

        let ProfileStatus::NotLoggedIn(need) = status else {
            panic!("not logged in: {status:?}");
        };
        assert_eq!(need.id, "login-expired");
        assert_eq!(need.title, "Вход истёк");
        assert_eq!(account.accounts[0].needs, [need]);
    }

    /// A login whose token is still good is found.
    #[test]
    fn a_login_before_its_expiry_is_found() {
        let host = host().with_now_ms(TEN);
        let profile = profile(&host, ".codex-plus", false);

        let status = as_part(&host, || OpenAi::detect(&profile));

        assert_eq!(status, ProfileStatus::Found);
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
        assert!(places.iter().all(|place| place.kind == PlaceKind::Files));
    }

    /// The tokens of a session on a provider of the user's own config are
    /// not OpenAI's.
    #[test]
    fn answers_are_read_with_the_provider_of_the_session() {
        let host = host();
        let profile = profile(&host, ".codex-work", false);
        let items = host.place_items(AGENT, &OpenAi::spend_places(&profile));
        let item = items
            .iter()
            .find(|item| item.file.path.ends_with("666677778888.jsonl"))
            .expect("the session file");

        let answers = as_part(&host, || OpenAi::read_answers(&profile, item))
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
