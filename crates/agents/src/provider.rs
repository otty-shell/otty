//! Who sells the model use an agent runs on (spec "Words").

/// The domains of the servers of each provider. A server is a provider's
/// when its host is one of these or ends with `.` and one of these.
const PROVIDER_DOMAINS: [(&str, Provider); 8] = [
    ("anthropic.com", Provider::Anthropic),
    ("openai.com", Provider::OpenAI),
    ("deepseek.com", Provider::DeepSeek),
    ("openrouter.ai", Provider::OpenRouter),
    ("moonshot.ai", Provider::Moonshot),
    ("moonshot.cn", Provider::Moonshot),
    ("kilo.ai", Provider::Kilo),
    ("kilocode.ai", Provider::Kilo),
];

/// Every provider OTTY knows.
const ALL_PROVIDERS: [Provider; 6] = [
    Provider::Anthropic,
    Provider::OpenAI,
    Provider::DeepSeek,
    Provider::OpenRouter,
    Provider::Moonshot,
    Provider::Kilo,
];

/// The providers an account can be added for by hand (spec 2026-11-19,
/// Decision 7), in the order the Add account dialog lists them: the money
/// left of a DeepSeek, OpenRouter or Moonshot key, and the bill of an
/// Anthropic or OpenAI organization.
const ADDABLE_PROVIDERS: [Provider; 5] = [
    Provider::DeepSeek,
    Provider::OpenRouter,
    Provider::Moonshot,
    Provider::Anthropic,
    Provider::OpenAI,
];

/// A company that sells the use of models: an [`Account`] is a login or a
/// key at one of them. Local models have no provider.
///
/// [`Account`]: crate::Account
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Provider {
    /// Anthropic: Claude plans and Anthropic Console organizations.
    Anthropic,
    /// OpenAI: ChatGPT plans and OpenAI API keys.
    OpenAI,
    /// DeepSeek API keys.
    DeepSeek,
    /// OpenRouter keys.
    OpenRouter,
    /// Moonshot AI (Kimi) API keys.
    Moonshot,
    /// The Kilo gateway.
    Kilo,
}

impl Provider {
    /// The name shown to the user, such as "OpenRouter".
    ///
    /// ```
    /// use otty_agents::Provider;
    ///
    /// assert_eq!(Provider::DeepSeek.name(), "DeepSeek");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAI => "OpenAI",
            Self::DeepSeek => "DeepSeek",
            Self::OpenRouter => "OpenRouter",
            Self::Moonshot => "Moonshot",
            Self::Kilo => "Kilo",
        }
    }

    /// The providers an account can be added for by hand, in the order the
    /// Add account dialog lists them.
    ///
    /// ```
    /// use otty_agents::Provider;
    ///
    /// assert_eq!(Provider::addable()[0], Provider::DeepSeek);
    /// assert!(!Provider::addable().contains(&Provider::Kilo));
    /// ```
    pub fn addable() -> &'static [Provider] {
        &ADDABLE_PROVIDERS
    }

    /// Whether an account added by hand at this provider is an
    /// organization, whose bill its admin key reads (Anthropic, OpenAI),
    /// rather than a key with money left on it.
    ///
    /// ```
    /// use otty_agents::Provider;
    ///
    /// assert!(Provider::OpenAI.is_added_as_organization());
    /// assert!(!Provider::DeepSeek.is_added_as_organization());
    /// ```
    pub fn is_added_as_organization(&self) -> bool {
        matches!(self, Self::Anthropic | Self::OpenAI)
    }

    /// The id the provider is saved under in `agents.json`, such as
    /// `anthropic`. It never changes once a provider is released.
    pub(crate) fn id(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::DeepSeek => "deepseek",
            Self::OpenRouter => "openrouter",
            Self::Moonshot => "moonshot",
            Self::Kilo => "kilo",
        }
    }

    /// The provider saved as `id`; `None` for an id this version does not
    /// know.
    pub(crate) fn of_id(id: &str) -> Option<Self> {
        ALL_PROVIDERS
            .into_iter()
            .find(|provider| provider.id() == id)
    }

    /// The provider whose server is at `base_url`, such as DeepSeek for
    /// `https://api.deepseek.com/anthropic`. `None` for a server OTTY does
    /// not know, such as a company gateway or a local model.
    pub(crate) fn of_base_url(base_url: &str) -> Option<Self> {
        let host = host_of(base_url)?;

        PROVIDER_DOMAINS
            .iter()
            .find(|(domain, _)| {
                host == *domain
                    || host
                        .strip_suffix(domain)
                        .is_some_and(|start| start.ends_with('.'))
            })
            .map(|(_, provider)| *provider)
    }
}

/// The host of `url` in lower case, such as `api.deepseek.com` of
/// `https://api.deepseek.com/anthropic`; `None` when it is not an address
/// of the web.
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

    #[test]
    fn the_server_of_a_known_provider_is_that_provider() {
        let cases = [
            ("https://api.anthropic.com", Provider::Anthropic),
            ("https://api.deepseek.com/anthropic", Provider::DeepSeek),
            ("https://openrouter.ai/api", Provider::OpenRouter),
            ("https://api.moonshot.ai/anthropic/", Provider::Moonshot),
            ("https://api.moonshot.cn/anthropic", Provider::Moonshot),
            ("https://api.kilo.ai/api/gateway", Provider::Kilo),
            ("https://api.openai.com/v1", Provider::OpenAI),
            (" HTTPS://API.DeepSeek.com ", Provider::DeepSeek),
        ];

        for (base_url, provider) in cases {
            assert_eq!(
                Provider::of_base_url(base_url),
                Some(provider),
                "{base_url}"
            );
        }
    }

    #[test]
    fn an_unknown_server_has_no_provider() {
        for base_url in [
            "https://gateway.example.com/anthropic",
            "http://localhost:4000",
            "https://deepseek.com.example.com",
            "https://notdeepseek.com",
            "not a url",
            "",
        ] {
            assert_eq!(Provider::of_base_url(base_url), None, "{base_url}");
        }
    }

    #[test]
    fn each_provider_is_found_again_by_its_saved_id() {
        let providers = [
            Provider::Anthropic,
            Provider::OpenAI,
            Provider::DeepSeek,
            Provider::OpenRouter,
            Provider::Moonshot,
            Provider::Kilo,
        ];

        for provider in providers {
            assert_eq!(Provider::of_id(provider.id()), Some(provider));
        }
        assert_eq!(Provider::Anthropic.id(), "anthropic");
        assert_eq!(Provider::of_id("cursor"), None);
    }
}
