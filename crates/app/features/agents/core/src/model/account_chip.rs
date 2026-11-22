//! The chip of an account (issue 38 of spec 2026-11-19-agent-plugins):
//! the logo of its provider, its name by the login alone and its number.
//! Every chip of an account, on a profile page and on a project page, is
//! built here, so they all look the same and change in one place.

use otty_agents::Account;

use super::super::state::AgentsState;
use super::account_name::{own_account_name, short_account_name};
use super::entry_icon::EntryIcon;
use super::short_number::{AccountNumber, account_number};

/// One chip of an account; a press opens the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountChipViewModel {
    /// The account, sent when the chip is pressed.
    pub(crate) account: Account,
    /// The logo of its provider.
    pub(crate) icon: EntryIcon,
    /// The login without the provider its logo shows, such as
    /// "you@example.com", or the name the user gave the account.
    pub(crate) name: String,
    /// The whole name in the tooltip, which says the provider too, such
    /// as "OpenAI · you@example.com": after the name the user gave, on a
    /// line of its own, when they gave one.
    pub(crate) full_name: String,
    /// Its number, its color and the tooltip of an estimate, as in the
    /// left panel ("42 % used", "login expired"); `None` while nothing is
    /// known.
    pub(crate) number: Option<AccountNumber>,
}

/// The chip of `account`, named and numbered from `state`.
pub(crate) fn account_chip(
    state: &AgentsState,
    account: &Account,
) -> AccountChipViewModel {
    let registry = state.registry();
    let saved = state.profiles().saved();

    let own = own_account_name(registry, account);
    let full_name = match saved.and_then(|saved| saved.account_name(account)) {
        Some(given) => format!("{given}\n{own}"),
        None => own,
    };

    AccountChipViewModel {
        account: account.clone(),
        icon: state.provider_icon(account.provider()),
        name: short_account_name(registry, saved, account),
        full_name,
        number: account_number(state, account),
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SavedProfiles;

    use super::*;
    use crate::testing::{
        chatgpt_login, codex, connected, organization, with_accounts,
    };

    fn chip_of(
        saved: SavedProfiles,
        account: &Account,
    ) -> AccountChipViewModel {
        let state = with_accounts(
            &[(codex(), connected(), vec![account.clone()])],
            saved,
        );

        account_chip(&state, account)
    }

    /// Issue 38: the chip names the account by its login, as its logo
    /// says the provider; the full name with the provider is its tooltip.
    #[test]
    fn a_chip_shows_the_login_and_the_full_name_on_hover() {
        let chip = chip_of(SavedProfiles::default(), &chatgpt_login());

        assert_eq!(chip.name, "you@example.com");
        assert_eq!(chip.full_name, "OpenAI · you@example.com");
        assert_eq!(chip.icon, crate::testing::provider_icon("openai"));
        assert_eq!(chip.account, chatgpt_login());
    }

    /// A name the user gave shows as given; the tooltip keeps it and the
    /// provider with the login.
    #[test]
    fn a_name_the_user_gave_shows_as_given() {
        let saved = SavedProfiles::default()
            .with_account_name(&organization(), Some(String::from("Work")));

        let chip = chip_of(saved, &organization());

        assert_eq!(chip.name, "Work");
        assert_eq!(chip.full_name, "Work\nAnthropic · dev@example.com");
    }

    /// A login two plugins read keeps its plugin, so the two chips tell
    /// apart.
    #[test]
    fn a_shared_account_names_its_plugin() {
        let fork = otty_agents::ProviderEntry::new(
            otty_agents::ProviderId::new("openai"),
            "OpenAI",
            otty_plugin_host::PluginPart::new(
                "openai-fork",
                "OpenAI Fork",
                "0.1.0",
                "",
                "openai",
            ),
        );
        let found = chatgpt_login().with_read_by(Some(String::from("openai")));
        let state = crate::testing::with_accounts_in(
            crate::testing::registry_with(fork),
            &[(codex(), connected(), vec![found.clone()])],
            SavedProfiles::default(),
        );

        let chip = account_chip(&state, &found);

        assert_eq!(chip.name, "you@example.com (OpenAI)");
        assert_eq!(chip.full_name, "OpenAI · you@example.com (OpenAI)");
    }
}
