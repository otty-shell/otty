//! The page of one account in the main area (spec 2026-11-19, Decisions 5
//! and 6): what it needs from the user, its numbers block and its facts.
//! No budget, no line under the title, and no profiles: nothing links an
//! account to a profile (spec 2026-11-23-accounts-apart-from-profiles,
//! Decision 1).
//! Where a number comes from is the plugin's business (issue 19 of spec
//! 2026-11-19-agent-plugins): the page tells only how old the numbers are,
//! what to do, and which plugin reads them.

use std::time::SystemTime;

use otty_agents::{Account, AccountKind};
use otty_app_i18n::{self as i18n, Key};

use super::super::own_login_actions::has_own_login;
use super::super::state::AgentsState;
use super::FactViewModel;
use super::account_name::short_account_name;
use super::account_numbers::AccountNumbersViewModel;
use super::entry_icon::EntryIcon;
use super::needs::{NeedViewModel, needs_of_account};
use super::read_by::account_read_by;
use super::source::source_text;

/// The page of one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountPageViewModel {
    /// The account.
    pub(crate) account: Account,
    /// The name the part gives without the provider the logo beside it
    /// shows, such as "you@example.com", or the name the user gave the
    /// account. Account info names the provider.
    pub(crate) name: String,
    /// The icon of the provider.
    pub(crate) icon: EntryIcon,
    /// What the account needs from the user; the block shows only when
    /// there is any.
    pub(crate) needs: Vec<NeedViewModel>,
    /// The Limits, Credits or Money block, once anything is known.
    pub(crate) numbers: Option<AccountNumbersViewModel>,
    /// Account info: Provider, Name, Kind, Plan, Source and Read by.
    pub(crate) info: Vec<FactViewModel>,
    /// For an account the user hid, the line under the title that says
    /// OTTY does not read it; its page then has no needs or numbers.
    pub(crate) hidden_note: Option<String>,
    /// Whether the account has OTTY's own login: the header shows the mark
    /// "OTTY's own login" and Log in again (issue 09 of spec
    /// 2026-11-23-accounts-apart-from-profiles).
    pub(crate) has_own_login: bool,
}

impl AccountPageViewModel {
    /// Build the page of `account` at `now`, the time the ages and the
    /// reset times count from. A hidden account is not read, so its page
    /// says so in place of its needs and numbers.
    pub(crate) fn of(
        state: &AgentsState,
        account: Account,
        now: SystemTime,
    ) -> Self {
        let page = Self {
            name: short_account_name(state.profiles().saved(), &account),
            icon: state.account_icon(&account),
            needs: Vec::new(),
            numbers: None,
            info: info(state, &account),
            hidden_note: None,
            has_own_login: has_own_login(state, &account),
            account,
        };
        if state.profiles().is_account_hidden(&page.account) {
            let note = i18n::t(Key::AgentsHiddenAccountNote).to_string();
            return Self {
                hidden_note: Some(note),
                ..page
            };
        }

        Self {
            needs: needs_of_account(state, &page.account),
            numbers: AccountNumbersViewModel::of(state, &page.account, now),
            ..page
        }
    }
}

/// The kind of an account in words, such as "Subscription", as Account
/// info and the Found by OTTY report write it.
pub(crate) fn kind_text(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Subscription => i18n::t(Key::AgentsKindSubscription),
        AccountKind::Key => i18n::t(Key::AgentsKindKey),
        AccountKind::Organization => i18n::t(Key::AgentsKindOrganization),
    }
}

