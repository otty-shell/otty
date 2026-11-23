//! The config files of a Claude Code profile that tell about its logins
//! and keys: the login file `.credentials.json`, the global config file
//! `.claude.json` and `settings.json`. Each one holds keys, so the host
//! gives it with every key replaced by a handle; the plugin learns only
//! whether a key is there.

use std::path::Path;

use otty_plugin_sdk::{JsonWithKeys, Key, files};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::plan::plan_name;

/// The billing types of `oauthAccount` in `.claude.json` that mean an
/// Anthropic Console organization, paid per token.
const CONSOLE_BILLING_TYPES: [&str; 1] = ["prepaid"];

/// The name of the default config folder, whose global config file lies
/// next to it.
const DEFAULT_FOLDER_NAME: &str = ".claude";

/// Where the host put a key: `{"$key": n}`, the index of its handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyMarker(usize);

/// A key file read through the host: the fields the plugin reads and the
/// handles of its keys.
#[derive(Debug)]
pub(crate) struct KeyFile<T> {
    fields: T,
    document: JsonWithKeys,
}

impl<T> KeyFile<T> {
    /// The fields the plugin reads.
    pub(crate) fn fields(&self) -> &T {
        &self.fields
    }

    /// Whether `marker` stands for a key that is there. The host finds no
    /// key in a blank field, so a blank key is none.
    pub(crate) fn holds_key(&self, marker: Option<KeyMarker>) -> bool {
        let Some(KeyMarker(index)) = marker else {
            return false;
        };

        self.document
            .key(index)
            .is_some_and(|key| key.fingerprint().is_ok())
    }

    /// The key `marker` stands for, when it is there; the rest of the
    /// file is dropped.
    pub(crate) fn into_key(self, marker: Option<KeyMarker>) -> Option<Key> {
        if !self.holds_key(marker) {
            return None;
        }
        let KeyMarker(index) = marker?;

        let (_, keys) = self.document.into_parts();
        keys.into_iter().nth(index)
    }
}

impl<T: DeserializeOwned> KeyFile<T> {
    /// The key file at `path`; `None` when it is missing, broken, of
    /// another shape or not allowed.
    pub(crate) fn read(path: &str) -> Option<Self> {
        let document = files::json_with_keys(path).ok()?;
        let fields = serde_json::from_str(document.json()).ok()?;

        Some(Self { fields, document })
    }
}

/// The parts of `.credentials.json` that tell about the login.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Credentials {
    claude_ai_oauth: Option<OauthLogin>,
}

impl Credentials {
    /// The login of a Claude account, when the file holds one.
    pub(crate) fn login(&self) -> Option<&OauthLogin> {
        self.claude_ai_oauth.as_ref()
    }
}

/// The login of a Claude account. Its token is never read: it is a handle.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OauthLogin {
    #[serde(default, deserialize_with = "key_marker")]
    access_token: Option<KeyMarker>,
    #[serde(default, deserialize_with = "key_marker")]
    refresh_token: Option<KeyMarker>,
    /// When the token expires, in milliseconds since 1970.
    expires_at: Option<Value>,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
}

impl OauthLogin {
    /// Where the token is, when the file names one.
    pub(crate) fn token(&self) -> Option<KeyMarker> {
        self.access_token
    }

    /// Where the refresh token is, which Claude Code renews the token
    /// with, when the file names one.
    pub(crate) fn refresh_token(&self) -> Option<KeyMarker> {
        self.refresh_token
    }

    /// When the token expires, in milliseconds since 1970, when the file
    /// tells.
    pub(crate) fn expires_at_ms(&self) -> Option<i64> {
        let expires_at = self.expires_at.as_ref()?;

        expires_at
            .as_i64()
            .or_else(|| expires_at.as_f64().map(|millis| millis as i64))
    }

    /// The plan as Claude Code names it, such as "Pro" or "Max 20x".
    pub(crate) fn plan(&self) -> Option<String> {
        plan_name(
            self.subscription_type.as_deref(),
            self.rate_limit_tier.as_deref(),
        )
    }
}

