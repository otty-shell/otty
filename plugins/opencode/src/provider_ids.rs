//! The ids OpenCode gives the providers, as in `providerID` of an answer,
//! the keys of `auth.json` and `integration_id` of a login, and the ids of
//! the provider parts that serve them in OTTY.

use otty_plugin_sdk::ProviderRef;

/// The id of OpenAI in OTTY: ChatGPT logins and OpenAI keys.
pub(crate) const OPENAI: &str = "openai";

/// The id of Kilo in OTTY, the gateway of Kilo Code. No official plugin
/// serves it; its logins show as "no provider plugin".
pub(crate) const KILO: &str = "kilo";

/// The providers OpenCode names, as in the models.dev catalog it uses,
/// with the ids of OTTY's provider parts.
const PROVIDER_IDS: [(&str, &str); 6] = [
    ("anthropic", "anthropic"),
    ("openai", OPENAI),
    ("deepseek", "deepseek"),
    ("openrouter", "openrouter"),
    ("kilo", KILO),
    ("kilocode", KILO),
];

/// The provider OpenCode names `id`; `None` for one OTTY does not read,
/// such as a local model or a company gateway.
pub(crate) fn provider_of(id: &str) -> Option<ProviderRef> {
    let id = id.trim().trim_end_matches('/');

    PROVIDER_IDS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(id))
        .map(|(_, provider)| ProviderRef::Id(provider.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(provider: &str) -> Option<ProviderRef> {
        Some(ProviderRef::Id(provider.to_string()))
    }

    #[test]
    fn known_ids_are_their_providers_and_others_are_none() {
        assert_eq!(provider_of("openai"), id("openai"));
        assert_eq!(provider_of("DeepSeek"), id("deepseek"));
        assert_eq!(provider_of("kilocode"), id("kilo"));
        assert_eq!(provider_of("kilo/"), id("kilo"));
        assert_eq!(provider_of("test"), None, "a local model");
        assert_eq!(provider_of("github-copilot"), None);
    }
}
