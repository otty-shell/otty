//! The page of one account in the main area (spec 2026-11-19, Decisions 5
//! and 6): what it needs from the user, its numbers block, the profiles
//! that use it, its facts and where the numbers come from. No budget, and
//! no line under the title.

use std::time::SystemTime;

use otty_agents::{Account, AccountKind, AccountLogin};
use otty_app_i18n::{self as i18n, Key};

use super::super::state::AgentsState;
use super::account_name::account_name;
use super::account_numbers::AccountNumbersViewModel;
use super::found_in::found_in_text;
use super::needs::{NeedViewModel, needs_of_account};
use super::sources::{
    SourceRowViewModel, bill_state, limit_sources, money_sources,
};
use super::used_by::{UsedByViewModel, used_by};
use super::{FactViewModel, UNKNOWN};

/// The page of one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountPageViewModel {
    /// The account.
    pub(crate) account: Account,
    /// The provider and the login, such as "OpenAI · you@example.com".
    pub(crate) name: String,
    /// The name of the provider, for its icon.
    pub(crate) provider: &'static str,
    /// What the account needs from the user; the block shows only when
    /// there is any.
    pub(crate) needs: Vec<NeedViewModel>,
    /// The Limits, Credits or Money block, once anything is known.
    pub(crate) numbers: Option<AccountNumbersViewModel>,
    /// The profiles that use the account.
    pub(crate) used_by: Vec<UsedByViewModel>,
    /// Account info: Provider, Email or Key, Plan and Found in.
    pub(crate) info: Vec<FactViewModel>,
    /// Where the numbers of the block come from.
    pub(crate) sources: Vec<SourceRowViewModel>,
}

impl AccountPageViewModel {
    /// Build the page of `account` at `now`, the time the ages and the
    /// reset times count from.
    pub(crate) fn of(
        state: &AgentsState,
        account: Account,
        now: SystemTime,
    ) -> Self {
        Self {
            name: account_name(&account),
            provider: account.provider().name(),
            needs: needs_of_account(state, &account),
            numbers: AccountNumbersViewModel::of(state, &account, now),
            used_by: used_by(state, &account),
            info: info(state, &account),
            sources: sources(state, &account),
            account,
        }
    }
}

/// Account info: the provider, the email or the key, the plan when known,
/// and each place the account was found in, the first one named "Found
/// in".
fn info(state: &AgentsState, account: &Account) -> Vec<FactViewModel> {
    let mut list = vec![FactViewModel::new(
        i18n::t(Key::AgentsColumnProvider),
        account.provider().name().to_string(),
    )];

    match account.login() {
        AccountLogin::Email(email) => list.push(FactViewModel::new(
            i18n::t(Key::AgentsFieldEmail),
            email.clone(),
        )),
        AccountLogin::Key(ending) => {
            let key = ending.as_ref().map_or_else(
                || String::from(UNKNOWN),
                |last| {
                    i18n::filled(Key::TplAgentsAccountKey, &[("last", last)])
                },
            );
            list.push(FactViewModel::new(i18n::t(Key::AgentsFieldKey), key));
        },
        _ => {},
    }
    if let Some(plan) = plan(state, account) {
        list.push(FactViewModel::new(i18n::t(Key::AgentsFieldPlan), plan));
    }

    let home = state.env().home();
    for (index, place) in account.found_in().iter().enumerate() {
        let label = if index == 0 {
            i18n::t(Key::AgentsColumnFoundIn)
        } else {
            ""
        };
        list.push(FactViewModel::new(label, found_in_text(place, home)));
    }

    list
}

/// The plan of `account`: as its files name it, else as its last good
/// limits do, such as Codex's "Plus".
fn plan(state: &AgentsState, account: &Account) -> Option<String> {
    if let Some(plan) = account.plan() {
        return Some(plan.to_string());
    }

    let shown = state.profiles().with_limits_of(account)?;
    let (answer, _) = shown.limits().last_good()?;
    answer.value().plan().map(String::from)
}

/// Where the numbers of the block come from: the limit sources as the
/// latest read through one of its profiles walked them, or the sources of
/// the money of the first profile it was found in.
fn sources(state: &AgentsState, account: &Account) -> Vec<SourceRowViewModel> {
    let profiles = state.profiles();
    let time_zone = profiles.time_zone();

    if account.kind() != AccountKind::Money {
        return profiles
            .with_limits_of(account)
            .map(|shown| limit_sources(shown.limits(), time_zone))
            .unwrap_or_default();
    }

    let Some(shown) = profiles.first_with(account) else {
        return Vec::new();
    };
    money_sources(
        shown.spend(),
        shown.money(),
        profiles.prices(),
        bill_state(state, shown),
        time_zone,
    )
}