/// The parts of the global config file, `.claude.json`, that tell about
/// the login.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GlobalConfig {
    oauth_account: Option<OauthAccount>,
    /// The key of a login with an Anthropic Console API key.
    #[serde(default, deserialize_with = "key_marker")]
    primary_api_key: Option<KeyMarker>,
}

impl GlobalConfig {
    /// The account the profile is logged in with, when the file names one.
    pub(crate) fn oauth_account(&self) -> Option<&OauthAccount> {
        self.oauth_account.as_ref()
    }

    /// Where the Console API key is, when the file names one.
    pub(crate) fn primary_api_key(&self) -> Option<KeyMarker> {
        self.primary_api_key
    }
}

/// The account a profile is logged in with.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OauthAccount {
    account_uuid: Option<String>,
    organization_uuid: Option<String>,
    organization_name: Option<String>,
    email_address: Option<String>,
    /// How the organization of the login pays, such as
    /// `stripe_subscription` for a Claude plan or `prepaid` for an
    /// Anthropic Console organization.
    billing_type: Option<String>,
}

impl OauthAccount {
    /// The id of the login, when the file names one.
    pub(crate) fn account_id(&self) -> Option<&str> {
        filled(self.account_uuid.as_deref())
    }

    /// The id of the organization of the login, when the file names one.
    pub(crate) fn organization_id(&self) -> Option<&str> {
        filled(self.organization_uuid.as_deref())
    }

    /// The name of the organization of the login, when the file names
    /// one.
    pub(crate) fn organization_name(&self) -> Option<&str> {
        filled(self.organization_name.as_deref())
    }

    /// The email of the login, when the file names one.
    pub(crate) fn email(&self) -> Option<&str> {
        filled(self.email_address.as_deref())
    }

    /// Whether the account belongs to an Anthropic Console organization,
    /// which pays for the tokens it uses rather than for a plan.
    pub(crate) fn is_console(&self) -> bool {
        self.billing_type.as_deref().is_some_and(|billing| {
            CONSOLE_BILLING_TYPES.contains(&billing.trim())
        })
    }
}

/// The parts of `settings.json` that tell about a key and the server.
#[derive(Debug, Deserialize)]
pub(crate) struct Settings {
    env: Option<SettingsEnv>,
    /// A command that prints the API key. Only checked for being there.
    #[serde(rename = "apiKeyHelper")]
    api_key_helper: Option<Value>,
}

impl Settings {
    /// Whether the settings name a command that prints the API key.
    pub(crate) fn has_api_key_helper(&self) -> bool {
        let helper = self.api_key_helper.as_ref().and_then(Value::as_str);

        filled(helper).is_some()
    }

    /// Where `ANTHROPIC_API_KEY` of the settings' variables is.
    pub(crate) fn api_key(&self) -> Option<KeyMarker> {
        self.env.as_ref()?.api_key
    }

    /// Where `ANTHROPIC_AUTH_TOKEN` of the settings' variables is; Claude
    /// Code sends it before `ANTHROPIC_API_KEY`.
    pub(crate) fn auth_token(&self) -> Option<KeyMarker> {
        self.env.as_ref()?.auth_token
    }

    /// The server Claude Code is sent to, `ANTHROPIC_BASE_URL`, when it is
    /// set.
    pub(crate) fn base_url(&self) -> Option<&str> {
        filled(self.env.as_ref()?.base_url.as_deref())
    }
}

/// The variables the settings set for Claude Code.
#[derive(Debug, Deserialize)]
struct SettingsEnv {
    #[serde(rename = "ANTHROPIC_BASE_URL")]
    base_url: Option<String>,
    #[serde(
        rename = "ANTHROPIC_AUTH_TOKEN",
        default,
        deserialize_with = "key_marker"
    )]
    auth_token: Option<KeyMarker>,
    #[serde(
        rename = "ANTHROPIC_API_KEY",
        default,
        deserialize_with = "key_marker"
    )]
    api_key: Option<KeyMarker>,
}

/// The login file of the profile in `folder`.
pub(crate) fn read_credentials(folder: &str) -> Option<KeyFile<Credentials>> {
    KeyFile::read(&format!("{folder}/.credentials.json"))
}

