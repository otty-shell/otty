//! The registry entries of the readers of this crate (spec "Links in the
//! interface"): today's ids, names, hosts, usage pages and the icons of
//! `assets/svg/`. They are "built in" until the plugin of each agent and
//! provider replaces them, and each goes away with its reader.

use crate::provider::{AddAccountKind, ProviderEntry, ProviderId};
use crate::registry::Reader;

/// The id of Anthropic: Claude plans and Anthropic Console organizations.
pub(crate) const ANTHROPIC: &str = "anthropic";
/// The id of OpenAI: ChatGPT plans and OpenAI API keys.
pub(crate) const OPENAI: &str = "openai";
/// The id of Moonshot AI (Kimi) API keys.
pub(crate) const MOONSHOT: &str = "moonshot";

/// Claude Code and Anthropic: the Claude logo from Simple Icons, in one
/// color that the app tints.
pub(crate) const CLAUDE_ICON: &[u8] =
    include_bytes!("../../../assets/svg/agent-claude-code.svg");
/// Codex and OpenAI: the OpenAI logo from Simple Icons, in one color that
/// the app tints.
pub(crate) const OPENAI_ICON: &[u8] =
    include_bytes!("../../../assets/svg/agent-codex.svg");
/// Moonshot, which has no logo here: the two cards of All accounts, as
/// before the registry.
const NO_LOGO_ICON: &[u8] =
    include_bytes!("../../../assets/svg/all-accounts.svg");

/// The providers the readers of this crate read, in the order they are
/// shown.
pub(crate) fn providers() -> Vec<ProviderEntry> {
    vec![
        built_in(ANTHROPIC, "Anthropic", CLAUDE_ICON, &["anthropic.com"])
            .with_plan_usage_page("https://claude.ai/settings/usage")
            .with_usage_page("https://console.anthropic.com/settings/usage")
            .with_add_account(AddAccountKind::AdminKey),
        built_in(OPENAI, "OpenAI", OPENAI_ICON, &["openai.com"])
            .with_plan_usage_page("https://chatgpt.com/codex/settings/usage")
            .with_usage_page("https://platform.openai.com/usage")
            .with_add_account(AddAccountKind::AdminKey),
        built_in(
            MOONSHOT,
            "Moonshot",
            NO_LOGO_ICON,
            &["moonshot.ai", "moonshot.cn"],
        )
        .with_usage_page("https://platform.moonshot.ai/console")
        .with_add_account(AddAccountKind::Key),
    ]
}

/// A built-in provider entry with its id, name, icon and hosts.
fn built_in(
    id: &str,
    name: &str,
    icon: &'static [u8],
    hosts: &[&str],
) -> ProviderEntry {
    ProviderEntry::new(ProviderId::new(id), name, Reader::BuiltIn)
        .with_icon(icon)
        .with_hosts(hosts.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountKind;

    fn page(id: &str, kind: AccountKind) -> Option<String> {
        providers()
            .into_iter()
            .find(|entry| entry.id().as_str() == id)
            .and_then(|entry| entry.usage_page(kind).map(String::from))
    }

    /// Today's usage pages: a subscription opens the page of its plan, a
    /// key or an organization the usage page of the provider's console.
    #[test]
    fn each_provider_opens_todays_usage_page() {
        let cases = [
            (
                ANTHROPIC,
                AccountKind::Limits,
                "https://claude.ai/settings/usage",
            ),
            (
                ANTHROPIC,
                AccountKind::Money,
                "https://console.anthropic.com/settings/usage",
            ),
            (
                OPENAI,
                AccountKind::Limits,
                "https://chatgpt.com/codex/settings/usage",
            ),
            (
                OPENAI,
                AccountKind::Money,
                "https://platform.openai.com/usage",
            ),
            (
                MOONSHOT,
                AccountKind::Money,
                "https://platform.moonshot.ai/console",
            ),
        ];

        for (id, kind, expected) in cases {
            assert_eq!(page(id, kind).as_deref(), Some(expected), "{id}");
        }
    }
}
