//! The numbers block of an account page (spec 2026-11-19, Decision 5),
//! named after what the account has: Limits (usage windows of a
//! subscription), Credits, or Money (Left, the money left on the account;
//! Key limit, of a key with a limit; Credits of a key's account; Spent this
//! month, from the bill or the provider's count). An account's money is
//! only what its provider tells: the tokens of the profiles do not count
//! for it (spec 2026-11-23-accounts-apart-from-profiles, Decision 1).
//! There is no budget: OTTY only shows. The last good numbers stay,
//! greyed, when a later read fails.

use std::time::SystemTime;

use otty_agents::{
    Account, AccountKind, Amount, Balance, FoundIn, KeyLimit, MoneyByKind,
    SourceError,
};
use otty_app_i18n::{self as i18n, Key};

use super::super::account_reads::AccountRead;
use super::super::failure_text::{named_failure, source_error_text};
use super::super::state::AgentsState;
use super::limits::{
    LimitsViewModel, UsageRowViewModel, credits_left, updated_text,
};
use super::money::{
    MoneyViewModel, OrganizationBillViewModel, SpentAtApiPricesViewModel,
};
use super::numbers::{dollars_text, money_text};

/// The numbers block of an account page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AccountNumbersViewModel {
    /// The usage windows of a subscription.
    Limits(LimitsViewModel),
    /// The credits of an account paid for by credits.
    Credits(LimitsViewModel),
    /// The money of an account paid per token.
    Money(MoneyViewModel),
}

impl AccountNumbersViewModel {
    /// The title of the block: "Limits", "Credits" or "Money".
    pub(crate) fn title(&self) -> &'static str {
        match self {
            Self::Limits(_) => i18n::t(Key::AgentsNumberLimits),
            Self::Credits(_) => i18n::t(Key::AgentsFieldCredits),
            Self::Money(_) => i18n::t(Key::AgentsNumberMoney),
        }
    }

    /// The numbers of `account` at `now`; `None` while nothing is known.
    pub(crate) fn of(
        state: &AgentsState,
        account: &Account,
        now: SystemTime,
    ) -> Option<Self> {
        match account.kind() {
            AccountKind::Subscription => limits(state, account, now),
            _ => money(state, account, now).map(Self::Money),
        }
    }
}

/// The amount `amount` in its currency, such as "$4.20" or "¥110.00".
pub(super) fn amount_text(amount: &Amount) -> String {
    money_text(amount.value(), Some(amount.currency()))
}

/// The usage windows or the credits of a subscription, as the latest read
/// through one of its profiles left them, or the read with its own login
/// when no profile reads them. The login hint is left out: the need of the
/// page says what to do.
fn limits(
    state: &AgentsState,
    account: &Account,
    now: SystemTime,
) -> Option<AccountNumbersViewModel> {
    let profiles = state.profiles();
    let account_limits = profiles.account_limits(account)?;
    let limits = LimitsViewModel::from_limits(
        &account_limits,
        now,
        profiles.time_zone(),
    )?;

    if limits.credits.is_some() {
        return Some(AccountNumbersViewModel::Credits(limits));
    }
    Some(AccountNumbersViewModel::Limits(limits))
}

/// The money of an account paid per token: what its own key told, the
/// money left on it and the limit of the key, then what it spent this
/// month. The numbers its own key told stay, greyed, when a later read
/// fails, with why.
fn money(
    state: &AgentsState,
    account: &Account,
    now: SystemTime,
) -> Option<MoneyViewModel> {
    let own = state.profiles().account_reads().of(account);
    let spent = spent_money(state, account, own, now);
    let Some(own) = own else {
        return spent;
    };

    let mut rows = balance_rows(own.balance().map(|(balance, _)| balance));
    let own_read_at = own.balance().map(|(_, read_at)| read_at);
    let failure = own
        .failure()
        .map(|error| own_failure_text(state, account, error));
    if rows.is_empty() && spent.is_none() && failure.is_none() {
        return None;
    }

    let mut vm = spent.unwrap_or(MoneyViewModel {
        updated: None,
        rows: Vec::new(),
        spent: None,
        spent_at_api_prices: None,
        organization_bill: None,
        is_stale: false,
        failure: None,
    });
    rows.append(&mut vm.rows);
    vm.rows = rows;
    if let Some(read_at) = own_read_at {
        vm.updated = Some(updated_text(now, read_at));
    }
    if let Some(failure) = failure {
        vm.is_stale = vm.is_stale || own_read_at.is_some();
        vm.failure = Some(failure);
    }
    Some(vm)
}

