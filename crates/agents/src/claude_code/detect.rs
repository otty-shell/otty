//! Whether a Claude Code profile can be used: its program, its config
//! folder and its login.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::credentials::{OauthLogin, read_login};
use crate::billing::BillingKind;
use crate::json_file::{is_filled_text, read_json};
use crate::profile::{Profile, ProfileStatus};

/// The billing types of `oauthAccount` in `.claude.json` that mean an
/// Anthropic Console organization, paid per token.
const CONSOLE_BILLING_TYPES: [&str; 1] = ["prepaid"];

/// The parts of Claude Code's global config file, `.claude.json`, that tell
/// about the login.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GlobalConfig {
    oauth_account: Option<OauthAccount>,
    /// The key of a login with an Anthropic Console API key. Only checked
    /// for being there and never kept.
    #[serde(
        rename = "primaryApiKey",
        default,
        deserialize_with = "is_filled_text"
    )]
    has_primary_api_key: bool,
}

/// The parts of `<config>/settings.json` that tell about an API key.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    env: Option<SettingsEnv>,
    /// A command that prints the API key. Only checked for being there.
    #[serde(
        rename = "apiKeyHelper",
        default,
        deserialize_with = "is_filled_text"
    )]
    has_api_key_helper: bool,
}

impl Settings {
    /// Whether the settings give Claude Code an API key.
    fn has_api_key(&self) -> bool {
        let has_env_key = self.env.as_ref().is_some_and(|env| env.has_api_key);

        has_env_key || self.has_api_key_helper
    }
}

/// The variables the settings set for Claude Code.
#[derive(Debug, Deserialize)]
struct SettingsEnv {
    /// Only checked for being there and never kept.
    #[serde(
        rename = "ANTHROPIC_API_KEY",
        default,
        deserialize_with = "is_filled_text"
    )]
    has_api_key: bool,
}

/// The account a profile is logged in with.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OauthAccount {
    email_address: Option<String>,
    /// How the organization of the login pays, such as
    /// `stripe_subscription` for a Claude plan or `prepaid` for an
    /// Anthropic Console organization.
    billing_type: Option<String>,
}

impl OauthAccount {
    /// Whether the account belongs to an Anthropic Console organization,
    /// which pays for the tokens it uses rather than for a plan.
    fn is_console(&self) -> bool {
        self.billing_type.as_deref().is_some_and(|billing| {
            CONSOLE_BILLING_TYPES.contains(&billing.trim())
        })
    }
}

/// Whether `profile` is installed and logged in.
///
/// A profile with an API key in its settings (`env.ANTHROPIC_API_KEY` or
/// `apiKeyHelper`) or a `primaryApiKey` in its global config file is an API
/// key profile; it has no subscription plan. So is a login to an Anthropic
/// Console organization (`billingType` `prepaid`) whose credentials file
/// names no plan: it pays for its tokens. Else a profile is logged in,
/// with a subscription, when its credentials file holds a token, or when
/// its global config file names an account: the credentials file can be an
/// empty object while the token is kept elsewhere. A blank key or token
/// counts as none. Looking for `claude` on `PATH` is left to the app.
pub(super) fn detect(profile: &Profile) -> ProfileStatus {
    let config_dir = profile.config_dir();

    if !config_dir.is_dir() {
        return ProfileStatus::ConfigDirNotFound;
    }

    let login = read_login(config_dir);
    let global_configs: Vec<GlobalConfig> = global_config_files(config_dir)
        .iter()
        .filter_map(|path| read_json(path))
        .collect();
    let settings = read_json::<Settings>(&config_dir.join("settings.json"));

    let has_api_key = settings.is_some_and(|settings| settings.has_api_key())
        || global_configs
            .iter()
            .any(|config| config.has_primary_api_key);
    let account = global_configs
        .into_iter()
        .find_map(|config| config.oauth_account);
    let has_account = account.is_some();
    let is_console = account.as_ref().is_some_and(OauthAccount::is_console);
    let email = account.and_then(|account| account.email_address);
    let plan = login.as_ref().and_then(OauthLogin::plan);

    if has_api_key || (is_console && plan.is_none()) {
        return ProfileStatus::Connected {
            account: email,
            plan: None,
            billing: Some(BillingKind::ApiKey),
        };
    }

    let has_token = login.as_ref().is_some_and(|login| login.token().is_some());
    if !has_token && !has_account {
        return ProfileStatus::NotLoggedIn;
    }

    ProfileStatus::Connected {
        account: email,
        plan,
        billing: Some(BillingKind::UsageWindows),
    }
}

