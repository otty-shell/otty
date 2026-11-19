//! The numbers block of an account page (spec 2026-11-19, Decision 5),
//! named after what the account has: Limits (usage windows of a
//! subscription), Credits, or Money (Left, the money left on the account;
//! Key limit, of an OpenRouter key; Spent this month, from the bill or an
//! estimate marked "~"). There is no budget: OTTY only shows. The last good
//! numbers stay, greyed, when a later read fails.

use std::time::SystemTime;

use otty_agents::{
    Account, AccountKind, AccountMoney, Amount, Balance, KeyLimit,
    MoneyEstimate, SourceError, Spend, own_source_of,
};
use otty_app_i18n::{self as i18n, Key};

use super::super::failure_text::{
    named_failure, source_error_text, source_name,
};
use super::super::own_reads::OwnRead;
use super::super::state::AgentsState;
use super::UNKNOWN;
use super::limits::{LimitsViewModel, UsageRowViewModel, updated_text};
use super::money::MoneyViewModel;
use super::numbers::{dollars_text, money_text};
use super::tokens::count_failed_text;

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
            AccountKind::Money => money(state, account, now).map(Self::Money),
            _ => limits(state, account, now),
        }
    }
}

/// The estimate of the money `account` spent, at the list prices, over the
/// tokens the profiles it was found in sent to its provider (spec
/// 2026-11-19, Decision 4); `None` before the prices were read or any of
/// those tokens were counted.
pub(super) fn account_estimate(
    state: &AgentsState,
    account: &Account,
) -> Option<MoneyEstimate> {
    let profiles = state.profiles();
    let prices = profiles.prices().table()?;
    let spends: Vec<(&Spend, &[Account])> = state
        .profiles()
        .found_with(account)
        .filter_map(|shown| {
            Some((shown.spend().last_good()?.0, shown.accounts()))
        })
        .collect();
    if spends.is_empty() {
        return None;
    }

    match AccountMoney::of(account, None, &spends, prices) {
        AccountMoney::Estimate(estimate) => Some(estimate),
        _ => None,
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
    let mut limits = LimitsViewModel::from_limits(
        &account_limits,
        now,
        profiles.time_zone(),
    )?;
    limits.log_in_hint = None;

    if account.kind() == AccountKind::Credits || limits.credits.is_some() {
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
    let own = state.profiles().own_reads().of(account);
    let spent = spent_money(state, account, own, now);
    let Some(own) = own else {
        return spent;
    };

    let mut rows = balance_rows(own.balance().map(|(balance, _)| balance));
    let own_read_at = own
        .balance()
        .map(|(_, read_at)| read_at)
        .or(own.bill().map(|(_, read_at)| read_at));
    let failure = own.failure().map(|error| own_failure_text(account, error));
    if rows.is_empty() && spent.is_none() && failure.is_none() {
        return None;
    }

    let mut vm = spent.unwrap_or(MoneyViewModel {
        updated: None,
        rows: Vec::new(),
        note: None,
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
/// the account, and Key limit, what the key used of its limit.
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

    left.into_iter().chain(key_limit).collect()
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

/// Why the own read of `account` failed: its source and the reason, such
/// as "DeepSeek balance: the key was refused".
fn own_failure_text(account: &Account, error: &SourceError) -> String {
    let reason = source_error_text(error);

    match own_source_of(account) {
        Some(id) => named_failure(source_name(id), &reason),
        None => reason,
    }
}

/// The money spent this month: from the bill when one of the account's
/// profiles read it, or the account's own admin key did, else the money of
/// the tokens of its profiles: the cost their agent wrote, or an estimate
/// marked with "~".
fn spent_money(
    state: &AgentsState,
    account: &Account,
    own: Option<&OwnRead>,
    now: SystemTime,
) -> Option<MoneyViewModel> {
    let bill = state
        .profiles()
        .shown()
        .iter()
        .filter(|shown| shown.pays_through(account))
        .find_map(|shown| {
            let bill = shown.bill();
            bill.answer().zip(bill.reads().read_at())
        })
        .or_else(|| own?.bill());

    match bill {
        Some((bill, read_at)) => Some(MoneyViewModel {
            updated: Some(updated_text(now, read_at)),
            rows: spent_this_month(dollars_text(bill.this_month())),
            note: None,
            is_stale: false,
            failure: None,
        }),
        None => estimated_money(state, account, now),
    }
}

/// The estimate of the money spent this month, as old as the latest count
/// of the tokens of the account's profiles; greyed, with why, when a count
/// failed. A dash while there are no prices.
fn estimated_money(
    state: &AgentsState,
    account: &Account,
    now: SystemTime,
) -> Option<MoneyViewModel> {
    let counted_at = state
        .profiles()
        .found_with(account)
        .filter_map(|shown| shown.spend().last_good().map(|(_, at)| at))
        .max();
    let failure = state
        .profiles()
        .found_with(account)
        .find_map(|shown| shown.spend().failure())
        .map(count_failed_text);

    let Some(counted_at) = counted_at else {
        return failure.map(|failure| MoneyViewModel {
            updated: None,
            rows: Vec::new(),
            note: None,
            is_stale: false,
            failure: Some(failure),
        });
    };

    let prices = state.profiles().prices();
    let (amount, note) = match account_estimate(state, account) {
        Some(estimate) => {
            let month = estimate.this_month();
            let dollars = dollars_text(month.cost());
            let amount = if month.is_estimate() {
                i18n::filled(Key::TplAgentsAboutAmount, &[("amount", &dollars)])
            } else {
                dollars
            };
            let unpriced = month.unpriced_requests();
            let note = (unpriced > 0).then(|| {
                i18n::with_count(Key::TplAgentsMoneyUnpriced, unpriced)
            });
            (amount, note)
        },
        None => {
            let note = prices
                .has_failed()
                .then(|| i18n::t(Key::AgentsNoPrices).to_string());
            (String::from(UNKNOWN), note)
        },
    };

    Some(MoneyViewModel {
        updated: Some(updated_text(now, counted_at)),
        rows: spent_this_month(amount),
        note,
        is_stale: failure.is_some(),
        failure,
    })
}

/// The row "Spent this month" with its `amount`.
fn spent_this_month(amount: String) -> Vec<UsageRowViewModel> {
    vec![UsageRowViewModel {
        name: i18n::t(Key::AgentsSpentThisMonth).to_string(),
        used: amount,
        percent: None,
        resets: None,
    }]
}

#[cfg(test)]
mod tests {
    use otty_agents::{
        Billing, Credits, DaySpend, Limits, ModelTokens, Provider,
        SavedProfiles, SourceError, SourceId, Spend, TokenCount,
    };

    use super::*;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, bill_loaded, bill_of, chatgpt_login, claude, codex,
        connected, five_hour_limits, limits_answer, limits_loaded, now,
        organization, price_table, saved_with_admin_key, set_prices,
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
            limits_loaded(
                &codex(),
                limits_answer(vec![(SourceId::ChatGptUsageApi, Ok(limits))]),
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
        .with_provider(Some(Provider::Anthropic));

        Spend::new(
            vec![DaySpend::new(jiff::civil::date(2026, 11, 16), vec![tokens])],
            Vec::new(),
        )
    }

    fn spent_this_month(amount: String) -> Vec<UsageRowViewModel> {
        vec![UsageRowViewModel {
            name: i18n::t(Key::AgentsSpentThisMonth).to_string(),
            used: amount,
            percent: None,
            resets: None,
        }]
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
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    SourceId::ChatGptUsageApi,
                    Err(SourceError::NotLoggedIn),
                )]),
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
        assert_eq!(limits.log_in_hint, None);
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
            &[(claude(), api_key(), vec![organization()])],
            saved_with_admin_key(),
        );
        let _ = reduce(&mut state, bill_loaded(&claude(), Ok(bill_of(40, 2))));

        let numbers =
            AccountNumbersViewModel::of(&state, &organization(), now());

        assert_eq!(
            numbers,
            Some(AccountNumbersViewModel::Money(MoneyViewModel {
                updated: Some(updated_text(now(), now())),
                rows: spent_this_month(String::from("$42.00")),
                note: None,
                is_stale: false,
                failure: None,
            }))
        );
    }

    /// With no bill the money is an estimate over the tokens its profiles
    /// sent to its provider, marked with "~".
    #[test]
    fn with_no_bill_the_money_is_an_estimate_marked_with_a_tilde() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
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

        let about =
            i18n::filled(Key::TplAgentsAboutAmount, &[("amount", "$6.00")]);
        assert_eq!(
            numbers,
            Some(AccountNumbersViewModel::Money(MoneyViewModel {
                updated: Some(updated_text(now(), at(10))),
                rows: spent_this_month(about),
                note: None,
                is_stale: false,
                failure: None,
            }))
        );
        assert_eq!(
            numbers.map(|numbers| numbers.title()),
            Some(i18n::t(Key::AgentsNumberMoney))
        );
    }

    #[test]
    fn an_account_with_nothing_read_has_no_numbers() {
        let state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        let organization_state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
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

    /// A made-up account the user added at `provider`, its key in a
    /// variable.
    fn added(provider: Provider) -> Account {
        Account::added(
            provider,
            otty_agents::AdminKeyRef::EnvVar(String::from("MADE_UP_KEY")),
        )
    }

    /// Codex with its login, and `account` added by hand, whose own read
    /// answered `numbers` at `at(10)`.
    fn added_with(
        account: &Account,
        numbers: Result<otty_agents::AccountNumbers, SourceError>,
    ) -> AgentsState {
        let saved = SavedProfiles::default().with_account_added(
            account.provider(),
            account.added_key().cloned().expect("an added account"),
        );
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            saved,
        );
        let _ = reduce(
            &mut state,
            crate::event::AgentsIntent::AccountNumbersLoaded(
                crate::types::LoadedAccountNumbers {
                    account: account.clone(),
                    numbers,
                    read_at: at(10),
                },
            ),
        );

        state
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

    /// The owner's check: a DeepSeek key added by hand shows Left.
    #[test]
    fn an_added_key_shows_the_money_left_in_its_currency() {
        let deepseek = added(Provider::DeepSeek);
        let state =
            added_with(&deepseek, Ok(balance(Amount::new(110.0, "CNY"), None)));

        let numbers = AccountNumbersViewModel::of(&state, &deepseek, now());

        assert_eq!(
            numbers,
            Some(AccountNumbersViewModel::Money(MoneyViewModel {
                updated: Some(updated_text(now(), at(10))),
                rows: vec![row(Key::AgentsLeft, "¥110.00", None)],
                note: None,
                is_stale: false,
                failure: None,
            }))
        );
    }

    #[test]
    fn an_openrouter_key_shows_its_limit_with_a_bar() {
        let openrouter = added(Provider::OpenRouter);
        let limited = otty_agents::KeyLimit::new(
            Amount::new(12.3, "USD").expect("a number"),
            Amount::new(50.0, "USD"),
        );
        let state = added_with(&openrouter, Ok(balance(None, Some(limited))));

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

    /// An added organization reads its bill with its own admin key.
    #[test]
    fn an_added_organization_shows_its_bill() {
        let openai = added(Provider::OpenAI);
        let state = added_with(
            &openai,
            Ok(otty_agents::AccountNumbers::Bill(bill_of(1, 2))),
        );

        let numbers = AccountNumbersViewModel::of(&state, &openai, now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the bill: {numbers:?}");
        };
        assert_eq!(
            money.rows,
            spent_this_month(dollars_text(bill_of(1, 2).this_month()))
        );
    }

    #[test]
    fn a_failed_read_with_nothing_before_says_why() {
        let deepseek = added(Provider::DeepSeek);
        let state = added_with(&deepseek, Err(SourceError::KeyRefused));

        let numbers = AccountNumbersViewModel::of(&state, &deepseek, now());

        let Some(AccountNumbersViewModel::Money(money)) = numbers else {
            panic!("the failure: {numbers:?}");
        };
        assert!(money.rows.is_empty());
        assert!(!money.is_stale, "nothing to grey");
        assert!(
            money.failure.as_deref().is_some_and(|failure| failure
                .contains(i18n::t(Key::AgentsSourceDeepseekBalance))),
            "{money:?}"
        );
    }
}
