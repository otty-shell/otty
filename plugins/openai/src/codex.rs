//! The agent part `codex`: Codex, OpenAI's coding agent, and the commands
//! that start it.

use otty_plugin_sdk::{
    Agent, Answers, Command, KeyPlace, Limits, Login, LoginRef, Need, Profile,
    ProfilePlace, ProfileStatus, ProjectStats, ReadError, Session,
    SessionStats, SpendItem, SpendKind, SpendPlace, clock, keys,
};

use crate::auth_file::ACCESS_TOKEN_FIELD;
use crate::glob::escaped;
use crate::profiles::{CONFIG_VARIABLE, profiles};
use crate::sessions::session_file_globs;
use crate::token_expiry::expires_at_ms;
use crate::{
    OpenAi, accounts, app_server, detect, needs, session_limits, sessions,
    spend_file, stats, version,
};

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

    /// The ChatGPT login or the API key of `auth.json`.
    fn logins(profile: &Profile) -> Vec<Login> {
        accounts::logins(&profile.folder)
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
        profile: &Profile,
        item: &SpendItem,
    ) -> Result<Answers, ReadError> {
        spend_file::read_profile_answers(&profile.folder, &item.path)
    }

    /// `codex` for a new session, `codex resume <id>` to continue one.
    fn launch(profile: &Profile, session: Option<&str>) -> Command {
        let args = match session {
            Some(session) => vec!["resume".to_string(), session.to_string()],
            None => Vec::new(),
        };

        start_command(profile, args)
    }

    /// The limits of a ChatGPT login; an API key has none to read.
    /// `codex app-server` first, which tells the limits now, then the last
    /// record of them in the session files. When neither answers, the
    /// error of `codex app-server` says why.
    ///
    /// `codex app-server` renews an expired login token by itself, which
    /// a plugin never makes another agent do: with an expired token it is
    /// not started, and the read waits for the user to run Codex, which
    /// renews it.
    fn read_limits(
        profile: &Profile,
        login: &LoginRef,
    ) -> Result<Limits, ReadError> {
        if !is_chatgpt_login(login) {
            return Err(ReadError::NotSupported);
        }

        let now = if has_expired(login) {
            let codex = start_command(profile, Vec::new());
            Err(ReadError::NeedsUser(needs::login_expired(codex)))
        } else {
            app_server::read_limits(&profile_env(profile))
        };

        now.or_else(|error| {
            session_limits::read_limits(&profile.folder).map_err(|_| error)
        })
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

    /// A provider part refused the ChatGPT login of the profile, or saw it
    /// expired, wherever it read it: Codex itself renews it in the
    /// profile's config folder, or asks to log in again. The same need as
    /// this part's own read of the limits gives, so it shows once. An API
    /// key is no login Codex renews.
    fn login_refused(profile: &Profile, login: &LoginRef) -> Option<Need> {
        if !is_chatgpt_login(login) {
            return None;
        }

        let codex = start_command(profile, Vec::new());
        Some(needs::login_expired(codex))
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

/// Whether `login` is the access token of a ChatGPT login.
fn is_chatgpt_login(login: &LoginRef) -> bool {
    matches!(login, KeyPlace::JsonField(field) if field.pointer == ACCESS_TOKEN_FIELD)
}

/// Whether the token at `login` expired, as its claims say. A token that
/// says nothing, or can not be opened, is not taken as expired.
fn has_expired(login: &LoginRef) -> bool {
    let claims = keys::open(login)
        .ok()
        .and_then(|token| token.jwt_claims().ok().flatten());

    claims
        .and_then(|claims| expires_at_ms(&claims))
        .is_some_and(|expires_at_ms| expires_at_ms <= clock::now_ms())
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{JsonField, NeedAction};

    use super::*;
    use crate::app_server::APP_SERVER_ARGS;
    use crate::testing::{BLOG_SESSION, TEN, as_part, host, path, profile};

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

    /// The access token of the login of the config folder `name`.
    fn token_of(host: &TestHost, name: &str) -> LoginRef {
        KeyPlace::JsonField(JsonField {
            file: path(host, &format!("{name}/auth.json")),
            pointer: ACCESS_TOKEN_FIELD.to_string(),
        })
    }

    #[test]
    fn the_app_server_of_a_profile_runs_in_its_config_folder() {
        let host = host();
        host.answer_talk(
            "codex",
            &APP_SERVER_ARGS,
            &[
                r#"{"id":1,"result":{}}"#,
                r#"{"id":2,"result":{"rateLimits":{"primary":{"usedPercent":42,"windowDurationMins":300,"resetsAt":1794826800},"planType":"pro"}}}"#,
            ],
        );
        let profile = profile(&host, ".codex-plus", false);
        let login = token_of(&host, ".codex-plus");

        let limits = as_part(&host, || OpenAi::read_limits(&profile, &login))
            .expect("the limits");

        assert_eq!(limits.plan.as_deref(), Some("Pro 200"));
        assert_eq!(
            host.started()[0].env(),
            [(CONFIG_VARIABLE.to_string(), profile.folder.clone())]
        );
    }

    /// `codex app-server` does not start here: the last record of the
    /// session files answers in its place.
    #[test]
    fn without_the_app_server_the_session_files_give_their_last_record() {
        let host = host();
        let profile = profile(&host, ".codex-limits", false);
        let login = token_of(&host, ".codex-limits");

        let limits = as_part(&host, || OpenAi::read_limits(&profile, &login))
            .expect("the limits");

        assert_eq!(limits.plan.as_deref(), Some("Plus"));
        assert!(limits.recorded_at_ms.is_some());
    }

    /// `codex app-server` would renew the expired token, writing Codex's
    /// login: it is not started, and with no session files to fall back
    /// on the read waits for the user to run Codex in the profile's config
    /// folder, which renews the login.
    #[test]
    fn an_expired_login_starts_no_app_server_and_asks_to_run_codex() {
        let host = host();
        host.answer_talk("codex", &APP_SERVER_ARGS, &[]);
        let profile = profile(&host, ".codex-expired", false);
        let login = token_of(&host, ".codex-expired");

        let limits = as_part(&host, || OpenAi::read_limits(&profile, &login));

        let Err(ReadError::NeedsUser(need)) = limits else {
            panic!("a need: {limits:?}");
        };
        assert_eq!(need.id, "login-expired");
        assert_eq!(need.title, "The login expired");
        assert!(need.text.contains("Open Codex"), "{}", need.text);
        assert_eq!(
            need.action,
            NeedAction::Run(command(
                &[],
                &[(CONFIG_VARIABLE, profile.folder.as_str())]
            ))
        );
        assert!(host.started().is_empty());
    }

    /// Issue 20: a provider part refused the ChatGPT login of a profile;
    /// the need runs Codex in that profile's config folder, as the part's
    /// own read gives it.
    #[test]
    fn a_refused_login_runs_codex_in_the_profiles_folder() {
        let host = host();
        let profile = profile(&host, ".codex-plus", false);
        let login = token_of(&host, ".codex-plus");

        let need = as_part(&host, || OpenAi::login_refused(&profile, &login))
            .expect("a need");

        assert_eq!(need.id, "login-expired");
        assert_eq!(
            need.action,
            NeedAction::Run(command(
                &[],
                &[(CONFIG_VARIABLE, profile.folder.as_str())]
            ))
        );
    }

    /// An API key is no login Codex renews: the part says nothing.
    #[test]
    fn a_refused_api_key_gives_no_need_of_codex() {
        let host = host();
        let profile = profile(&host, ".codex-work", false);
        let key = as_part(&host, || OpenAi::logins(&profile)).remove(0);

        let need =
            as_part(&host, || OpenAi::login_refused(&profile, &key.place));

        assert_eq!(need, None);
    }

    #[test]
    fn a_token_that_does_not_tell_its_expiry_is_used() {
        let host = host().with_now_ms(TEN);
        let login = token_of(&host, ".codex");
        let fresh = token_of(&host, ".codex-plus");

        let opaque = as_part(&host, || has_expired(&login));
        let far = as_part(&host, || has_expired(&fresh));

        assert!(!opaque);
        assert!(!far);
    }

    #[test]
    fn an_api_key_has_no_limits_to_read() {
        let host = host();
        let profile = profile(&host, ".codex-work", false);
        let key = as_part(&host, || OpenAi::logins(&profile)).remove(0);

        let limits =
            as_part(&host, || OpenAi::read_limits(&profile, &key.place));

        assert_eq!(limits, Err(ReadError::NotSupported));
        assert!(host.started().is_empty());
    }

    #[test]
    fn the_logins_of_a_profile_are_its_auth_file() {
        let host = host();
        let profile = profile(&host, ".codex-plus", false);

        let logins = as_part(&host, || OpenAi::logins(&profile));

        assert_eq!(logins.len(), 1);
        assert_eq!(logins[0].place, token_of(&host, ".codex-plus"));
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
