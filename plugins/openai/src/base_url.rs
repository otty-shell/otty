//! Whom the logins of a Codex profile are for. Codex sends the token of a
//! ChatGPT login to `chatgpt_base_url` of `<config>/config.toml` when it is
//! set, else to ChatGPT, and the API key of `auth.json` to the server of
//! the provider `model_provider` names when that provider takes the
//! OpenAI login (`requires_openai_auth` in its `[model_providers.<id>]`
//! table), else to OpenAI. A key sent to another server is a key for that
//! server's host.
//!
//! The file is read line by line, as CodexBar reads `chatgpt_base_url`
//! (`resolveUsageURL` in
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift>,
//! MIT licence), so no TOML reader is needed: only top-level keys and the
//! keys of the `[model_providers.<id>]` tables count.

use std::collections::BTreeMap;

use otty_plugin_sdk::agents::ProviderRef;
use otty_plugin_sdk::files;

/// The name of the config file in the config folder.
const CONFIG_FILE: &str = "config.toml";

/// The longest config file read, in bytes.
const MAX_CONFIG_BYTES: u32 = 1024 * 1024;

/// The id of OpenAI, Codex's own provider.
pub(crate) const OPENAI: &str = "openai";

/// The hosts that are OpenAI's own: a host is one of them or ends with
/// `.` and one of them.
const OPENAI_HOSTS: [&str; 2] = ["openai.com", "chatgpt.com"];

/// The table prefix of a provider of the user's own config.
const PROVIDER_TABLE: &str = "model_providers.";

/// A provider of the user's own config.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ProviderTable {
    base_url: Option<String>,
    requires_openai_auth: bool,
}

/// What `config.toml` tells about where the logins go.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct CodexConfig {
    chatgpt_base_url: Option<String>,
    model_provider: Option<String>,
    providers: BTreeMap<String, ProviderTable>,
}

impl CodexConfig {
    /// The config of the profile in `folder`; empty when the file is
    /// missing or can not be read.
    pub(crate) fn read(folder: &str) -> Self {
        let path = format!("{folder}/{CONFIG_FILE}");
        let bytes = files::read(&path, MAX_CONFIG_BYTES).unwrap_or_default();

        Self::parse(&String::from_utf8_lossy(&bytes))
    }

    /// Whom the token of a ChatGPT login is for: OpenAI, or the host of
    /// `chatgpt_base_url` when it names another server.
    pub(crate) fn chatgpt_login_for(&self) -> ProviderRef {
        provider_of(self.chatgpt_base_url.as_deref())
    }

    /// Whom the API key of `auth.json` is for: OpenAI, or the host of the
    /// provider `model_provider` names when it takes the OpenAI login.
    pub(crate) fn api_key_for(&self) -> ProviderRef {
        let provider = self
            .model_provider
            .as_deref()
            .and_then(|id| self.providers.get(id))
            .filter(|provider| provider.requires_openai_auth);

        provider_of(provider.and_then(|provider| provider.base_url.as_deref()))
    }

    /// The keys of `config` this module reads.
    fn parse(config: &str) -> Self {
        let mut read = Self::default();
        let mut table: Option<String> = None;

        for line in config.lines() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if let Some(name) = table_name(line) {
                table = Some(name);
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let key = key.trim();
            let value = value.trim();
            match table.as_deref() {
                None => read.add_top_level(key, value),
                Some(name) => {
                    if let Some(id) = name.strip_prefix(PROVIDER_TABLE) {
                        read.add_provider_key(&unquoted(id), key, value);
                    }
                },
            }
        }

        read
    }

    fn add_top_level(&mut self, key: &str, value: &str) {
        match key {
            "chatgpt_base_url" => self.chatgpt_base_url = text_value(value),
            "model_provider" => self.model_provider = text_value(value),
            _ => {},
        }
    }

    fn add_provider_key(&mut self, id: &str, key: &str, value: &str) {
        let provider = self.providers.entry(id.to_string()).or_default();

        match key {
            "base_url" => provider.base_url = text_value(value),
            "requires_openai_auth" => {
                provider.requires_openai_auth = value == "true";
            },
            _ => {},
        }
    }
}

/// The provider a server at `base_url` is: OpenAI with no server, for an
/// address of OpenAI and for one that is no address of the web; else the
/// host of the server, in small letters.
fn provider_of(base_url: Option<&str>) -> ProviderRef {
    let host = base_url.and_then(host_of);

    match host {
        Some(host) if !is_openai_host(&host) => ProviderRef::Host(host),
        _ => ProviderRef::Id(OPENAI.to_string()),
    }
}

/// Whether `host` is one of OpenAI's own.
fn is_openai_host(host: &str) -> bool {
    OPENAI_HOSTS.iter().any(|own| {
        host == *own
            || host
                .strip_suffix(own)
                .is_some_and(|start| start.ends_with('.'))
    })
}

