//! The addresses of the Codex usage request and of the requests about the
//! workspace.
//!
//! It is `https://chatgpt.com/backend-api/wham/usage`, unless
//! `chatgpt_base_url` in `<config>/config.toml` names another server. The
//! rules follow CodexBar (`resolveUsageURL` in
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift>,
//! MIT licence): a ChatGPT address gets `/backend-api`, and a server
//! without `/backend-api` is asked at `/api/codex/usage`. The requests
//! about the workspace go only to a server with `/backend-api`
//! (`resolveSpendControlsMonthlyUsageURL` there).

use std::path::Path;

/// The server Codex talks to when its config names none.
const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api";
/// The path of the usage on a ChatGPT server.
const CHATGPT_USAGE_PATH: &str = "/wham/usage";
/// The path of the usage on another server.
const CODEX_USAGE_PATH: &str = "/api/codex/usage";
/// The key of the server in `config.toml`.
const BASE_URL_KEY: &str = "chatgpt_base_url";

/// The address of the usage request on ChatGPT itself, for a login another
/// agent keeps, such as Kilo, whose server is always ChatGPT.
pub(super) fn chatgpt_usage_url() -> String {
    format!("{DEFAULT_BASE_URL}{CHATGPT_USAGE_PATH}")
}

/// The address of the usage request of the profile in `config_dir`.
pub(super) fn usage_url(config_dir: &Path) -> String {
    let base_url = base_url(config_dir);
    let path = if base_url.contains("/backend-api") {
        CHATGPT_USAGE_PATH
    } else {
        CODEX_USAGE_PATH
    };

    format!("{base_url}{path}")
}

/// The address of the ChatGPT backend of the profile in `config_dir`,
/// such as `https://chatgpt.com/backend-api`, for the requests about its
/// workspace; `None` for a server without `/backend-api`, which has no
/// such requests (as in CodexBar).
pub(super) fn backend_api_url(config_dir: &Path) -> Option<String> {
    let base_url = base_url(config_dir);

    base_url.contains("/backend-api").then_some(base_url)
}

/// The server of the profile in `config_dir`, from `chatgpt_base_url` in
/// its `config.toml`, else ChatGPT; without a trailing `/`.
fn base_url(config_dir: &Path) -> String {
    let configured = std::fs::read_to_string(config_dir.join("config.toml"))
        .ok()
        .and_then(|config| configured_base_url(&config));

    normalized(configured.as_deref().unwrap_or(DEFAULT_BASE_URL))
}

/// The value of `chatgpt_base_url` among the top-level keys of
/// `config.toml`. The file is read line by line, as CodexBar does, so no
/// TOML reader is needed; a key inside a `[table]` is not the server.
fn configured_base_url(config: &str) -> Option<String> {
    for line in config.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') {
            return None;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != BASE_URL_KEY {
            continue;
        }

        let value = value.trim();
        let unquoted = value
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|rest| rest.strip_suffix('\''))
            })
            .unwrap_or(value)
            .trim();

        return (!unquoted.is_empty()).then(|| unquoted.to_string());
    }

    None
}

/// The server without a trailing `/`; a ChatGPT address gets
/// `/backend-api`.
fn normalized(base_url: &str) -> String {
    let mut base_url = base_url.trim().trim_end_matches('/').to_string();

    let is_chatgpt = base_url.starts_with("https://chatgpt.com")
        || base_url.starts_with("https://chat.openai.com");
    if is_chatgpt && !base_url.contains("/backend-api") {
        base_url.push_str("/backend-api");
    }

    base_url
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, write_file};

    fn url_with_config(config: &str) -> String {
        let temp = TempDir::new();
        write_file(&temp.path().join("config.toml"), config);

        usage_url(temp.path())
    }

    #[test]
    fn with_no_config_file_the_usage_is_asked_of_chatgpt() {
        let temp = TempDir::new();

        assert_eq!(
            usage_url(temp.path()),
            "https://chatgpt.com/backend-api/wham/usage"
        );
    }

    #[test]
    fn the_base_url_of_the_config_file_replaces_the_server() {
        let config = r#"
            model = "gpt-5-codex"
            # chatgpt_base_url = "https://commented.example.com"
            chatgpt_base_url = "https://gateway.example.com/backend-api/" # a team gateway
        "#;

        assert_eq!(
            url_with_config(config),
            "https://gateway.example.com/backend-api/wham/usage"
        );
    }

    #[test]
    fn a_chatgpt_address_gets_backend_api() {
        assert_eq!(
            url_with_config("chatgpt_base_url = 'https://chatgpt.com/'"),
            "https://chatgpt.com/backend-api/wham/usage"
        );
        assert_eq!(
            url_with_config("chatgpt_base_url = \"https://chat.openai.com\""),
            "https://chat.openai.com/backend-api/wham/usage"
        );
    }

    #[test]
    fn a_server_without_backend_api_is_asked_at_the_codex_path() {
        assert_eq!(
            url_with_config(
                "chatgpt_base_url = \"https://proxy.example.com/codex\""
            ),
            "https://proxy.example.com/codex/api/codex/usage"
        );
    }

    #[test]
    fn the_workspace_requests_go_to_the_chatgpt_backend() {
        let temp = TempDir::new();
        let custom = TempDir::new();
        write_file(
            &custom.path().join("config.toml"),
            "chatgpt_base_url = \"https://gateway.example.com/backend-api/\"",
        );
        let chatgpt = TempDir::new();
        write_file(
            &chatgpt.path().join("config.toml"),
            "chatgpt_base_url = \"https://chatgpt.com\"",
        );

        assert_eq!(
            backend_api_url(temp.path()).as_deref(),
            Some("https://chatgpt.com/backend-api")
        );
        assert_eq!(
            backend_api_url(custom.path()).as_deref(),
            Some("https://gateway.example.com/backend-api")
        );
        assert_eq!(
            backend_api_url(chatgpt.path()).as_deref(),
            Some("https://chatgpt.com/backend-api")
        );
    }

    #[test]
    fn a_server_without_backend_api_has_no_workspace_requests() {
        let temp = TempDir::new();
        write_file(
            &temp.path().join("config.toml"),
            "chatgpt_base_url = \"https://proxy.example.com/codex\"",
        );

        assert_eq!(backend_api_url(temp.path()), None);
    }

    #[test]
    fn a_key_inside_a_table_or_an_empty_one_is_not_the_base_url() {
        let in_table =
            "[profiles.work]\nchatgpt_base_url = \"https://work.example.com\"";

        assert_eq!(
            url_with_config(in_table),
            "https://chatgpt.com/backend-api/wham/usage"
        );
        assert_eq!(
            url_with_config("chatgpt_base_url = \"\""),
            "https://chatgpt.com/backend-api/wham/usage"
        );
    }
}
