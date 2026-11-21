//! The shape of `otty-plugin.toml` as `toml` reads it, before any check.
//! Unknown fields are refused, so a misspelt permission is never dropped
//! silently.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The whole manifest.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawManifest {
    pub(crate) plugin: RawPlugin,
    #[serde(default)]
    pub(crate) api: BTreeMap<String, RawApiEntry>,
    #[serde(default)]
    pub(crate) agents: RawAgentsDomain,
}

/// One entry of `[api]`: the version of a contract of no domain, such as
/// `language = "0.1"`, or the versions of the points of one domain, such
/// as `agents.agent = "0.1"`, which TOML reads as the table `agents`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawApiEntry {
    Version(String),
    Domain(BTreeMap<String, String>),
}

/// The parts of the `agents` domain: `[[agents.agent]]` and
/// `[[agents.provider]]`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAgentsDomain {
    #[serde(default)]
    pub(crate) agent: Vec<RawAgent>,
    #[serde(default)]
    pub(crate) provider: Vec<RawProvider>,
}

/// `[plugin]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPlugin {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) description: Option<String>,
    pub(crate) repository: Option<String>,
    pub(crate) license: Option<String>,
    #[serde(default)]
    pub(crate) authors: Vec<String>,
    pub(crate) wasm: String,
}

/// One `[[agents.agent]]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAgent {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) icon: Option<String>,
    pub(crate) program: Option<String>,
    pub(crate) tested_version: Option<String>,
    #[serde(default)]
    pub(crate) permissions: RawAgentPermissions,
}

/// `[agents.agent.permissions]`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAgentPermissions {
    #[serde(default)]
    pub(crate) read: Vec<String>,
    #[serde(default)]
    pub(crate) write: Vec<String>,
    #[serde(default)]
    pub(crate) sqlite: Vec<String>,
    #[serde(default)]
    pub(crate) env: Vec<String>,
    #[serde(default)]
    pub(crate) run: Vec<RawProgram>,
    #[serde(default)]
    pub(crate) terminal: Vec<RawProgram>,
    #[serde(default)]
    pub(crate) keyring: Vec<RawKeyring>,
    #[serde(default)]
    pub(crate) keys: Vec<RawKeyPlace>,
}

/// One `[[agents.agent.permissions.run]]` or
/// `[[agents.agent.permissions.terminal]]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawProgram {
    pub(crate) program: String,
    #[serde(default)]
    pub(crate) args: Vec<String>,
}

/// One `[[agents.agent.permissions.keyring]]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawKeyring {
    pub(crate) service: String,
    pub(crate) user: String,
    #[serde(default)]
    pub(crate) fields: Vec<String>,
}

/// One `[[agents.agent.permissions.keys]]`: JSON fields of files (`files`), a
/// column of a SQLite table (`sqlite`, `table`, `column`) or environment
/// variables (`env`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawKeyPlace {
    pub(crate) files: Option<Vec<String>>,
    pub(crate) sqlite: Option<Vec<String>>,
    pub(crate) table: Option<String>,
    pub(crate) column: Option<String>,
    pub(crate) fields: Option<Vec<String>>,
    pub(crate) env: Option<Vec<String>>,
}

/// One `[[agents.provider]]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawProvider {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) icon: Option<String>,
    #[serde(default)]
    pub(crate) hosts: Vec<String>,
    pub(crate) usage_page: Option<String>,
    pub(crate) plan_usage_page: Option<String>,
    #[serde(default)]
    pub(crate) add_by_hand: bool,
    #[serde(default)]
    pub(crate) permissions: RawProviderPermissions,
}

/// `[agents.provider.permissions]`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawProviderPermissions {
    #[serde(default)]
    pub(crate) http: Vec<String>,
}