/// The host of the address `url`, in small letters, without its port;
/// `None` when `url` is not an `http` or `https` address.
fn host_of(url: &str) -> Option<String> {
    let url = url.trim().to_ascii_lowercase();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;

    let host = rest.split(['/', ':', '?', '#']).next().unwrap_or_default();

    (!host.is_empty()).then(|| host.to_string())
}

/// The name of the table `line` opens, such as `model_providers.proxy`.
fn table_name(line: &str) -> Option<String> {
    let name = line.strip_prefix('[')?.strip_suffix(']')?;

    Some(name.trim().to_string())
}

/// The text of a quoted value, when it is not blank.
fn text_value(value: &str) -> Option<String> {
    let text = unquoted(value);

    (!text.is_empty()).then_some(text)
}

/// `text` without the quotes around it, trimmed.
fn unquoted(text: &str) -> String {
    let text = text.trim();
    let inner = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            text.strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .unwrap_or(text);

    inner.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    fn openai() -> ProviderRef {
        ProviderRef::Id(OPENAI.to_string())
    }

    fn host_ref(host: &str) -> ProviderRef {
        ProviderRef::Host(host.to_string())
    }

    fn config_of(folder: &str) -> CodexConfig {
        let host = host();
        let folder = path(&host, folder);

        as_part(&host, || CodexConfig::read(&folder))
    }

    #[test]
    fn with_no_config_file_both_logins_are_for_openai() {
        let config = config_of(".codex-work");

        assert_eq!(config.chatgpt_login_for(), openai());
        assert_eq!(config.api_key_for(), openai());
    }

    #[test]
    fn the_base_url_of_the_config_file_names_the_host_of_the_chatgpt_login() {
        let config = config_of(".codex-gateway");

        assert_eq!(config.chatgpt_login_for(), host_ref("gateway.example.com"));
        assert_eq!(config.api_key_for(), openai());
    }

    #[test]
    fn a_chatgpt_address_is_openai() {
        for config in [
            "chatgpt_base_url = 'https://chatgpt.com/'",
            "chatgpt_base_url = \"https://chat.openai.com\"",
            "chatgpt_base_url = \"https://chatgpt.com/backend-api\"",
        ] {
            assert_eq!(
                CodexConfig::parse(config).chatgpt_login_for(),
                openai(),
                "{config}"
            );
        }
    }

    #[test]
    fn a_key_inside_a_table_or_an_empty_one_is_not_the_base_url() {
        let in_table =
            "[profiles.work]\nchatgpt_base_url = \"https://work.example.com\"";

        assert_eq!(CodexConfig::parse(in_table).chatgpt_login_for(), openai());
        assert_eq!(
            CodexConfig::parse("chatgpt_base_url = \"\"").chatgpt_login_for(),
            openai()
        );
    }

    #[test]
    fn a_base_url_that_is_no_web_address_is_openai() {
        let config = CodexConfig::parse("chatgpt_base_url = \"gateway\"");

        assert_eq!(config.chatgpt_login_for(), openai());
    }

    #[test]
    fn the_api_key_goes_to_the_provider_that_takes_the_openai_login() {
        let config = config_of(".codex-proxy");

        assert_eq!(config.api_key_for(), host_ref("llm.example.com"));
        assert_eq!(config.chatgpt_login_for(), openai());
    }

    #[test]
    fn a_provider_that_does_not_take_the_openai_login_leaves_the_key_to_openai()
    {
        let own_key = "model_provider = \"ollama\"\n\
                       [model_providers.ollama]\n\
                       base_url = \"http://localhost:11434/v1\"";
        let not_chosen = "model_provider = \"openai\"\n\
                          [model_providers.proxy]\n\
                          base_url = \"https://llm.example.com/v1\"\n\
                          requires_openai_auth = true";
        let quoted = "model_provider = \"team-proxy\"\n\
                      [model_providers.\"team-proxy\"]\n\
                      base_url = \"https://LLM.example.com:8443/v1\"\n\
                      requires_openai_auth = true";

        assert_eq!(CodexConfig::parse(own_key).api_key_for(), openai());
        assert_eq!(CodexConfig::parse(not_chosen).api_key_for(), openai());
        assert_eq!(
            CodexConfig::parse(quoted).api_key_for(),
            host_ref("llm.example.com")
        );
    }

    #[test]
    fn the_hosts_of_openai_are_its_own() {
        assert!(is_openai_host("openai.com"));
        assert!(is_openai_host("api.openai.com"));
        assert!(is_openai_host("chatgpt.com"));
        assert!(!is_openai_host("notopenai.com"));
        assert!(!is_openai_host("openai.com.example.com"));
    }
}