/// The global config files of the profile in `folder` that can be read,
/// in the order to look.
pub(crate) fn read_global_configs(folder: &str) -> Vec<KeyFile<GlobalConfig>> {
    global_config_paths(folder)
        .into_iter()
        .filter_map(|path| KeyFile::read(&path))
        .collect()
}

/// The settings of the profile in `folder`; `None` when the file is
/// missing or broken.
pub(crate) fn read_settings(folder: &str) -> Option<KeyFile<Settings>> {
    KeyFile::read(&format!("{folder}/settings.json"))
}

/// Where the global config file of the profile in `folder` can be, in the
/// order to look.
///
/// With `CLAUDE_CONFIG_DIR` set, Claude Code keeps the file inside the
/// config folder. Without it, the config folder is `~/.claude` and the
/// file is `~/.claude.json`, next to the folder.
fn global_config_paths(folder: &str) -> Vec<String> {
    let mut paths = vec![format!("{folder}/.claude.json")];

    let path = Path::new(folder);
    let is_default_folder = path
        .file_name()
        .is_some_and(|name| name == DEFAULT_FOLDER_NAME);
    if let Some(parent) = path.parent()
        && is_default_folder
    {
        paths.push(format!("{}/.claude.json", parent.display()));
    }

    paths
}

/// `text` when it is not blank.
fn filled(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.trim().is_empty())
}

/// The marker of a key field: `{"$key": n}` where the host put a key;
/// anything else, such as `null`, is no key.
fn key_marker<'de, D>(deserializer: D) -> Result<Option<KeyMarker>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    let index = value
        .get("$key")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok());

    Ok(index.map(KeyMarker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    #[test]
    fn the_global_config_file_in_the_folder_comes_before_the_one_next_to_it() {
        assert_eq!(
            global_config_paths("/home/dev/.claude"),
            ["/home/dev/.claude/.claude.json", "/home/dev/.claude.json"]
        );
    }

    #[test]
    fn another_folder_looks_only_inside_itself() {
        assert_eq!(
            global_config_paths("/home/dev/.claude-work"),
            ["/home/dev/.claude-work/.claude.json"]
        );
    }

    #[test]
    fn the_default_folder_finds_the_global_config_file_next_to_it() {
        let host = host();
        let folder = path(&host, ".claude");

        let configs = as_part(&host, || read_global_configs(&folder));

        // `.claude` holds no `.claude.json`: the one read is next to it.
        assert_eq!(configs.len(), 1);
        let account = configs[0].fields().oauth_account();
        assert_eq!(
            account.and_then(OauthAccount::email),
            Some("dev@example.com")
        );
    }

    #[test]
    fn a_key_is_a_handle_and_a_blank_or_null_one_is_none() {
        let host = host();
        let api = path(&host, ".claude-api");
        let blank = path(&host, ".claude-blank");
        let null = path(&host, ".claude-nullkey");

        let found = as_part(&host, || {
            let held = |folder: &str| {
                let settings = read_settings(folder).expect("the settings");
                let marker = settings.fields().api_key();

                (marker.is_some(), settings.holds_key(marker))
            };

            [held(&api), held(&blank), held(&null)]
        });

        assert_eq!(found, [(true, true), (true, false), (true, false)]);
    }

    #[test]
    fn the_token_is_never_in_the_text_the_plugin_reads() {
        let host = host();
        let folder = path(&host, ".claude");

        let credentials = as_part(&host, || read_credentials(&folder))
            .expect("the login file");

        let text = format!("{credentials:?}");
        assert!(!text.contains("made-up-access-token"), "{text}");
        assert!(!text.contains("made-up-refresh-token"), "{text}");
        let login = credentials.fields().login().expect("the login");
        assert_eq!(login.plan().as_deref(), Some("Max 20x"));
    }

    #[test]
    fn a_broken_file_is_none() {
        let host = host();
        let folder = path(&host, ".claude-broken");

        let (credentials, configs) = as_part(&host, || {
            (read_credentials(&folder), read_global_configs(&folder))
        });

        assert!(credentials.is_none());
        assert!(configs.is_empty());
    }
}