/// The rows of what the key of an account told: Left, the money left on
/// the account, Key limit, what the key used of its limit, and Credits,
/// what is left of the credits of the account.
fn balance_rows(balance: Option<&Balance>) -> Vec<UsageRowViewModel> {
    let Some(balance) = balance else {
        return Vec::new();
    };

    let left = balance.left().map(|left| UsageRowViewModel {
        name: i18n::t(Key::AgentsLeft).to_string(),
        used: amount_text(left),
        percent: None,
        resets: None,
    });
    let key_limit = balance.key_limit().map(key_limit_row);
    let credits = balance.credits().and_then(credits_left).map(|left| {
        UsageRowViewModel {
            name: i18n::t(Key::AgentsFieldCredits).to_string(),
            used: left,
            percent: None,
            resets: None,
        }
    });

    left.into_iter().chain(key_limit).chain(credits).collect()
}

/// The row of the limit of a key: "$12.30 of $50.00" with a bar, or
/// "$0.50, no limit".
fn key_limit_row(key_limit: &KeyLimit) -> UsageRowViewModel {
    let used = amount_text(key_limit.used());
    let (text, percent) = match key_limit.limit() {
        Some(limit) => {
            let text = i18n::filled(
                Key::TplAgentsKeyLimitUsed,
                &[("used", &used), ("limit", &amount_text(limit))],
            );
            let percent = (limit.value() > 0.0).then(|| {
                let share = key_limit.used().value() / limit.value() * 100.0;
                share.clamp(0.0, 100.0).round() as u8
            });
            (text, percent)
        },
        None => (
            i18n::filled(Key::TplAgentsKeyUsedNoLimit, &[("used", &used)]),
            None,
        ),
    };

    UsageRowViewModel {
        name: i18n::t(Key::AgentsKeyLimit).to_string(),
        used: text,
        percent,
        resets: None,
    }
}

/// Why the own read of `account` failed: the provider whose part read it
/// and the reason, such as "Example Cloud: the key was refused".
fn own_failure_text(
    state: &AgentsState,
    account: &Account,
    error: &SourceError,
) -> String {
    let reason = source_error_text(error);

    match state.registry().reader_of(account) {
        Some(provider) => named_failure(provider.name(), &reason),
        None => reason,
    }
}

/// The money spent this month of `account`, from `own`, what was read of
/// it: from the bill of the organization when it was read, else as the
/// provider counted it for the account's key, else what the agent's
/// sessions spent through it at API prices, as its part counted them
/// (issue 22 of spec 2026-11-23-accounts-apart-from-profiles); with none,
/// only why the bill could not be read with the admin key, if it could
/// not. The bill is the row "Organization bill", which waits for an
/// admin key on an organization whose bill was not read (issue 24).
fn spent_money(
    state: &AgentsState,
    account: &Account,
    own: Option<&AccountRead>,
    now: SystemTime,
) -> Option<MoneyViewModel> {
    let account_bill = own.map(AccountRead::bill);
    let bill =
        account_bill.and_then(|bill| bill.answer().zip(bill.reads().read_at()));
    let bill_failure =
        account_bill.and_then(|bill| bill.failure()).map(|error| {
            i18n::with_error(
                Key::TplAgentsBillReadFailed,
                &source_error_text(error),
            )
        });

    let counted = own.and_then(|own| {
        let (balance, read_at) = own.balance()?;
        Some((balance, balance.spent_this_month()?, read_at))
    });

    let mut vm = MoneyViewModel {
        updated: None,
        rows: Vec::new(),
        spent: None,
        spent_at_api_prices: None,
        organization_bill: None,
        is_stale: false,
        failure: None,
    };
    let read_at = match (bill, counted) {
        (Some((bill, read_at)), _) => {
            let amount = dollars_text(bill.this_month());
            vm.organization_bill =
                Some(OrganizationBillViewModel::Read(amount));
            read_at
        },
        (None, Some((balance, spent, read_at))) => {
            match balance.spent_by_kind() {
                Some(by_kind) => {
                    vm.spent_at_api_prices = Some(spent_at_api_prices(
                        state, account, spent, by_kind,
                    )?);
                },
                None => vm.spent = Some(amount_text(spent)),
            }
            read_at
        },
        (None, None) => {
            vm.failure = Some(bill_failure?);
            return Some(vm);
        },
    };

    if vm.organization_bill.is_none()
        && account.kind() == AccountKind::Organization
    {
        vm.organization_bill =
            Some(OrganizationBillViewModel::WaitsForAdminKey);
    }
    vm.updated = Some(updated_text(now, read_at));
    Some(vm)
}

