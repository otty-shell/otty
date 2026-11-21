//! Whom the tokens and the keys of a Claude Code profile are for. Claude
//! Code does not write it: it sends everything to the server of
//! `ANTHROPIC_BASE_URL` in its settings when it is set, else to
//! Anthropic's, so a DeepSeek key behind
//! `https://api.deepseek.com/anthropic` is a key for `api.deepseek.com`.

use otty_plugin_sdk::agents::ProviderRef;

use crate::config_files::{Settings, read_settings};

/// The provider id of Anthropic.
pub(crate) const ANTHROPIC: &str = "anthropic";

/// The provider the server of `settings` is: the host of
/// `ANTHROPIC_BASE_URL`, else Anthropic. `None` for a base URL that is not
/// an address of the web.
pub(crate) fn provider_of(settings: Option<&Settings>) -> Option<ProviderRef> {
    match settings.and_then(Settings::base_url) {
        Some(base_url) => host_of(base_url).map(ProviderRef::Host),
        None => Some(ProviderRef::Id(ANTHROPIC.to_string())),
    }
}

/// The provider the tokens of the profile in `folder` are counted under.
pub(crate) fn profile_provider(folder: &str) -> Option<ProviderRef> {
    let settings = read_settings(folder);

    provider_of(settings.as_ref().map(|settings| settings.fields()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    #[test]
    fn a_base_url_names_the_host_of_its_server_in_small_letters() {
        let cases = [
            ("https://api.anthropic.com", "api.anthropic.com"),
            ("https://api.deepseek.com/anthropic", "api.deepseek.com"),
            ("https://openrouter.ai/api", "openrouter.ai"),
            ("http://localhost:4000", "localhost"),
            (" HTTPS://API.DeepSeek.com ", "api.deepseek.com"),
        ];

        for (base_url, host) in cases {
            assert_eq!(host_of(base_url).as_deref(), Some(host), "{base_url}");
        }
    }

    #[test]
    fn what_is_not_an_address_of_the_web_names_no_server() {
        for base_url in ["not a url", "", "https://", "ftp://example.com"] {
            assert_eq!(host_of(base_url), None, "{base_url}");
        }
    }

    #[test]
    fn tokens_are_counted_under_the_server_of_the_settings() {
        let host = host();
        let cases = [
            (".claude", Some(ProviderRef::Id(ANTHROPIC.to_string()))),
            (
                ".claude-token",
                Some(ProviderRef::Id(ANTHROPIC.to_string())),
            ),
            (".claude-gone", Some(ProviderRef::Id(ANTHROPIC.to_string()))),
            (
                ".claude-deepseek",
                Some(ProviderRef::Host("api.deepseek.com".to_string())),
            ),
            (".claude-local", None),
        ];

        for (folder, provider) in cases {
            let folder = path(&host, folder);

            let found = as_part(&host, || profile_provider(&folder));

            assert_eq!(found, provider, "{folder}");
        }
    }
}
