//! The logins and keys found in a Claude Code profile, each named by where
//! its key sits, never by the key.
//!
//! A profile can hold a Claude login (the token of `.credentials.json`,
//! the account in `oauthAccount` of `.claude.json`), a Console key
//! (`primaryApiKey` of `.claude.json`) and a key in the `env` of
//! `settings.json` (`ANTHROPIC_AUTH_TOKEN` or `ANTHROPIC_API_KEY`), which
//! is a key for the host of `ANTHROPIC_BASE_URL` when it is set.

use otty_plugin_sdk::{
    Identity, JsonField, KeyPlace, Login, LoginKind, ProviderRef,
};

use crate::config_files::{
    API_KEY_FIELD, AUTH_TOKEN_FIELD, GlobalConfig, KeyFile, OauthAccount,
    OauthLogin, PRIMARY_API_KEY_FIELD, Settings, TOKEN_FIELD, credentials_path,
    read_credentials, read_global_configs, read_settings,
};
use crate::provider::{ANTHROPIC, provider_of};

/// The logins of the profile in `folder`: the Claude login first, then
/// the Console key of `.claude.json`, then the key of the settings.
pub(crate) fn logins(folder: &str) -> Vec<Login> {
    let configs = read_global_configs(folder);
    let settings = read_settings(folder);

    let claude_login = configs
        .iter()
        .find_map(|config| config.fields().oauth_account())
        .and_then(|account| claude_login(folder, account));
    let console_key = configs.iter().find_map(console_key);
    let settings_key = settings.as_ref().and_then(settings_key);

    [claude_login, console_key, settings_key]
        .into_iter()
        .flatten()
        .collect()
}

/// The Claude login of the account `account` of the profile in `folder`,
/// with the plan of its login file; its key is the token of the login
/// file.
///
/// A login to a Console organization with no plan pays per token, and its
/// bill is the organization's, so it is known by its organization. Any
/// other login is a subscription, known by its own id, else its email. A
/// login whose files name neither can not be told from the others, so it
/// is none.
fn claude_login(folder: &str, account: &OauthAccount) -> Option<Login> {
    let credentials = read_credentials(folder);
    let plan = credentials
        .as_ref()
        .and_then(|credentials| credentials.fields().login())
        .and_then(OauthLogin::plan);
    let is_paid_per_token = account.is_console() && plan.is_none();

    let identity = if is_paid_per_token && account.organization_id().is_some() {
        Identity {
            organization_id: account.organization_id().map(String::from),
            email: account.email().map(String::from),
            ..Identity::default()
        }
    } else {
        Identity {
            account_id: account.account_id().map(String::from),
            email: account.email().map(String::from),
            ..Identity::default()
        }
    };
    if identity.account_id.is_none()
        && identity.organization_id.is_none()
        && identity.email.is_none()
    {
        return None;
    }

    let kind = if is_paid_per_token {
        LoginKind::ApiKey
    } else {
        LoginKind::Subscription
    };

    Some(Login {
        kind,
        for_whom: ProviderRef::Id(ANTHROPIC.to_string()),
        place: json_field(&credentials_path(folder), TOKEN_FIELD),
        identity,
        plan,
    })
}

/// The Console key of the global config file `config`, when it holds one.
fn console_key(config: &KeyFile<GlobalConfig>) -> Option<Login> {
    if !config.holds_key(config.fields().primary_api_key()) {
        return None;
    }

    Some(key_login(
        ProviderRef::Id(ANTHROPIC.to_string()),
        json_field(config.path(), PRIMARY_API_KEY_FIELD),
    ))
}

/// The key the settings give Claude Code in their variables, for the
/// server they send it to: `ANTHROPIC_AUTH_TOKEN`, which Claude Code sends
/// first, else `ANTHROPIC_API_KEY`. None for a server that is not an
/// address of the web.
fn settings_key(settings: &KeyFile<Settings>) -> Option<Login> {
    let fields = settings.fields();
    let provider = provider_of(Some(fields))?;

    let pointer = if settings.holds_key(fields.auth_token()) {
        AUTH_TOKEN_FIELD
    } else if settings.holds_key(fields.api_key()) {
        API_KEY_FIELD
    } else {
        return None;
    };

    Some(key_login(provider, json_field(settings.path(), pointer)))
}

/// An API key for `provider` at `place`; the files tell nothing else
/// about it.
fn key_login(provider: ProviderRef, place: KeyPlace) -> Login {
    Login {
        kind: LoginKind::ApiKey,
        for_whom: provider,
        place,
        identity: Identity::default(),
        plan: None,
    }
}

