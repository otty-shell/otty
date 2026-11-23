//! The limits Codex itself knows of a ChatGPT login, read by the provider
//! part `openai` in a Codex profile folder the account was found in
//! (issue 29): `codex app-server` first, started with `CODEX_HOME` of that
//! folder, which tells them now, then the last record of them in the
//! session files. When neither answers, the error of `codex app-server`
//! says why.
//!
//! `codex app-server` renews an expired login token by itself, which a
//! plugin never makes another agent do: with an expired token it is not
//! started, and the read waits for the user to log in again in the folder.
//! With no token in the folder there is no login to ask about, and only
//! the session files are read.

use otty_plugin_sdk::Key;
use otty_plugin_sdk::agents::ReadError;

use crate::limits::Limits;
use crate::needs::login_expired;
use crate::profiles::CONFIG_VARIABLE;
use crate::token_expiry::has_expired;
use crate::{app_server, session_limits};

/// The limits Codex knows in the profile folder `folder`, whose login
/// token is `token` when the folder holds one.
pub(crate) fn read_codex_limits(
    folder: &str,
    token: Option<&Key>,
) -> Result<Limits, ReadError> {
    let now = match token {
        None => Err(ReadError::Failed("no login in the folder".to_string())),
        Some(token) if has_expired(token) => {
            Err(ReadError::NeedsUser(login_expired(folder)))
        },
        Some(_) => {
            let env = [(CONFIG_VARIABLE.to_string(), folder.to_string())];
            app_server::read_limits(&env)
        },
    };

    now.or_else(|error| session_limits::read_limits(folder).map_err(|_| error))
}

/// The limits `codex app-server` tells in OTTY's own login folder
/// `folder` (spec 2026-11-23-accounts-apart-from-profiles, Decision 7).
/// It starts there even with the token past its time: the folder is
/// OTTY's alone, and Codex renews its own login as it always does, so no
/// agent the user works with is logged out. The part never renews a token
/// itself, and the session files there are not read.
pub(crate) fn read_own_login_limits(folder: &str) -> Result<Limits, ReadError> {
    let env = [(CONFIG_VARIABLE.to_string(), folder.to_string())];

    app_server::read_limits(&env)
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{JsonField, KeyPlace, NeedAction};

    use super::*;
    use crate::app_server::APP_SERVER_ARGS;
    use crate::auth_file::ACCESS_TOKEN_FIELD;
    use crate::testing::{PROVIDER, TEN, codex_folder, host, path};

    /// A handle to the access token of the login of the config folder
    /// `name`, read at each use.
    fn token_of(host: &TestHost, name: &str) -> Key {
        host.key_at(&KeyPlace::JsonField(JsonField {
            file: path(host, &format!("{name}/auth.json")),
            pointer: ACCESS_TOKEN_FIELD.to_string(),
        }))
    }

    /// The limits Codex knows in the config folder `name`, read in a call
    /// given that folder, with its token or with none.
    fn read(
        host: &TestHost,
        name: &str,
        with_token: bool,
    ) -> Result<Limits, ReadError> {
        let folders = [codex_folder(host, name)];
        let folder = path(host, name);
        let token = with_token.then(|| token_of(host, name));

        host.as_provider_in(PROVIDER, &folders, || {
            read_codex_limits(&folder, token.as_ref())
        })
    }

    #[test]
    fn the_app_server_runs_in_the_accounts_folder() {
        let host = host();
        host.answer_talk(
            "codex",
            &APP_SERVER_ARGS,
            &[
                r#"{"id":1,"result":{}}"#,
                r#"{"id":2,"result":{"rateLimits":{"primary":{"usedPercent":42,"windowDurationMins":300,"resetsAt":1794826800},"planType":"pro"}}}"#,
            ],
        );

        let limits = read(&host, ".codex-plus", true).expect("the limits");

        assert_eq!(limits.plan.as_deref(), Some("Pro 200"));
        assert_eq!(
            host.started()[0].env(),
            [(CONFIG_VARIABLE.to_string(), path(&host, ".codex-plus"))]
        );
    }

    /// `codex app-server` does not start here: the last record of the
    /// session files answers in its place.
    #[test]
    fn without_the_app_server_the_session_files_give_their_last_record() {
        let host = host();

        let limits = read(&host, ".codex-limits", true).expect("the limits");

        assert_eq!(limits.plan.as_deref(), Some("Plus"));
        assert!(limits.recorded_at_ms.is_some());
    }

    /// A folder with no login now has only what Codex wrote: nothing is
    /// started.
    #[test]
    fn with_no_token_only_the_session_files_are_read() {
        let host = host();
        host.answer_talk("codex", &APP_SERVER_ARGS, &[]);

        let limits = read(&host, ".codex-limits", false).expect("the limits");

        assert_eq!(limits.plan.as_deref(), Some("Plus"));
        assert!(host.started().is_empty());
    }

    /// `codex app-server` would renew the expired token, writing Codex's
    /// login: it is not started, and with no session files to fall back
    /// on the read waits for the user to log in again in the folder.
    #[test]
    fn an_expired_login_starts_no_app_server_and_asks_to_log_in() {
        let host = host().with_now_ms(TEN);
        host.answer_talk("codex", &APP_SERVER_ARGS, &[]);
        let folder = path(&host, ".codex-expired");

        let limits = read(&host, ".codex-expired", true);

        let Err(ReadError::NeedsUser(need)) = limits else {
            panic!("a need: {limits:?}");
        };
        assert_eq!(need, host.as_provider(PROVIDER, || login_expired(&folder)));
        let NeedAction::Run(command) = need.action else {
            panic!("a command: {need:?}");
        };
        assert_eq!(command.args, ["login"]);
        assert!(host.started().is_empty());
    }
}
