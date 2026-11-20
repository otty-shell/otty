//! The accounts found in a Claude Code profile, and the provider its tokens
//! are counted under.
//!
//! A profile can hold a Claude login (`.credentials.json` with the account
//! in `oauthAccount` of `.claude.json`), a Console key (`primaryApiKey` of
//! `.claude.json`) and a key in the `env` of `settings.json`
//! (`ANTHROPIC_AUTH_TOKEN` or `ANTHROPIC_API_KEY`), which is a key at the
//! provider of `ANTHROPIC_BASE_URL` when it is set. A key read here is
//! turned into its fingerprint at once and never kept.

use std::path::Path;

use super::config_files::{
    GlobalConfig, OauthAccount, Settings, read_global_configs, read_settings,
    read_settings_key,
};
use super::credentials::{OauthLogin, read_login};
use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
};
use crate::account_key::AccountKey;
use crate::built_in::ANTHROPIC;
use crate::key_fingerprint::KeyFingerprint;
use crate::profile::Profile;
use crate::provider::ProviderRef;

/// The accounts found in the files of `profile`: the login first, then the
/// Console key of `.claude.json`, then the key of the settings.
///
/// A login whose files name no id and no email can not be told from the
/// others, so it is no account. A key for another server than
/// Anthropic's is a key for the host of that server.
pub(super) fn accounts(profile: &Profile) -> Vec<Account> {
    let config_dir = profile.config_dir();
    let found_in = FoundIn::ProfileFolder(config_dir.to_path_buf());
    let global_configs = read_global_configs(config_dir);
    let settings = read_settings(config_dir);

    let login = global_configs
        .iter()
        .find_map(GlobalConfig::oauth_account)
        .and_then(|account| login_account(account, config_dir, &found_in));
    let console_key = global_configs
        .iter()
        .find_map(GlobalConfig::primary_api_key)
        .map(|key| {
            Account::of_key(
                ProviderRef::of_id(ANTHROPIC),
                key,
                found_in.clone(),
            )
        });
    let settings_key = settings.as_ref().and_then(|settings| {
        let provider = provider_of(Some(settings))?;
        let key = settings.env_key()?;

        Some(Account::of_key(provider, key, found_in.clone()))
    });

    [login, console_key, settings_key]
        .into_iter()
        .flatten()
        .collect()
}

/// The key of the settings of `profile`, read again for one request, when
/// it is the key of `account`: the same fingerprint. The Console key of
/// `.claude.json` is not read: an Anthropic key tells no money.
pub(super) fn settings_key_of(
    profile: &Profile,
    account: &Account,
) -> Option<AccountKey> {
    let AccountIdentity::KeyFingerprint(fingerprint) = account.identity()
    else {
        return None;
    };

    let key = read_settings_key(profile.config_dir())?;

    (KeyFingerprint::of(key.secret()) == *fingerprint)
        .then(|| AccountKey::new(key))
}

/// The provider the tokens of the profile in `config_dir` are counted
/// under, since Claude Code does not write it (spec Decision 4): the host
/// of `ANTHROPIC_BASE_URL` in its settings when it is set, else Anthropic.
/// `None` for a base URL that is not an address of the web.
pub(super) fn profile_provider(config_dir: &Path) -> Option<ProviderRef> {
    provider_of(read_settings(config_dir).as_ref())
}

/// The provider of the server `settings` send Claude Code to: the host of
/// `ANTHROPIC_BASE_URL`, else Anthropic.
fn provider_of(settings: Option<&Settings>) -> Option<ProviderRef> {
    match settings.and_then(Settings::base_url) {
        Some(base_url) => ProviderRef::of_base_url(base_url),
        None => Some(ProviderRef::of_id(ANTHROPIC)),
    }
}

/// The account of the Claude login `account` of the profile in
/// `config_dir`, with the plan of its credentials file.
///
/// A login to a Console organization with no plan pays per token, and its
/// bill is the organization's, so the organization is the account. Any
/// other login is a subscription, told by its own id.
fn login_account(
    account: &OauthAccount,
    config_dir: &Path,
    found_in: &FoundIn,
) -> Option<Account> {
    let plan = read_login(config_dir).as_ref().and_then(OauthLogin::plan);
    let is_paid_per_token = account.is_console() && plan.is_none();

    let identity = login_identity(account, is_paid_per_token)?;
    let login = match account.email() {
        Some(email) => AccountLogin::Email(email.to_string()),
        None => AccountLogin::Unnamed,
    };
    let kind = if is_paid_per_token {
        AccountKind::Money
    } else {
        AccountKind::Limits
    };

    Some(
        Account::new(
            ProviderRef::of_id(ANTHROPIC),
            identity,
            login,
            kind,
            found_in.clone(),
        )
        .with_plan(plan),
    )
}