/// Account info (spec 2026-11-23-accounts-apart-from-profiles, "Account
/// page"): the provider, the name its part gives, its kind, the plan when
/// known, its Source, one line per folder it was found in, and the plugin
/// that reads it.
fn info(state: &AgentsState, account: &Account) -> Vec<FactViewModel> {
    let mut list = vec![
        FactViewModel::new(
            i18n::t(Key::AgentsColumnProvider),
            state.account_provider_name(account).to_string(),
        ),
        FactViewModel::new(
            i18n::t(Key::AgentsFieldName),
            account.name().to_string(),
        ),
        FactViewModel::new(
            i18n::t(Key::AgentsFieldKind),
            kind_text(account.kind()).to_string(),
        ),
    ];
    if let Some(plan) = plan(state, account) {
        list.push(FactViewModel::new(i18n::t(Key::AgentsFieldPlan), plan));
    }

    let home = state.env().home();
    for (index, place) in account.found_in().iter().enumerate() {
        let label = match index {
            0 => i18n::t(Key::AgentsFieldSource),
            _ => "",
        };
        list.push(FactViewModel::new(label, source_text(place, home)));
    }
    list.push(account_read_by(state, account));

    list
}

/// The plan of `account`: as its files name it, else as its last good
/// limits do, such as "Plus".
fn plan(state: &AgentsState, account: &Account) -> Option<String> {
    if let Some(plan) = account.plan() {
        return Some(plan.to_string());
    }

    let limits = state.profiles().account_limits(account)?;
    let (answer, _) = limits.last_good()?;
    answer.value().plan().map(String::from)
}

#[cfg(test)]
mod tests {
    use otty_agents::{AccountKind, ProviderId, SavedProfiles};

