//! The ids OpenCode and Kilo give the providers, as in `providerID` of an
//! answer, the keys of `auth.json` and `integration_id` of a login.

use crate::provider::Provider;

/// The ids of the providers OTTY knows, as the agents name them in the
/// models.dev catalog they use.
const PROVIDER_IDS: [(&str, Provider); 8] = [
    ("anthropic", Provider::Anthropic),
    ("openai", Provider::OpenAI),
    ("deepseek", Provider::DeepSeek),
    ("openrouter", Provider::OpenRouter),
    ("moonshotai", Provider::Moonshot),
    ("moonshotai-cn", Provider::Moonshot),
    ("kilo", Provider::Kilo),
    ("kilocode", Provider::Kilo),
];

/// The provider the agent names `id`; `None` for a provider OTTY does not
/// know, such as a local model or a company gateway.
pub(super) fn provider_of(id: &str) -> Option<Provider> {
    let id = id.trim().trim_end_matches('/');

    PROVIDER_IDS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(id))
        .map(|(_, provider)| *provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ids_are_their_providers_and_others_are_none() {
        assert_eq!(provider_of("openai"), Some(Provider::OpenAI));
        assert_eq!(provider_of("moonshotai-cn"), Some(Provider::Moonshot));
        assert_eq!(provider_of("kilo/"), Some(Provider::Kilo));
        assert_eq!(provider_of("test"), None, "a local model");
        assert_eq!(provider_of("github-copilot"), None);
    }
}
