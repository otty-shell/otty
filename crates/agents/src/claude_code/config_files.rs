//! The parts of Claude Code's config files that tell about its logins and
//! keys: the global config file `.claude.json` and `<config>/settings.json`.
//! A key in them is read as its fingerprint only and never kept.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::admin_key::AdminKey;
use crate::json_file::{filled, is_filled_text, read_json};
use crate::key_fingerprint::{SeenKey, read_seen_key};

/// The billing types of `oauthAccount` in `.claude.json` that mean an
/// Anthropic Console organization, paid per token.
const CONSOLE_BILLING_TYPES: [&str; 1] = ["prepaid"];

/// The parts of Claude Code's global config file, `.claude.json`, that tell
/// about the login.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GlobalConfig {
    oauth_account: Option<OauthAccount>,
    /// The key of a login with an Anthropic Console API key.
    #[serde(
        rename = "primaryApiKey",
        default,
        deserialize_with = "read_seen_key"
    )]
    primary_api_key: Option<SeenKey>,
}

impl GlobalConfig {
    /// The account the profile is logged in with, when the file names one.
    pub(super) fn oauth_account(&self) -> Option<&OauthAccount> {
        self.oauth_account.as_ref()
    }

    /// The Console API key of the profile, when the file holds one.
    pub(super) fn primary_api_key(&self) -> Option<&SeenKey> {
        self.primary_api_key.as_ref()
    }
}

/// The account a profile is logged in with.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OauthAccount {
    account_uuid: Option<String>,
    organization_uuid: Option<String>,
    email_address: Option<String>,
    /// How the organization of the login pays, such as
    /// `stripe_subscription` for a Claude plan or `prepaid` for an
    /// Anthropic Console organization.
    billing_type: Option<String>,
}

impl OauthAccount {
    /// The id of the login, when the file names one.
    pub(super) fn account_id(&self) -> Option<&str> {
        filled(self.account_uuid.as_deref())
    }

    /// The id of the organization of the login, when the file names one.
    pub(super) fn organization_id(&self) -> Option<&str> {
        filled(self.organization_uuid.as_deref())
    }

    /// The email of the login, when the file names one.
    pub(super) fn email(&self) -> Option<&str> {
        filled(self.email_address.as_deref())
    }

    /// Whether the account belongs to an Anthropic Console organization,
    /// which pays for the tokens it uses rather than for a plan.
    pub(super) fn is_console(&self) -> bool {
        self.billing_type.as_deref().is_some_and(|billing| {
            CONSOLE_BILLING_TYPES.contains(&billing.trim())
        })
    }
}

/// The parts of `<config>/settings.json` that tell about a key and the
/// server.
#[derive(Debug, Deserialize)]
pub(super) struct Settings {
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
    /// Whether the settings give Claude Code an API key: in
    /// `env.ANTHROPIC_API_KEY` or through `apiKeyHelper`.
    pub(super) fn has_api_key(&self) -> bool {
        let has_env_key =
            self.env.as_ref().is_some_and(|env| env.api_key.is_some());

        has_env_key || self.has_api_key_helper
    }

    /// The key the settings give Claude Code in their variables:
    /// `ANTHROPIC_AUTH_TOKEN`, which Claude Code sends first, else
    /// `ANTHROPIC_API_KEY`.
    pub(super) fn env_key(&self) -> Option<&SeenKey> {
        let env = self.env.as_ref()?;

        env.auth_token.as_ref().or(env.api_key.as_ref())
    }

    /// The server Claude Code is sent to, `ANTHROPIC_BASE_URL`, when it is
    /// set.
    pub(super) fn base_url(&self) -> Option<&str> {
        let env = self.env.as_ref()?;

        filled(env.base_url.as_deref())
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
        deserialize_with = "read_seen_key"
    )]
    auth_token: Option<SeenKey>,
    #[serde(
        rename = "ANTHROPIC_API_KEY",
        default,
        deserialize_with = "read_seen_key"
    )]
    api_key: Option<SeenKey>,
}

/// The parts of `<config>/settings.json` that hold a key, read as they are
/// written for one request.
#[derive(Deserialize)]
struct SettingsWithKey {
    env: Option<EnvWithKey>,
}

/// The variables of the settings that hold a key, as they are written.
#[derive(Deserialize)]
struct EnvWithKey {
    #[serde(rename = "ANTHROPIC_AUTH_TOKEN")]
    auth_token: Option<String>,
    #[serde(rename = "ANTHROPIC_API_KEY")]
    api_key: Option<String>,
}

/// The global config files of the profile in `config_dir` that can be
/// read, in the order to look.
pub(super) fn read_global_configs(config_dir: &Path) -> Vec<GlobalConfig> {
    global_config_files(config_dir)
        .iter()
        .filter_map(|path| read_json(path))
        .collect()
}

/// The settings of the profile in `config_dir`. `None` when the file is
/// missing or broken.
pub(super) fn read_settings(config_dir: &Path) -> Option<Settings> {
    read_json(&config_dir.join("settings.json"))
}

/// The key the settings of the profile in `config_dir` give Claude Code,
/// read again for one request: `ANTHROPIC_AUTH_TOKEN`, which Claude Code
/// sends first, else `ANTHROPIC_API_KEY`, as [`Settings::env_key`] tells
/// its fingerprint.
pub(super) fn read_settings_key(config_dir: &Path) -> Option<AdminKey> {
    let settings: SettingsWithKey =
        read_json(&config_dir.join("settings.json"))?;
    let env = settings.env?;

    let auth_token = env.auth_token.as_deref().and_then(AdminKey::of);
    auth_token.or_else(|| env.api_key.as_deref().and_then(AdminKey::of))
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