/// What the agent's sessions spent through `account` this month, `spent`
/// made of `by_kind` at API prices, with the mark of the list prices read
/// and the agent of the first profile the account was found in; `None`
/// before the prices are read, as the part could not have priced it.
fn spent_at_api_prices(
    state: &AgentsState,
    account: &Account,
    spent: &Amount,
    by_kind: &MoneyByKind,
) -> Option<SpentAtApiPricesViewModel> {
    let profiles = state.profiles();
    let prices = profiles.prices().last_good()?;
    let agent = account.found_in().iter().find_map(|place| match place {
        FoundIn::ProfileFolder { agent, .. } => Some(state.agent_name(agent)),
        _ => None,
    });

    Some(SpentAtApiPricesViewModel::of(
        &amount_text(spent),
        by_kind,
        agent,
        prices,
        profiles.time_zone(),
    ))
}

#[cfg(test)]
mod tests {
    use otty_agents::{
        Billing, Credits, DaySpend, Limits, ModelTokens, ProviderRef,
        SavedProfiles, Spend, TokenCount,
    };

    use super::*;
    use crate::reducer::reduce;
    use crate::testing::{
        at, bill_loaded, bill_of, chatgpt_login, claude, codex, connected,
        five_hour_limits, kind_line, limits_numbers, now, numbers_loaded,
        organization, price_table, saved_with_given_key, set_prices,
        spend_loaded, with_accounts,
    };
    use crate::types::SpendCount;

    /// Codex logged in to the [`chatgpt_login`], with `limits` read at
    /// `at(10)`.
    fn chatgpt_with(limits: Limits) -> AgentsState {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            numbers_loaded(
                &chatgpt_login(),
                Ok(limits_numbers(limits)),
                at(10),
            ),
        );