#[cfg(test)]
mod tests {
    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, FoundIn, KeyFingerprint,
        Provider, SavedProfiles, SourceError, SourceId,
    };

    use super::*;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, chatgpt_login, chatgpt_login_in, claude, codex,
        codex_work, connected, five_hour_limits, limits_answer, limits_loaded,
        now, organization, with_accounts,
    };

    fn fact(key: Key, value: &str) -> FactViewModel {
        FactViewModel::new(i18n::t(key), String::from(value))
    }

    fn found_in(folder: &str) -> FactViewModel {
        FactViewModel::new("", String::from(folder))
    }

    /// Codex and Codex · Work logged in to the [`chatgpt_login`] on the
    /// Plus plan, with 42 % used read through Codex.
    fn shared_chatgpt() -> AgentsState {
        let plus = |folder: &str| {
            chatgpt_login_in(folder).with_plan(Some(String::from("Plus")))
        };
        let mut state = with_accounts(
            &[
                (codex(), connected(), vec![plus("/home/user/.codex")]),
                (
                    codex_work(),
                    connected(),
                    vec![plus("/home/user/.codex-work")],
                ),
            ],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    SourceId::ChatGptUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
                at(10),
            ),
        );

        state
    }

    /// The page of `account` as the left panel opens it: the account with
    /// every place it was found in.
    fn page_of(state: &AgentsState, account: Account) -> AccountPageViewModel {
        let found = state
            .profiles()
            .accounts()
            .into_iter()
            .find(|found| found.is_same_account(&account))
            .expect("the account is found");

        AccountPageViewModel::of(state, found, now())
    }

    #[test]
    fn a_shared_login_shows_its_numbers_users_info_and_sources() {
        let state = shared_chatgpt();

        let page = page_of(&state, chatgpt_login());

        assert_eq!(page.name, "OpenAI · you@example.com");
        assert_eq!(page.provider, "OpenAI");
        assert!(page.needs.is_empty());
        assert!(matches!(
            page.numbers,
            Some(AccountNumbersViewModel::Limits(_))
        ));
        let users: Vec<&str> =
            page.used_by.iter().map(|chip| chip.name.as_str()).collect();
        assert_eq!(users, ["Codex", "Codex · Work"]);
        assert_eq!(
            page.info,
            vec![
                fact(Key::AgentsColumnProvider, "OpenAI"),
                fact(Key::AgentsFieldEmail, "you@example.com"),
                fact(Key::AgentsFieldPlan, "Plus"),
                fact(Key::AgentsColumnFoundIn, "~/.codex"),
                found_in("~/.codex-work"),
            ]
        );
        let sources: Vec<SourceId> =
            page.sources.iter().map(|row| row.source).collect();
        assert_eq!(sources, [SourceId::ChatGptUsageApi]);
    }

    /// A key shows its last four characters; a key found in the system
    /// keyring or added by the user says so.
    #[test]
    fn a_key_shows_its_ending_and_where_it_was_found() {
        let key = Account::new(
            Provider::Anthropic,
            AccountIdentity::KeyFingerprint(KeyFingerprint::of(
                "sk-made-up-key-for-tests-a1f3",
            )),
            AccountLogin::Key(Some(String::from("a1f3"))),
            AccountKind::Money,
            FoundIn::SystemKeyring,
        );
        let state = with_accounts(
            &[(claude(), api_key(), vec![key.clone()])],
            SavedProfiles::default(),
        );

        let page = page_of(&state, key);

        assert_eq!(
            page.info,
            vec![
                fact(Key::AgentsColumnProvider, "Anthropic"),
                FactViewModel::new(
                    i18n::t(Key::AgentsFieldKey),
                    i18n::filled(Key::TplAgentsAccountKey, &[("last", "a1f3")])
                ),
                fact(
                    Key::AgentsColumnFoundIn,
                    i18n::t(Key::AgentsFoundInKeyring)
                ),
            ]
        );
    }

    /// Spec 2026-11-19, Decision 3: an expired login needs the user to
    /// open the agent, and the last numbers stay, greyed.
    #[test]
    fn an_expired_login_asks_to_open_the_agent() {
        let mut state = shared_chatgpt();
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    SourceId::ChatGptUsageApi,
                    Err(SourceError::NotLoggedIn),
                )]),
                at(20),
            ),
        );

        let page = page_of(&state, chatgpt_login());

        let actions: Vec<NeedAction> = page
            .needs
            .iter()
            .filter_map(|need| need.fix.as_ref())
            .map(|fix| fix.action.clone())
            .collect();
        assert_eq!(actions, vec![NeedAction::OpenAgent(codex())]);
        assert!(matches!(
            page.numbers,
            Some(AccountNumbersViewModel::Limits(limits)) if limits.is_stale
        ));
    }

    /// The money of an organization comes from its bill, then the
    /// estimate; with no admin key the bill is not available.
    #[test]
    fn an_organization_lists_the_sources_of_its_money() {
        let state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            SavedProfiles::default(),
        );

        let page = page_of(&state, organization());

        let sources: Vec<SourceId> =
            page.sources.iter().map(|row| row.source).collect();
        assert_eq!(sources, [SourceId::AnthropicCostReport]);
        assert_eq!(page.needs.len(), 1, "the admin key is needed");
    }
}
