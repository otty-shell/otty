//! Whether a Claude Code profile can be used: its config folder and
//! whether a login is there. Whose login it is and how it pays, the agent
//! part does not say: the provider part `anthropic` finds the account
//! (spec 2026-11-23-accounts-apart-from-profiles, Decision 2).

use otty_plugin_sdk::agents::ProfileStatus;
use otty_plugin_sdk::{Command, files};

use crate::config_files::{
    OauthLogin, read_credentials, read_global_configs, read_settings,
};
use crate::glob::escaped;
use crate::needs;

/// Whether the profile in `folder` can be used: found when it holds a
/// login ([`has_login`]), else not logged in with the need to run
/// `login`, Claude Code's login.
pub(crate) fn status(folder: &str, login: Command) -> ProfileStatus {
    if !folder_exists(folder) {
        return ProfileStatus::ConfigFolderNotFound;
    }

    match has_login(folder) {
        true => ProfileStatus::Found,
        false => ProfileStatus::NotLoggedIn(needs::not_logged_in(login)),
    }
}

/// Whether the profile in `folder` holds a login Claude Code can use: an
/// API key in its settings (`env.ANTHROPIC_API_KEY`,
/// `env.ANTHROPIC_AUTH_TOKEN` or `apiKeyHelper`), a `primaryApiKey` in its
/// global config file, a token in its login file, or an account its
/// global config file names: the login file can be an empty object while
/// the token is kept elsewhere. A blank key or token counts as none.
fn has_login(folder: &str) -> bool {
    let credentials = read_credentials(folder);
    let configs = read_global_configs(folder);
    let settings = read_settings(folder);

    let has_settings_key = settings.as_ref().is_some_and(|settings| {
        settings.fields().has_api_key_helper()
            || settings.holds_key(settings.fields().api_key())
            || settings.holds_key(settings.fields().auth_token())
    });
    let has_console_key = configs
        .iter()
        .any(|config| config.holds_key(config.fields().primary_api_key()));
    let has_account = configs
        .iter()
        .any(|config| config.fields().oauth_account().is_some());
    let has_token = credentials.as_ref().is_some_and(|credentials| {
        let login = credentials.fields().login();
        credentials.holds_key(login.and_then(OauthLogin::token))
    });

    has_settings_key || has_console_key || has_account || has_token
}

/// Whether the config folder `folder` holds any file. A folder is never
/// listed itself, so an empty folder counts as missing.
fn folder_exists(folder: &str) -> bool {
    let folder = escaped(folder);
    let has_files =
        |glob: String| files::list(&glob).is_ok_and(|found| !found.is_empty());

    has_files(format!("{folder}/*")) || has_files(format!("{folder}/**"))
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::Command;

    use super::*;
    use crate::testing::{as_part, host, path};

    fn status_of(folder: &str) -> ProfileStatus {
        let host = host();
        let folder = path(&host, folder);
        let login = Command {
            program: "claude".to_string(),
            args: Vec::new(),
            env: Vec::new(),
        };

        as_part(&host, || status(&folder, login))
    }

    /// Whether nobody is logged in with the config folder `folder`.
    fn is_not_logged_in(folder: &str) -> bool {
        matches!(status_of(folder), ProfileStatus::NotLoggedIn(_))
    }

    /// Decision 2: a token, an account the global config names, a key in
    /// the settings or the global config, and a Console login are all
    /// found, with no word on whose login it is or how it pays.
    #[test]
    fn every_kind_of_login_is_found() {
        for folder in [
            ".claude-token",
            ".claude-work",
            ".claude",
            ".claude-account",
            ".claude-email",
            ".claude-api",
            ".claude-gateway",
            ".claude-helper",
            ".claude-console",
            ".claude-nullkey",
            ".claude-prepaid",
            ".claude-prepaid-max",
            ".claude-deepseek",
        ] {
            assert_eq!(status_of(folder), ProfileStatus::Found, "{folder}");
        }
    }

    /// `~/.claude.json` names an account, but it belongs to `~/.claude`.
    #[test]
    fn another_folder_with_no_login_is_not_logged_in() {
        assert!(is_not_logged_in(".claude-nothing"));
    }

    #[test]
    fn broken_files_are_not_logged_in() {
        assert!(is_not_logged_in(".claude-broken"));
    }

    /// A blank text is no text: the host finds no key in it.
    #[test]
    fn a_blank_token_or_api_key_is_none() {
        assert!(is_not_logged_in(".claude-blank"));
    }

    #[test]
    fn a_missing_config_folder_is_not_found() {
        assert_eq!(
            status_of(".claude-gone"),
            ProfileStatus::ConfigFolderNotFound
        );
    }
}
