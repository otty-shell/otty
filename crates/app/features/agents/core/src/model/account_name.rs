//! The name an account is shown by: the user's own (spec 2026-11-19,
//! Decision 7), else the name its provider part gives, such as an email
//! or a short key (spec 2026-11-23-accounts-apart-from-profiles, Decision
//! 8). The left panel, whose rows show the logo of the provider, names an
//! account without its provider.

use otty_agents::{Account, Registry, SavedProfiles};

use super::super::state::account_provider_name;

/// The name of `account`: the one the user gave it in `saved`, the
/// content of `agents.json`, else its own ([`own_account_name`]), with
/// the name of its provider from `registry`.
pub(crate) fn account_name(
    registry: &Registry,
    saved: Option<&SavedProfiles>,
    account: &Account,
) -> String {
    match saved.and_then(|saved| saved.account_name(account)) {
        Some(name) => name.to_string(),
        None => own_account_name(registry, account),
    }
}

/// The name of `account` in a row that already shows the logo of its
/// provider: the one the user gave it in `saved`, else the name its part
/// gives, such as "you@example.com" or "key ••••a1f3".
pub(crate) fn short_account_name(
    saved: Option<&SavedProfiles>,
    account: &Account,
) -> String {
    match saved.and_then(|saved| saved.account_name(account)) {
        Some(name) => name.to_string(),
        None => account.name().to_string(),
    }
}

/// The provider and the name the part gives `account`, such as "OpenAI ·
/// you@example.com". The provider is named as the entry of `registry` of
/// the part that found it names it, else by its id.
pub(crate) fn own_account_name(
    registry: &Registry,
    account: &Account,
) -> String {
    let provider = account_provider_name(registry, account);

    format!("{provider} · {}", account.name())
}

#[cfg(test)]
mod tests {
    use otty_agents::{AccountKind, ProviderId};

    use super::*;
    use crate::testing::registry;

    fn login(plugin: &str) -> Account {
        Account::new(
            plugin,
            ProviderId::new("openai"),
            "chatgpt:made-up-account",
            "you@example.com",
            AccountKind::Subscription,
            Vec::new(),
        )
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 8: an
    /// account is named by its provider and the name its part gives.
    #[test]
    fn an_account_is_named_by_its_provider_and_its_parts_name() {
        let registry = registry();

        assert_eq!(
            own_account_name(&registry, &login("openai")),
            "OpenAI · you@example.com"
        );
        assert_eq!(
            short_account_name(None, &login("openai")),
            "you@example.com"
        );
    }

    /// The user's name wins, kept by the plugin and the id.
    #[test]
    fn the_users_name_wins() {
        let saved = SavedProfiles::default()
            .with_account_name(&login("openai"), Some(String::from("Home")));

        assert_eq!(short_account_name(Some(&saved), &login("openai")), "Home");
        assert_eq!(
            short_account_name(Some(&saved), &login("openai-fork")),
            "you@example.com"
        );
    }
}
