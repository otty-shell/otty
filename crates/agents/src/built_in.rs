//! The registry entries of the readers of this crate (spec "Links in the
//! interface"): today's ids, names, hosts, usage pages and the icons of
//! `assets/svg/`. They are "built in" until the plugin of each agent and
//! provider replaces them, and each goes away with its reader: only
//! Moonshot is left, until issue 17 of spec 2026-11-19-agent-plugins.

use crate::provider::{AddAccountKind, ProviderEntry, ProviderId};
use crate::registry::Reader;

/// The id of Moonshot AI (Kimi) API keys.
pub(crate) const MOONSHOT: &str = "moonshot";

/// Moonshot, which has no logo here: the two cards of All accounts, as
/// before the registry.
const NO_LOGO_ICON: &[u8] =
    include_bytes!("../../../assets/svg/all-accounts.svg");

/// The providers the readers of this crate read, in the order they are
/// shown.
pub(crate) fn providers() -> Vec<ProviderEntry> {
    vec![
        ProviderEntry::new(
            ProviderId::new(MOONSHOT),
            "Moonshot",
            Reader::BuiltIn,
        )
        .with_icon(NO_LOGO_ICON)
        .with_hosts(["moonshot.ai", "moonshot.cn"])
        .with_usage_page("https://platform.moonshot.ai/console")
        .with_add_account(AddAccountKind::Key),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountKind;

    #[test]
    fn moonshot_opens_the_usage_page_of_its_console() {
        let moonshot = providers().remove(0);

        assert_eq!(
            moonshot.usage_page(AccountKind::Money),
            Some("https://platform.moonshot.ai/console")
        );
    }
}