/// Where the global config file of the profile in `config_dir` can be,
/// in the order to look.
///
/// With `CLAUDE_CONFIG_DIR` set, Claude Code keeps the file inside the
/// config folder. Without it, the config folder is `~/.claude` and the file
/// is `~/.claude.json`, next to the folder.
fn global_config_files(config_dir: &Path) -> Vec<PathBuf> {
    let mut files = vec![config_dir.join(".claude.json")];

    let is_default_folder =
        config_dir.file_name().is_some_and(|name| name == ".claude");
    if let Some(parent) = config_dir.parent()
        && is_default_folder
    {
        files.push(parent.join(".claude.json"));
    }

    files
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::testing::{TempDir, claude_profile, write_file};

    const TOKEN_CREDENTIALS: &str = r#"{
        "claudeAiOauth": {
            "accessToken": "made-up-access-token",
            "refreshToken": "made-up-refresh-token",
            "expiresAt": 1794823200000,
            "scopes": ["user:inference"],
            "subscriptionType": "max",
            "rateLimitTier": "default_claude_max_20x"
        }
    }"#;

    const ACCOUNT_CONFIG: &str = r#"{
        "numStartups": 12,
        "oauthAccount": {
            "accountUuid": "00000000-0000-4000-8000-000000000000",
            "emailAddress": "dev@example.com",
            "organizationName": "Example Org"
        },
        "projects": {}
    }"#;

    /// A connected profile paid for by a subscription.
    fn connected(account: Option<&str>, plan: Option<&str>) -> ProfileStatus {
        ProfileStatus::Connected {
            account: account.map(String::from),
            plan: plan.map(String::from),
            billing: Some(BillingKind::UsageWindows),
        }
    }

    /// A connected profile paid for by an API key.
    fn api_key(account: Option<&str>) -> ProfileStatus {
        ProfileStatus::Connected {
            account: account.map(String::from),
            plan: None,
            billing: Some(BillingKind::ApiKey),
        }
    }

    fn config_dir(temp: &TempDir, name: &str) -> PathBuf {
        let dir = temp.path().join(name);
        std::fs::create_dir_all(&dir).expect("create config folder");
        dir
    }

    fn detect_in(config_dir: &Path) -> ProfileStatus {
        detect(&claude_profile(config_dir))
    }

    #[test]
    fn a_profile_with_a_token_is_connected_with_its_plan() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);

        assert_eq!(detect_in(&dir), connected(None, Some("Max 20x")));
    }

    #[test]
    fn the_account_comes_from_the_global_config_file() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        write_file(&dir.join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(
            detect_in(&dir),
            connected(Some("dev@example.com"), Some("Max 20x"))
        );
    }

    #[test]
    fn an_empty_credentials_file_with_an_account_is_connected() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&dir.join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(detect_in(&dir), connected(Some("dev@example.com"), None));
    }

    #[test]
    fn the_default_folder_finds_the_global_config_file_next_to_it() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&temp.path().join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(detect_in(&dir), connected(Some("dev@example.com"), None));
    }

    #[test]
    fn the_global_config_file_in_the_folder_comes_first() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude");
        let own_config = ACCOUNT_CONFIG.replace("dev@", "own@");
        write_file(&dir.join(".claude.json"), &own_config);
        write_file(&temp.path().join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(detect_in(&dir), connected(Some("own@example.com"), None));
    }

    #[test]
    fn another_folder_does_not_look_for_the_global_config_file_next_to_it() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&temp.path().join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn an_empty_credentials_file_is_not_logged_in() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&dir.join(".claude.json"), r#"{"numStartups": 1}"#);

        assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn broken_files_are_not_logged_in() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{\"claudeAiOauth\": ");
        write_file(&dir.join(".claude.json"), "not json");

        assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn an_api_key_in_the_settings_makes_an_api_key_profile() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(
            &dir.join("settings.json"),
            r#"{"model": "opus", "env": {"ANTHROPIC_API_KEY": "sk-ant-made-up-key", "DISABLE_TELEMETRY": "1"}}"#,
        );

        assert_eq!(detect_in(&dir), api_key(None));
    }

    #[test]
    fn an_api_key_helper_in_the_settings_makes_an_api_key_profile() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        write_file(&dir.join(".claude.json"), ACCOUNT_CONFIG);
        write_file(
            &dir.join("settings.json"),
            r#"{"apiKeyHelper": "/home/user/bin/made-up-key-helper.sh"}"#,
        );

        assert_eq!(detect_in(&dir), api_key(Some("dev@example.com")));
    }

    #[test]
    fn a_primary_api_key_in_the_global_config_makes_an_api_key_profile() {
        let temp = TempDir::new();
        let added = config_dir(&temp, ".claude-work");
        write_file(
            &added.join(".claude.json"),
            r#"{"numStartups": 3, "primaryApiKey": "sk-ant-made-up-key"}"#,
        );
        let default = config_dir(&temp, ".claude");
        write_file(
            &temp.path().join(".claude.json"),
            r#"{"primaryApiKey": "sk-ant-made-up-key"}"#,
        );

        assert_eq!(detect_in(&added), api_key(None));
        assert_eq!(detect_in(&default), api_key(None));
    }

    #[test]
    fn an_empty_or_null_api_key_is_no_api_key() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        write_file(
            &dir.join("settings.json"),
            r#"{"env": {"ANTHROPIC_API_KEY": ""}, "apiKeyHelper": null}"#,
        );
        write_file(&dir.join(".claude.json"), r#"{"primaryApiKey": null}"#);

        assert_eq!(detect_in(&dir), connected(None, Some("Max 20x")));
    }

    /// A blank text is no text: the usage request does not send a blank
    /// token, so a blank one does not make the profile logged in either.
    #[test]
    fn a_blank_token_or_api_key_is_none() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(
            &dir.join(".credentials.json"),
            r#"{"claudeAiOauth": {"accessToken": "  "}}"#,
        );
        write_file(
            &dir.join("settings.json"),
            r#"{"env": {"ANTHROPIC_API_KEY": "  "}, "apiKeyHelper": " "}"#,
        );

        assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn broken_settings_are_skipped() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        write_file(&dir.join("settings.json"), "{\"env\": ");

        assert_eq!(detect_in(&dir), connected(None, Some("Max 20x")));
    }

    /// A `.claude.json` of a login to an organization of the Anthropic
    /// Console, with `billing` as its billing type.
    fn console_config(billing: &str) -> String {
        format!(
            r#"{{
                "numStartups": 4,
                "oauthAccount": {{
                    "accountUuid": "00000000-0000-4000-8000-000000000001",
                    "emailAddress": "dev@example.com",
                    "organizationName": "Example Console Org",
                    "billingType": "{billing}"
                }}
            }}"#
        )
    }

    /// Owner, issue 27: a login to a Console organization is paid per
    /// token, even with no API key in the files.
    #[test]
    fn a_prepaid_console_login_is_an_api_key_profile() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&dir.join(".claude.json"), &console_config("prepaid"));

        assert_eq!(detect_in(&dir), api_key(Some("dev@example.com")));
    }

    #[test]
    fn a_prepaid_console_login_with_a_token_and_no_plan_is_an_api_key_profile()
    {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude");
        write_file(
            &dir.join(".credentials.json"),
            r#"{"claudeAiOauth": {"accessToken": "made-up-access-token", "subscriptionType": null}}"#,
        );
        write_file(
            &temp.path().join(".claude.json"),
            &console_config("prepaid"),
        );

        assert_eq!(detect_in(&dir), api_key(Some("dev@example.com")));
    }

    #[test]
    fn a_subscription_login_stays_a_subscription() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        write_file(
            &dir.join(".claude.json"),
            &console_config("stripe_subscription"),
        );

        assert_eq!(
            detect_in(&dir),
            connected(Some("dev@example.com"), Some("Max 20x"))
        );
    }

    /// The plan in the token file is a subscription the files tell for
    /// sure, so it wins over a prepaid billing type.
    #[test]
    fn a_plan_in_the_token_file_wins_over_a_prepaid_billing_type() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        write_file(&dir.join(".claude.json"), &console_config("prepaid"));

        assert_eq!(
            detect_in(&dir),
            connected(Some("dev@example.com"), Some("Max 20x"))
        );
    }

    #[test]
    fn a_missing_config_folder_is_not_found() {
        let temp = TempDir::new();

        let status = detect_in(&temp.path().join(".claude-gone"));

        assert_eq!(status, ProfileStatus::ConfigDirNotFound);
    }

    #[test]
    fn the_plan_reads_like_the_name_claude_code_shows() {
        let credentials = |subscription: &str, tier: &str| {
            format!(
                r#"{{"claudeAiOauth": {{"accessToken": "made-up",
                "subscriptionType": "{subscription}",
                "rateLimitTier": "{tier}"}}}}"#
            )
        };
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        let cases = [
            ("pro", "default_claude_pro", "Pro"),
            ("max", "default_claude_max_5x", "Max 5x"),
            ("team", "default_raven", "Team"),
        ];

        for (subscription, tier, plan) in cases {
            write_file(
                &dir.join(".credentials.json"),
                &credentials(subscription, tier),
            );

            assert_eq!(detect_in(&dir), connected(None, Some(plan)), "{tier}");
        }
    }
}
