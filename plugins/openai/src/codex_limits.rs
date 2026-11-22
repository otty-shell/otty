//! The limits Codex itself knows of a ChatGPT login, read by the provider
//! part `openai` in a Codex profile the login was found in (issue 29):
//! `codex app-server` first, which tells them now, then the last record
//! of them in the session files. When neither answers, the error of
//! `codex app-server` says why.
//!
//! `codex app-server` renews an expired login token by itself, which a
//! plugin never makes another agent do: with an expired token it is not
//! started, and the read waits for the user to run Codex, which renews it.

use otty_plugin_sdk::agents::{FoundProfile, ReadError};
use otty_plugin_sdk::{Key, clock};

use crate::limits::Limits;
use crate::token_expiry::expires_at_ms;
use crate::{app_server, needs, session_limits};

/// The limits of the login of `token` that Codex knows in `profile`, run
/// with the command the agent part `codex` starts Codex with there.
pub(crate) fn read_codex_limits(
    profile: &FoundProfile,
    token: &Key,
) -> Result<Limits, ReadError> {
    let now = if has_expired(token) {
        let codex = profile.launch.clone();
        Err(ReadError::NeedsUser(needs::login_expired(codex)))
    } else {
        app_server::read_limits(&profile.launch.env)
    };

    now.or_else(|error| {
        session_limits::read_limits(&profile.folder).map_err(|_| error)
    })
}

/// Whether `token` expired, as its claims say. A token that says nothing,
/// or that is not at its place, is not taken as expired.
fn has_expired(token: &Key) -> bool {
    token
        .jwt_claims()
        .ok()
        .flatten()
        .and_then(|claims| expires_at_ms(&claims))
        .is_some_and(|expires_at_ms| expires_at_ms <= clock::now_ms())
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::FoundProfile;
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{JsonField, KeyPlace, NeedAction};

    use super::*;
    use crate::app_server::APP_SERVER_ARGS;
    use crate::auth_file::ACCESS_TOKEN_FIELD;
    use crate::profiles::CONFIG_VARIABLE;
    use crate::testing::{TEN, as_provider, found_profile, host, path};

    /// A handle to the access token of the login of the config folder
    /// `name`, read at each use.
    fn token_of(host: &TestHost, name: &str) -> Key {
        host.key_at(&KeyPlace::JsonField(JsonField {
            file: path(host, &format!("{name}/auth.json")),
            pointer: ACCESS_TOKEN_FIELD.to_string(),
        }))
    }

    fn read(
        host: &TestHost,
        profile: &FoundProfile,
        name: &str,
    ) -> Result<Limits, ReadError> {
        let token = token_of(host, name);

        as_provider(host, || read_codex_limits(profile, &token))
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
        let profile = found_profile(&host, ".codex-plus", false);

        let limits = read(&host, &profile, ".codex-plus").expect("the limits");

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
        let profile = found_profile(&host, ".codex-limits", false);

        let limits =
            read(&host, &profile, ".codex-limits").expect("the limits");

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
        let profile = found_profile(&host, ".codex-expired", false);

        let limits = read(&host, &profile, ".codex-expired");

        let Err(ReadError::NeedsUser(need)) = limits else {
            panic!("a need: {limits:?}");
        };
        assert_eq!(need.id, "login-expired");
        assert!(need.text.contains("Open Codex"), "{}", need.text);
        assert_eq!(need.action, NeedAction::Run(profile.launch.clone()));
        assert!(host.started().is_empty());
    }

    #[test]
    fn a_token_that_does_not_tell_its_expiry_is_used() {
        let host = host().with_now_ms(TEN);
        let opaque = token_of(&host, ".codex");
        let fresh = token_of(&host, ".codex-plus");
        let missing = token_of(&host, ".codex-empty");

        let expired = as_provider(&host, || {
            [
                has_expired(&opaque),
                has_expired(&fresh),
                has_expired(&missing),
            ]
        });

        assert_eq!(expired, [false, false, false]);
    }
}