/// The field `pointer` of the JSON file `file` as a key place.
fn json_field(file: &str, pointer: &str) -> KeyPlace {
    KeyPlace::JsonField(JsonField {
        file: file.to_string(),
        pointer: pointer.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::keys;

    use super::*;
    use crate::testing::{as_part, host, path};

    fn logins_of(folder: &str) -> Vec<Login> {
        let host = host();
        let folder = path(&host, folder);

        as_part(&host, || logins(&folder))
    }

    fn anthropic() -> ProviderRef {
        ProviderRef::Id(ANTHROPIC.to_string())
    }

    fn place(folder: &str, file: &str, pointer: &str) -> KeyPlace {
        let host = host();

        json_field(&path(&host, &format!("{folder}/{file}")), pointer)
    }

    #[test]
    fn a_subscription_login_is_a_claude_account_with_its_plan() {
        let found = logins_of(".claude-work");

        assert_eq!(
            found,
            [Login {
                kind: LoginKind::Subscription,
                for_whom: anthropic(),
                place: place(".claude-work", ".credentials.json", TOKEN_FIELD),
                identity: Identity {
                    account_id: Some(
                        "00000000-0000-4000-8000-00000000000c".to_string()
                    ),
                    email: Some("own@example.com".to_string()),
                    ..Identity::default()
                },
                plan: Some("Max 20x".to_string()),
            }]
        );
    }

    #[test]
    fn the_default_folder_finds_the_login_next_to_it() {
        let found = logins_of(".claude");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].plan.as_deref(), Some("Max 20x"));
        assert_eq!(found[0].identity.email.as_deref(), Some("dev@example.com"));
    }

    /// The bill of a Console organization belongs to the organization, so
    /// two people logged in to it are one account.
    #[test]
    fn a_prepaid_login_is_its_organization_paid_per_token() {
        let found = logins_of(".claude-prepaid");

        assert_eq!(
            found,
            [Login {
                kind: LoginKind::ApiKey,
                for_whom: anthropic(),
                place: place(
                    ".claude-prepaid",
                    ".credentials.json",
                    TOKEN_FIELD
                ),
                identity: Identity {
                    organization_id: Some(
                        "00000000-0000-4000-8000-0000000000dd".to_string()
                    ),
                    email: Some("dev@example.com".to_string()),
                    ..Identity::default()
                },
                plan: None,
            }]
        );
    }

    #[test]
    fn a_login_with_no_id_is_told_by_its_email() {
        let found = logins_of(".claude-email");

        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].identity,
            Identity {
                email: Some("dev@example.com".to_string()),
                ..Identity::default()
            }
        );
    }

    #[test]
    fn a_token_with_no_account_in_the_files_is_no_login() {
        assert_eq!(logins_of(".claude-token"), []);
        assert_eq!(logins_of(".claude-nothing"), []);
    }

    #[test]
    fn a_primary_api_key_is_an_anthropic_key() {
        assert_eq!(
            logins_of(".claude-console"),
            [key_login(
                anthropic(),
                place(".claude-console", ".claude.json", PRIMARY_API_KEY_FIELD)
            )]
        );
    }

    #[test]
    fn an_api_key_in_the_settings_is_an_anthropic_key() {
        assert_eq!(
            logins_of(".claude-api"),
            [key_login(
                anthropic(),
                place(".claude-api", "settings.json", API_KEY_FIELD)
            )]
        );
    }

    /// The issue's case: a DeepSeek key behind DeepSeek's server is a
    /// login for `api.deepseek.com`, not for Anthropic.
    #[test]
    fn a_token_for_a_deepseek_base_url_is_a_key_for_its_host() {
        assert_eq!(
            logins_of(".claude-deepseek"),
            [key_login(
                ProviderRef::Host("api.deepseek.com".to_string()),
                place(".claude-deepseek", "settings.json", AUTH_TOKEN_FIELD)
            )]
        );
    }

    /// Claude Code sends the auth token before the API key, so the token
    /// is the key in use; a key for any other server is a key of its host.
    #[test]
    fn the_auth_token_comes_before_the_api_key() {
        assert_eq!(
            logins_of(".claude-gateway"),
            [key_login(
                ProviderRef::Host("gateway.example.com".to_string()),
                place(".claude-gateway", "settings.json", AUTH_TOKEN_FIELD)
            )]
        );
    }

    #[test]
    fn a_key_for_a_server_that_is_no_web_address_is_none() {
        assert_eq!(logins_of(".claude-local"), []);
    }

    #[test]
    fn empty_null_and_blank_keys_are_none() {
        assert_eq!(logins_of(".claude-nullkey"), []);
        assert_eq!(logins_of(".claude-blank"), []);
    }

    #[test]
    fn a_login_and_a_key_in_one_profile_are_two_logins() {
        let kinds: Vec<LoginKind> = logins_of(".claude-mixed")
            .iter()
            .map(|login| login.kind)
            .collect();

        assert_eq!(kinds, [LoginKind::Subscription, LoginKind::ApiKey]);
    }

    #[test]
    fn no_key_or_token_reaches_the_plugin() {
        let host = host();
        let folders = [
            ".claude",
            ".claude-mixed",
            ".claude-console",
            ".claude-gateway",
        ];

        let found: Vec<Login> = folders
            .iter()
            .flat_map(|folder| {
                let folder = path(&host, folder);
                as_part(&host, || logins(&folder))
            })
            .collect();

        let text = format!("{found:?}");
        for secret in [
            "made-up-access-token",
            "made-up-refresh-token",
            "sk-ant-made-up",
            "made-up-gateway-token",
        ] {
            assert!(!text.contains(secret), "{secret} in {text}");
        }
    }

    #[test]
    fn the_key_of_a_login_opens_as_a_handle() {
        let host = host();
        let folder = path(&host, ".claude-deepseek");
        let found = as_part(&host, || logins(&folder));

        let fingerprint =
            as_part(&host, || keys::open(&found[0].place)?.fingerprint());

        assert_eq!(fingerprint.map(|text| text.len()), Ok(64));
    }
}