/// What tells the login `account` from the others: its organization when
/// it `is_paid_per_token`, else its own id, else its email.
fn login_identity(
    account: &OauthAccount,
    is_paid_per_token: bool,
) -> Option<AccountIdentity> {
    if is_paid_per_token && let Some(id) = account.organization_id() {
        return Some(AccountIdentity::OrganizationId(id.to_string()));
    }

    if let Some(id) = account.account_id() {
        return Some(AccountIdentity::AccountId(id.to_string()));
    }

    let email = account.email()?;

    Some(AccountIdentity::Email(email.to_string()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::key_fingerprint::KeyFingerprint;
    use crate::testing::{TempDir, claude_profile, write_file};

    const TOKEN: &str = "made-up-access-token-0000";
    const KEY: &str = "sk-ant-made-up-key-0000-a1f3";

    const SUBSCRIPTION_CREDENTIALS: &str = r#"{
        "claudeAiOauth": {
            "accessToken": "made-up-access-token-0000",
            "refreshToken": "made-up-refresh-token-0000",
            "expiresAt": 1794823200000,
            "scopes": ["user:inference", "user:profile"],
            "subscriptionType": "max",
            "rateLimitTier": "default_claude_max_20x"
        }
    }"#;

    /// A `.claude.json` with the login of `billing` as its billing type.
    fn global_config(billing: &str) -> String {
        format!(
            r#"{{
                "numStartups": 4,
                "oauthAccount": {{
                    "accountUuid": "00000000-0000-4000-8000-00000000000a",
                    "emailAddress": "dev@example.com",
                    "organizationUuid": "00000000-0000-4000-8000-00000000000b",
                    "organizationName": "Example Org",
                    "billingType": "{billing}"
                }}
            }}"#
        )
    }

    fn config_dir(temp: &TempDir) -> PathBuf {
        let dir = temp.path().join(".claude-work");
        std::fs::create_dir_all(&dir).expect("create config folder");
        dir
    }

    fn accounts_in(config_dir: &Path) -> Vec<Account> {
        accounts(&claude_profile(config_dir))
    }

    fn key_account(provider: ProviderRef, config_dir: &Path) -> Account {
        Account::new(
            provider,
            AccountIdentity::KeyFingerprint(KeyFingerprint::of(KEY)),
            AccountLogin::Key(Some("a1f3".into())),
            AccountKind::Money,
            FoundIn::ProfileFolder(config_dir.to_path_buf()),
        )
    }

    #[test]
    fn a_subscription_login_is_a_claude_account_with_limits() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join(".credentials.json"), SUBSCRIPTION_CREDENTIALS);
        write_file(
            &dir.join(".claude.json"),
            &global_config("stripe_subscription"),
        );

        let expected = Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::AccountId(
                "00000000-0000-4000-8000-00000000000a".into(),
            ),
            AccountLogin::Email("dev@example.com".into()),
            AccountKind::Limits,
            FoundIn::ProfileFolder(dir.clone()),
        )
        .with_plan(Some("Max 20x".into()));
        assert_eq!(accounts_in(&dir), [expected]);
    }

    /// The bill of a Console organization belongs to the organization, so
    /// two people logged in to it are one account.
    #[test]
    fn a_prepaid_login_is_its_organization_paid_per_token() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&dir.join(".claude.json"), &global_config("prepaid"));

        let expected = Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::OrganizationId(
                "00000000-0000-4000-8000-00000000000b".into(),
            ),
            AccountLogin::Email("dev@example.com".into()),
            AccountKind::Money,
            FoundIn::ProfileFolder(dir.clone()),
        );
        assert_eq!(accounts_in(&dir), [expected]);
    }

    #[test]
    fn the_default_folder_finds_the_login_next_to_it() {
        let temp = TempDir::new();
        let dir = temp.path().join(".claude");
        std::fs::create_dir_all(&dir).expect("create config folder");
        write_file(&dir.join(".credentials.json"), SUBSCRIPTION_CREDENTIALS);
        write_file(
            &temp.path().join(".claude.json"),
            &global_config("stripe_subscription"),
        );

        let found = accounts_in(&dir);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].plan(), Some("Max 20x"));
    }

    #[test]
    fn a_login_with_no_id_is_told_by_its_email() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join(".claude.json"),
            r#"{"oauthAccount": {"emailAddress": "dev@example.com"}}"#,
        );

        let found = accounts_in(&dir);

        assert_eq!(
            found.iter().map(Account::identity).collect::<Vec<_>>(),
            [&AccountIdentity::Email("dev@example.com".into())]
        );
    }

    #[test]
    fn a_token_with_no_account_in_the_files_is_no_account() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join(".credentials.json"), SUBSCRIPTION_CREDENTIALS);

        assert_eq!(accounts_in(&dir), []);
    }

    #[test]
    fn a_primary_api_key_is_an_anthropic_key() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join(".claude.json"),
            &format!(r#"{{"primaryApiKey": "{KEY}"}}"#),
        );

        assert_eq!(
            accounts_in(&dir),
            [key_account(ProviderRef::of_id("anthropic"), &dir)]
        );
    }

    #[test]
    fn an_api_key_in_the_settings_is_an_anthropic_key() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join("settings.json"),
            &format!(
                r#"{{"model": "opus", "env": {{"ANTHROPIC_API_KEY": "{KEY}", "DISABLE_TELEMETRY": "1"}}}}"#
            ),
        );

        assert_eq!(
            accounts_in(&dir),
            [key_account(ProviderRef::of_id("anthropic"), &dir)]
        );
        assert_eq!(
            profile_provider(&dir),
            Some(ProviderRef::of_id("anthropic"))
        );
    }

    #[test]
    fn a_token_for_a_deepseek_base_url_is_a_key_for_its_host() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join("settings.json"),
            &format!(
                r#"{{"env": {{
                    "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                    "ANTHROPIC_AUTH_TOKEN": "{KEY}",
                    "ANTHROPIC_MODEL": "deepseek-chat"
                }}}}"#
            ),
        );

        let deepseek = ProviderRef::Host(String::from("api.deepseek.com"));
        assert_eq!(accounts_in(&dir), [key_account(deepseek.clone(), &dir)]);
        assert_eq!(profile_provider(&dir), Some(deepseek));
    }

    /// Claude Code sends the auth token before the API key, so the token
    /// is the key in use.
    #[test]
    fn the_auth_token_comes_before_the_api_key() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join("settings.json"),
            &format!(
                r#"{{"env": {{
                    "ANTHROPIC_AUTH_TOKEN": "{KEY}",
                    "ANTHROPIC_API_KEY": "sk-ant-made-up-other-key-9999"
                }}}}"#
            ),
        );

        assert_eq!(
            accounts_in(&dir),
            [key_account(ProviderRef::of_id("anthropic"), &dir)]
        );
    }

    /// Spec 2026-11-19-agent-plugins, "Identity and links do not depend on
    /// a provider plugin": a key for a server is an account known by the
    /// host of the server; the registry names its provider later, if any
    /// entry serves the host.
    #[test]
    fn a_key_for_any_other_server_is_an_account_of_its_host() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join("settings.json"),
            &format!(
                r#"{{"env": {{
                    "ANTHROPIC_BASE_URL": "https://gateway.example.com",
                    "ANTHROPIC_AUTH_TOKEN": "{KEY}"
                }}}}"#
            ),
        );

        let gateway = ProviderRef::Host(String::from("gateway.example.com"));
        assert_eq!(accounts_in(&dir), [key_account(gateway.clone(), &dir)]);
        assert_eq!(profile_provider(&dir), Some(gateway));
    }

    #[test]
    fn a_login_and_a_key_in_one_profile_are_two_accounts() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join(".credentials.json"), SUBSCRIPTION_CREDENTIALS);
        write_file(
            &dir.join(".claude.json"),
            &global_config("stripe_subscription"),
        );
        write_file(
            &dir.join("settings.json"),
            &format!(r#"{{"env": {{"ANTHROPIC_API_KEY": "{KEY}"}}}}"#),
        );

        let kinds: Vec<AccountKind> =
            accounts_in(&dir).iter().map(Account::kind).collect();

        assert_eq!(kinds, [AccountKind::Limits, AccountKind::Money]);
    }

    #[test]
    fn a_profile_with_no_files_has_no_account_and_counts_under_anthropic() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join("settings.json"), "{\"env\": ");

        assert_eq!(accounts_in(&dir), []);
        assert_eq!(
            profile_provider(&dir),
            Some(ProviderRef::of_id("anthropic"))
        );
    }

    #[test]
    fn no_key_or_token_is_in_the_debug_output() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join(".credentials.json"), SUBSCRIPTION_CREDENTIALS);
        write_file(
            &dir.join(".claude.json"),
            &format!(
                r#"{{"primaryApiKey": "{KEY}", "oauthAccount": {{"accountUuid": "made-up-id"}}}}"#
            ),
        );

        let found = accounts_in(&dir);
        let debug = format!("{found:?}");

        assert_eq!(found.len(), 2);
        assert!(!debug.contains(KEY), "{debug}");
        assert!(!debug.contains(TOKEN), "{debug}");
        assert!(!debug.contains("made-up-refresh-token"), "{debug}");
    }

    /// Spec 2026-11-19, Decision 5: the key of the settings is read again
    /// for the request of the money of its account, and only for it.
    #[test]
    fn the_key_of_the_settings_is_read_again_for_its_own_account() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join("settings.json"),
            &format!(
                r#"{{"env": {{
                    "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                    "ANTHROPIC_AUTH_TOKEN": " {KEY}\n"
                }}}}"#
            ),
        );
        let profile = claude_profile(&dir);
        let other = Account::new(
            ProviderRef::of_id("deepseek"),
            AccountIdentity::KeyFingerprint(KeyFingerprint::of("sk-other")),
            AccountLogin::Key(None),
            AccountKind::Money,
            FoundIn::ProfileFolder(dir.clone()),
        );

        let key = settings_key_of(
            &profile,
            &key_account(ProviderRef::of_id("deepseek"), &dir),
        );
        let not_this = settings_key_of(&profile, &other);

        assert_eq!(
            key.map(|key| key.key().secret().to_string()),
            Some(String::from(KEY))
        );
        assert!(not_this.is_none());
    }
}