        state
    }

    /// Two million input tokens of Claude Sonnet sent to Anthropic today,
    /// 2026-11-16: $6.00 at the made-up prices.
    fn anthropic_spend() -> Spend {
        let tokens = ModelTokens::new(
            Some("claude-sonnet-4-5"),
            TokenCount::new(2_000_000, 0, 0),
            1,
        )
        .with_provider(Some(ProviderRef::of_id("anthropic")));

        Spend::new(
            vec![DaySpend::new(jiff::civil::date(2026, 11, 16), vec![tokens])],
            Vec::new(),
        )
    }

    /// Money spent this month, such as a bill.
    fn exact(amount: &str) -> Option<String> {
        Some(String::from(amount))
    }

    #[test]
    fn a_subscription_shows_its_windows_in_a_limits_block() {
        let state = chatgpt_with(five_hour_limits(42.0));

        let numbers =
            AccountNumbersViewModel::of(&state, &chatgpt_login(), now());

        let Some(AccountNumbersViewModel::Limits(limits)) = numbers else {
            panic!("a subscription has limits");
        };
        assert_eq!(limits.windows.len(), 1);
        assert_eq!(
            limits.windows[0].used,
            i18n::with_count(Key::TplAgentsPercentUsed, 42)
        );
        assert_eq!(limits.updated, Some(updated_text(now(), at(10))));
        assert!(!limits.is_stale);
    }

    /// Spec 2026-11-19, Decision 9: the last good numbers stay, greyed,
    /// and the need of the page says what to do, not the block.
    #[test]
    fn a_failed_read_keeps_the_last_limits_greyed() {
        let mut state = chatgpt_with(five_hour_limits(42.0));
        let _ = reduce(
            &mut state,
            numbers_loaded(
                &chatgpt_login(),
                Err(crate::testing::login_expired()),
                at(20),
            ),
        );

        let numbers =
            AccountNumbersViewModel::of(&state, &chatgpt_login(), now());

        let Some(AccountNumbersViewModel::Limits(limits)) = numbers else {
            panic!("the last limits stay");
        };
        assert!(limits.is_stale);
        assert_eq!(limits.windows.len(), 1);
    }

    #[test]
    fn credits_show_in_a_credits_block() {
        let state = chatgpt_with(Limits::new(
            Billing::Credits(Credits::new(
                None,
                Some(1240.0),
                Some(5000.0),
                None,
            )),
            None,
        ));

        let numbers =
            AccountNumbersViewModel::of(&state, &chatgpt_login(), now());

        let title = numbers.as_ref().map(AccountNumbersViewModel::title);
        let Some(AccountNumbersViewModel::Credits(limits)) = numbers else {
            panic!("the account is paid for by credits");
        };
        let credits = limits.credits.expect("the credits show");
        assert_eq!(
            credits.value,
            i18n::filled(
                Key::TplAgentsCreditsUsedOf,
                &[("used", "1 240"), ("pool", "5 000")]
            )
        );
        assert_eq!(title, Some(i18n::t(Key::AgentsFieldCredits)));
    }

    #[test]
    fn an_organization_shows_the_money_of_its_bill_this_month() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            saved_with_given_key(),
        );
        let _ = reduce(&mut state, bill_loaded(Ok(bill_of(40, 2))));

        let numbers =
            AccountNumbersViewModel::of(&state, &organization(), now());

        assert_eq!(
            numbers,
            Some(AccountNumbersViewModel::Money(MoneyViewModel {
                updated: Some(updated_text(now(), now())),
                rows: Vec::new(),
                spent: None,
                spent_at_api_prices: None,
                organization_bill: Some(OrganizationBillViewModel::Read(
                    String::from("$42.00")
                )),
                is_stale: false,
                failure: None,
            }))
        );
    }

    /// The money of a key that its part counted from the agent's sessions
    /// at API prices, $42.10 this month: $30 of input, $10 written to the
    /// cache and $2.10 read from it.
    fn spent_at_api_prices() -> Balance {
        let micro = otty_agents::Dollars::from_micro_dollars;
        let by_kind = otty_agents::MoneyByKind::new(
            micro(30_000_000),
            micro(10_000_000),
            micro(2_100_000),
            micro(0),
        );

        Balance::new(None, None)
            .with_spent_this_month(Amount::new(42.1, "USD"))
            .with_spent_by_kind(Some(by_kind))
    }

    /// Issue 22 of spec 2026-11-23: a key whose provider tells no spend
    /// shows what its sessions on this computer spent this month at API
    /// prices, marked so, with its kinds for the tooltip and where it was
    /// counted.
    #[test]
    fn a_key_shows_its_spend_at_api_prices_from_its_sessions() {
        let key = crate::testing::found_key("anthropic");
        let numbers =
            otty_agents::AccountNumbers::Balance(spent_at_api_prices());
        let mut state = crate::testing::with_key_read(&key, Ok(numbers));
        set_prices(&mut state, price_table());

        let numbers = AccountNumbersViewModel::of(&state, &key, now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the money of the key: {numbers:?}");
        };
        assert_eq!(money.spent, None, "no exact money");
        assert_eq!(money.organization_bill, None, "a key has no bill");
        let spent = money.spent_at_api_prices.expect("the spend at API prices");
        assert_eq!(spent.amount, "~$42.10");
        assert_eq!(
            spent.kinds,
            [
                kind_line(Key::AgentsKindInput, "~$30.00"),
                kind_line(Key::AgentsKindCacheWritten, "~$10.00"),
                kind_line(Key::AgentsKindCached, "~$2.10"),
            ]
        );
        assert_eq!(spent.mark.label, i18n::t(Key::AgentsApiPricesMark));
        let agent = state.agent_name(&crate::testing::claude_id()).to_string();
        assert_eq!(
            spent.source,
            Some(i18n::filled(
                Key::TplAgentsSpentFromSessions,
                &[("agent", &agent)]
            ))
        );
        assert_eq!(money.updated, Some(updated_text(now(), at(10))));
    }

    /// Issue 22: an organization whose bill is read with an admin key
    /// shows its bill, not the spend of its sessions.
    #[test]
    fn the_bill_comes_before_the_spend_at_api_prices() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            saved_with_given_key(),
        );
        let numbers =
            otty_agents::AccountNumbers::Balance(spent_at_api_prices());
        let _ = reduce(
            &mut state,
            numbers_loaded(&organization(), Ok(numbers), at(10)),
        );
        let _ = reduce(&mut state, bill_loaded(Ok(bill_of(40, 2))));
        set_prices(&mut state, price_table());

        let numbers =
            AccountNumbersViewModel::of(&state, &organization(), now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the money of the organization: {numbers:?}");
        };
        assert_eq!(money.spent, None);
        assert_eq!(
            money.organization_bill,
            Some(OrganizationBillViewModel::Read(String::from("$42.00")))
        );
        assert_eq!(money.spent_at_api_prices, None);
    }

    /// Issue 24 of spec 2026-11-23: an organization with no admin key
    /// shows what its sessions spent and the row of its bill, which waits
    /// for an admin key.
    #[test]
    fn an_organization_with_no_admin_key_shows_its_spend_and_waits_for_its_bill()
     {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            SavedProfiles::default(),
        );
        let numbers =
            otty_agents::AccountNumbers::Balance(spent_at_api_prices());
        let _ = reduce(
            &mut state,
            numbers_loaded(&organization(), Ok(numbers), at(10)),
        );
        set_prices(&mut state, price_table());

        let numbers =
            AccountNumbersViewModel::of(&state, &organization(), now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the money of the organization: {numbers:?}");
        };
        assert!(money.spent_at_api_prices.is_some(), "{money:?}");
        assert_eq!(
            money.organization_bill,
            Some(OrganizationBillViewModel::WaitsForAdminKey)
        );
    }

    /// Spec 2026-11-23, Decision 1: with no bill, the tokens of the
    /// profiles the organization was found in do not make its money: it
    /// has no numbers, and no estimate.
    #[test]
    fn with_no_bill_the_tokens_of_its_profiles_make_no_money() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            SavedProfiles::default(),
        );
        set_prices(&mut state, price_table());
        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                SpendCount::Counted(anthropic_spend()),
                at(10),
            ),
        );

        let numbers =
            AccountNumbersViewModel::of(&state, &organization(), now());

        assert_eq!(numbers, None);
    }

    /// Issue 19 of spec 2026-11-19-agent-plugins: a bill that could not be
    /// read with the admin key says why, with no sum.
    #[test]
    fn a_bill_that_failed_says_why() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            saved_with_given_key(),
        );
        let _ = reduce(
            &mut state,
            bill_loaded(Err(crate::testing::refused_key_need())),
        );

        let numbers =
            AccountNumbersViewModel::of(&state, &organization(), now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the failure: {numbers:?}");
        };
        assert_eq!(money.spent, None);
        assert_eq!(
            money.failure,
            Some(i18n::with_error(
                Key::TplAgentsBillReadFailed,
                "The key was refused"
            ))
        );
    }

    #[test]
    fn an_account_with_nothing_read_has_no_numbers() {
        let state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        let organization_state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            SavedProfiles::default(),
        );

        assert_eq!(
            AccountNumbersViewModel::of(&state, &chatgpt_login(), now()),
            None
        );
        assert_eq!(
            AccountNumbersViewModel::of(
                &organization_state,
                &organization(),
                now()
            ),
            None
        );
    }

    fn balance(
        left: Option<otty_agents::Amount>,
        key_limit: Option<otty_agents::KeyLimit>,
    ) -> otty_agents::AccountNumbers {
        otty_agents::AccountNumbers::Balance(Balance::new(left, key_limit))
    }

    fn row(key: Key, used: &str, percent: Option<u8>) -> UsageRowViewModel {
        UsageRowViewModel {
            name: i18n::t(key).to_string(),
            used: String::from(used),
            percent,
            resets: None,
        }
    }

    /// The owner's check: a DeepSeek key shows Left.
    #[test]
    fn a_key_shows_the_money_left_in_its_currency() {
        let deepseek = crate::testing::found_key("deepseek");
        let state = crate::testing::with_key_read(
            &deepseek,
            Ok(balance(Amount::new(110.0, "CNY"), None)),
        );

        let numbers = AccountNumbersViewModel::of(&state, &deepseek, now());

        assert_eq!(
            numbers,
            Some(AccountNumbersViewModel::Money(MoneyViewModel {
                updated: Some(updated_text(now(), at(10))),
                rows: vec![row(Key::AgentsLeft, "¥110.00", None)],
                spent: None,
                spent_at_api_prices: None,
                organization_bill: None,
                is_stale: false,
                failure: None,
            }))
        );
    }

    #[test]
    fn an_openrouter_key_shows_its_limit_with_a_bar() {
        let openrouter = crate::testing::found_key("openrouter");
        let limited = otty_agents::KeyLimit::new(
            Amount::new(12.3, "USD").expect("a number"),
            Amount::new(50.0, "USD"),
        );
        let state = crate::testing::with_key_read(
            &openrouter,
            Ok(balance(None, Some(limited))),
        );

        let numbers = AccountNumbersViewModel::of(&state, &openrouter, now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the money of the key: {numbers:?}");
        };
        let used = i18n::filled(
            Key::TplAgentsKeyLimitUsed,
            &[("used", "$12.30"), ("limit", "$50.00")],
        );
        assert_eq!(money.rows, [row(Key::AgentsKeyLimit, &used, Some(25))]);
    }

    #[test]
    fn a_failed_read_with_nothing_before_says_why() {
        let openrouter = crate::testing::found_key("openrouter");
        let state = crate::testing::with_key_read(
            &openrouter,
            Err(crate::testing::refused_key_need()),
        );

        let numbers = AccountNumbersViewModel::of(&state, &openrouter, now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the failure: {numbers:?}");
        };
        assert!(money.rows.is_empty());
        assert!(!money.is_stale, "nothing to grey");
        assert!(
            money
                .failure
                .as_deref()
                .is_some_and(|failure| failure.contains("OpenRouter")),
            "{money:?}"
        );
    }

    /// Issue 15 of spec 2026-11-19-agent-plugins: a key whose provider
    /// tells the money spent this month and the credits of the account
    /// shows both, as the provider counted them.
    #[test]
    fn a_key_shows_the_money_spent_this_month_and_its_credits() {
        let deepseek = crate::testing::found_key("deepseek");
        let numbers = otty_agents::AccountNumbers::Balance(
            Balance::new(Amount::new(110.0, "USD"), None)
                .with_spent_this_month(Amount::new(4.2, "USD"))
                .with_credits(Some(Credits::new(
                    Some(3760.0),
                    None,
                    None,
                    None,
                ))),
        );
        let state = crate::testing::with_key_read(&deepseek, Ok(numbers));

        let numbers = AccountNumbersViewModel::of(&state, &deepseek, now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the money of the key: {numbers:?}");
        };
        let credits = crate::model::limits::credits_left(&Credits::new(
            Some(3760.0),
            None,
            None,
            None,
        ))
        .expect("credits left");
        assert_eq!(
            money.rows,
            [
                row(Key::AgentsLeft, "$110.00", None),
                row(Key::AgentsFieldCredits, &credits, None),
            ]
        );
        assert_eq!(money.spent, exact("$4.20"));
        assert_eq!(money.updated, Some(updated_text(now(), at(10))));
    }
}
