//! The ids OpenCode and Kilo give the providers, as in `providerID` of an
//! answer, the keys of `auth.json` and `integration_id` of a login.

use crate::built_in::{
    ANTHROPIC, DEEPSEEK, KILO, MOONSHOT, OPENAI, OPENROUTER,
};
use crate::provider::ProviderRef;

/// The ids of the providers OTTY reads, as the agents name them in the
/// models.dev catalog they use, with the ids OTTY gives them.
const PROVIDER_IDS: [(&str, &str); 8] = [
    ("anthropic", ANTHROPIC),
    ("openai", OPENAI),
    ("deepseek", DEEPSEEK),
    ("openrouter", OPENROUTER),
    ("moonshotai", MOONSHOT),
    ("moonshotai-cn", MOONSHOT),
    ("kilo", KILO),
    ("kilocode", KILO),
];

/// The provider the agent names `id`; `None` for a provider OTTY does not
/// know, such as a local model or a company gateway.
pub(super) fn provider_of(id: &str) -> Option<ProviderRef> {
    let id = id.trim().trim_end_matches('/');

    PROVIDER_IDS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(id))
        .map(|(_, provider)| ProviderRef::of_id(*provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ids_are_their_providers_and_others_are_none() {
        assert_eq!(provider_of("openai"), Some(ProviderRef::of_id("openai")));
        assert_eq!(
            provider_of("moonshotai-cn"),
            Some(ProviderRef::of_id("moonshot"))
        );
        assert_eq!(provider_of("kilo/"), Some(ProviderRef::of_id("kilo")));
        assert_eq!(provider_of("test"), None, "a local model");
        assert_eq!(provider_of("github-copilot"), None);
    }
}
