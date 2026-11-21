//! Whether a Claude Code profile is logged in, and how it pays.

use otty_plugin_sdk::{Billing, Command, Connection, ProfileStatus, files};

use crate::config_files::{
    OauthAccount, OauthLogin, read_credentials, read_global_configs,
    read_settings,
};
use crate::glob::escaped;
use crate::needs;

/// Whether the profile in `folder` is logged in, as [`connection`] tells.
/// A profile nobody is logged in with needs the user to run `login`,
/// Claude Code's login.
pub(crate) fn status(folder: &str, login: Command) -> ProfileStatus {
    if !folder_exists(folder) {
        return ProfileStatus::ConfigFolderNotFound;
    }

    match connection(folder) {
        Some(connection) => ProfileStatus::Connected(connection),
        None => ProfileStatus::NotLoggedIn(needs::not_logged_in(login)),
    }
}

/// The login of the profile in `folder`; `None` when nobody is logged in.
///
/// A profile with an API key in its settings (`env.ANTHROPIC_API_KEY`,
/// `env.ANTHROPIC_AUTH_TOKEN` or `apiKeyHelper`) or a `primaryApiKey` in its global config file is an
/// API key profile; it has no subscription plan. So is a login to an
/// Anthropic Console organization (`billingType` `prepaid`) whose login
/// file names no plan: it pays for its tokens. Else a profile is logged
/// in, with a subscription, when its login file holds a token, or when its
/// global config file names an account: the login file can be an empty
/// object while the token is kept elsewhere. A blank key or token counts
/// as none.
pub(crate) fn connection(folder: &str) -> Option<Connection> {
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
    let account = configs
        .iter()
        .find_map(|config| config.fields().oauth_account());
    let login = credentials
        .as_ref()
        .and_then(|credentials| credentials.fields().login());
    let email = account.and_then(OauthAccount::email).map(String::from);
    let plan = login.and_then(OauthLogin::plan);
    let is_console = account.is_some_and(OauthAccount::is_console);

    if has_settings_key || has_console_key || (is_console && plan.is_none()) {
        return Some(connected(email, None, Billing::ApiKey));
    }

    let has_token = credentials.as_ref().is_some_and(|credentials| {
        credentials.holds_key(login.and_then(OauthLogin::token))
    });
    if !has_token && account.is_none() {
        return None;
    }

    Some(connected(email, plan, Billing::UsageWindows))
}

/// A login as `account` with `plan`, paid as `billing`.
fn connected(
    account: Option<String>,
    plan: Option<String>,
    billing: Billing,
) -> Connection {
    Connection {
        account,
        plan,
        billing: Some(billing),
    }
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

    /// A connected profile paid for by a subscription.
    fn subscription(
        account: Option<&str>,
        plan: Option<&str>,
    ) -> ProfileStatus {
        ProfileStatus::Connected(connected(
            account.map(String::from),
            plan.map(String::from),
            Billing::UsageWindows,
        ))
    }

    /// A connected profile paid for by an API key.
    fn api_key(account: Option<&str>) -> ProfileStatus {
        ProfileStatus::Connected(connected(
            account.map(String::from),
            None,
            Billing::ApiKey,
        ))
    }

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

    #[test]
    fn a_profile_with_a_token_is_connected_with_its_plan() {
        assert_eq!(
            status_of(".claude-token"),
            subscription(None, Some("Max 20x"))
        );
    }

    #[test]
    fn the_account_comes_from_the_global_config_file() {
        assert_eq!(
            status_of(".claude-work"),
            subscription(Some("own@example.com"), Some("Max 20x"))
        );
    }

    #[test]
    fn the_default_folder_finds_the_global_config_file_next_to_it() {
        assert_eq!(
            status_of(".claude"),
            subscription(Some("dev@example.com"), Some("Max 20x"))
        );
    }

    #[test]
    fn an_empty_credentials_file_with_an_account_is_connected() {
        assert_eq!(
            status_of(".claude-account"),
            subscription(Some("dev@example.com"), None)
        );
        assert_eq!(
            status_of(".claude-email"),
            subscription(Some("dev@example.com"), None)
        );
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

    #[test]
    fn an_api_key_in_the_settings_makes_an_api_key_profile() {
        assert_eq!(status_of(".claude-api"), api_key(None));
        assert_eq!(status_of(".claude-gateway"), api_key(None));
    }

    #[test]
    fn an_api_key_helper_in_the_settings_makes_an_api_key_profile() {
        assert_eq!(
            status_of(".claude-helper"),
            api_key(Some("dev@example.com"))
        );
    }

    #[test]
    fn a_primary_api_key_in_the_global_config_makes_an_api_key_profile() {
        assert_eq!(status_of(".claude-console"), api_key(None));
    }

    #[test]
    fn an_empty_or_null_api_key_is_no_api_key() {
        assert_eq!(
            status_of(".claude-nullkey"),
            subscription(None, Some("Max 20x"))
        );
    }

    /// A blank text is no text: the host finds no key in it.
    #[test]
    fn a_blank_token_or_api_key_is_none() {
        assert!(is_not_logged_in(".claude-blank"));
    }

    #[test]
    fn broken_settings_are_skipped() {
        assert_eq!(
            status_of(".claude-token"),
            subscription(None, Some("Max 20x"))
        );
    }

    /// A login to a Console organization is paid per token, even with no
    /// API key in the files.
    #[test]
    fn a_prepaid_console_login_with_no_plan_is_an_api_key_profile() {
        assert_eq!(
            status_of(".claude-prepaid"),
            api_key(Some("dev@example.com"))
        );
    }

    /// The plan in the login file is a subscription the files tell for
    /// sure, so it wins over a prepaid billing type.
    #[test]
    fn a_plan_in_the_login_file_wins_over_a_prepaid_billing_type() {
        assert_eq!(
            status_of(".claude-prepaid-max"),
            subscription(Some("dev@example.com"), Some("Max 20x"))
        );
    }

    /// Claude Code sends `ANTHROPIC_AUTH_TOKEN` as its key, such as a
    /// DeepSeek key behind DeepSeek's server.
    #[test]
    fn an_auth_token_in_the_settings_makes_an_api_key_profile() {
        assert_eq!(status_of(".claude-deepseek"), api_key(None));
    }

    #[test]
    fn a_missing_config_folder_is_not_found() {
        assert_eq!(
            status_of(".claude-gone"),
            ProfileStatus::ConfigFolderNotFound
        );
    }
}