    use super::*;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        at, chatgpt_login, chatgpt_login_in, claude, codex, codex_work,
        connected, five_hour_limits, limits_numbers, now, numbers_loaded,
        organization, with_accounts,
    };

    fn fact(key: Key, value: &str) -> FactViewModel {
        FactViewModel::new(i18n::t(key), String::from(value))
    }

    /// The Source line of a folder after the first one.
    fn also_found_in(folder: &str) -> FactViewModel {
        FactViewModel::new("", source(folder))
    }

    /// The Source line of `folder`: "Found in" it.
    fn source(folder: &str) -> String {
        i18n::filled(Key::TplAgentsSourceFoundIn, &[("folder", folder)])
    }

    /// Codex and Codex (Work) logged in to the [`chatgpt_login`] on the
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
            numbers_loaded(
                &chatgpt_login(),
                Ok(limits_numbers(five_hour_limits(42.0))),
                at(10),
            ),
        );

        state
    }

    /// The page of `account` as the left panel opens it: the account with
    /// every place it was found in.
    /// "Read by" the plugin `plugin`, named `name` with its version.
    fn read_by_plugin(name: &str, plugin: &str) -> FactViewModel {
        FactViewModel::linked(
            i18n::t(Key::AgentsFieldReadBy),
            name.to_string(),
            plugin.to_string(),
        )
    }

    fn page_of(state: &AgentsState, account: Account) -> AccountPageViewModel {
        let found = state
            .profiles()
            .accounts()
            .into_iter()
            .find(|found| found.is_same_account(&account))
            .expect("the account is found");

        AccountPageViewModel::of(state, found, now())
    }

    /// Spec 2026-11-23, Decision 1: the page of a login two profiles use
    /// shows its numbers and its info, and no Used by. Spec
    /// 2026-11-23-accounts-apart-from-profiles, "Account page": Account
    /// info is Provider, Name, Kind, Plan, Source, one line per folder,
    /// and Read by.
    #[test]
    fn a_shared_login_shows_its_numbers_and_info() {
        let state = shared_chatgpt();

        let page = page_of(&state, chatgpt_login());

        assert_eq!(page.name, "you@example.com", "the logo says OpenAI");
        assert_eq!(page.icon, crate::testing::provider_icon("openai"));
        assert!(page.needs.is_empty());
        assert!(matches!(
            page.numbers,
            Some(AccountNumbersViewModel::Limits(_))
        ));
        assert_eq!(
            page.info,
            vec![
                fact(Key::AgentsColumnProvider, "OpenAI"),
                fact(Key::AgentsFieldName, "you@example.com"),
                fact(
                    Key::AgentsFieldKind,
                    i18n::t(Key::AgentsKindSubscription)
                ),
                fact(Key::AgentsFieldPlan, "Plus"),
                fact(Key::AgentsFieldSource, &source("~/.codex")),
                also_found_in("~/.codex-work"),
                read_by_plugin("OpenAI 0.1.0", "openai"),
            ]
        );
    }

    /// Issue 09 of spec 2026-11-23-accounts-apart-from-profiles: the page
    /// of an account with OTTY's own login has its mark and Log in again in
    /// the header; the page of a found account has neither.
    #[test]
    fn only_the_page_of_an_own_login_has_its_mark() {
        let own = crate::testing::with_own_login(vec![
            crate::testing::chatgpt_login_in_both(),
        ]);
        let found = shared_chatgpt();

        let own_page = page_of(&own, chatgpt_login());
        let found_page = page_of(&found, chatgpt_login());

        assert!(own_page.has_own_login);
        assert!(!found_page.has_own_login);
    }

    /// A key shows the name its part gives it and the folder it was
    /// found in.
    #[test]
    fn a_key_shows_its_name_and_where_it_was_found() {
        let key = Account::new(
            "anthropic",
            ProviderId::new("anthropic"),
            "key:made-up-a1f3",
            "key ••••a1f3",
            AccountKind::Key,
            vec![crate::testing::found_in(
                crate::testing::claude_id(),
                "/home/user/.claude",
            )],
        );
        let state = with_accounts(
            &[(claude(), connected(), vec![key.clone()])],
            SavedProfiles::default(),
        );

        let page = page_of(&state, key);

        assert_eq!(
            page.info,
            vec![
                fact(Key::AgentsColumnProvider, "Anthropic"),
                fact(Key::AgentsFieldName, "key ••••a1f3"),
                fact(Key::AgentsFieldKind, i18n::t(Key::AgentsKindKey)),
                fact(Key::AgentsFieldSource, &source("~/.claude")),
                read_by_plugin("Anthropic 0.1.0", "anthropic"),
            ]
        );
    }

    /// Issue 19 and 20 of spec 2026-11-19-agent-plugins: a read the part
    /// did not answer shows what the part asks, and the last numbers stay,
    /// greyed, with why under them.
    #[test]
    fn a_read_the_part_did_not_answer_shows_its_need() {
        let mut state = shared_chatgpt();
        let _ = reduce(
            &mut state,
            numbers_loaded(
                &chatgpt_login(),
                Err(crate::testing::login_expired()),
                at(20),
            ),
        );

        let page = page_of(&state, chatgpt_login());

        let actions: Vec<NeedAction> = page
            .needs
            .iter()
            .filter_map(|need| need.fix.as_ref())
            .filter_map(|fix| fix.action.clone())
            .collect();
        assert_eq!(
            actions,
            vec![NeedAction::Run {
                title: String::from("OpenAI (you@example.com)"),
                command: otty_agents::AgentCommand::new("codex"),
            }]
        );
        let Some(AccountNumbersViewModel::Limits(limits)) = page.numbers else {
            panic!("the limits block stays");
        };
        assert!(limits.is_stale);
        assert_eq!(limits.windows.len(), 1, "the last numbers stay");
        assert!(limits.failure.is_some());
    }

    /// Issue 20: the provider part asks for the key of the bill of an
    /// organization; the page shows it as a need of the account.
    #[test]
    fn an_organization_whose_part_asks_for_a_key_shows_the_need() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            crate::testing::saved_with_given_key(),
        );
        let _ = reduce(
            &mut state,
            crate::testing::bill_loaded(Err(crate::testing::bill_key_needed())),
        );

        let page = page_of(&state, organization());

        assert_eq!(page.needs.len(), 1, "the key is asked for");
        assert!(matches!(
            page.needs[0]
                .fix
                .as_ref()
                .and_then(|fix| fix.action.as_ref()),
            Some(NeedAction::AskKey { .. })
        ));
    }
}
