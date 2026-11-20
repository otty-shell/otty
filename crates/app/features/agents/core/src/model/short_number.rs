//! The short numbers of the rows of the left panel and of the All
//! accounts and All profiles tables (spec Decision 1): each number with
//! its word. An account shows what is used, left or spent of it ("62 %
//! used", "3 760 left", "¥110.00 left", "$12.30 used", "~ $42.10 this
//! month"); a profile shows the tokens
//! it spent today ("30.6M today", "no sessions"), or the state that needs
//! the user.

use otty_agents::{Account, AccountKind, Billing, Dollars};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::account_numbers::{account_estimate, amount_text};
use super::limits::credits_left;
use super::numbers::{dollars_text, short_count};
use super::user_need::UserNeed;

/// From this percent on, the most used window is drawn in the waiting
/// color: the window is nearly used up.
const NEARLY_USED: u8 = 80;

/// The number of the profile `shown` and its color: the state that needs
/// the user ("needs login", "program not found", "format changed") in the
/// error color, "no sessions" when its agent has none, else the tokens it
/// spent today ("1.8M today"). `None` while nothing is known yet.
pub(crate) fn profile_number(
    shown: &ProfileState,
) -> Option<(String, SessionTone)> {
    if let Some(need) = UserNeed::of(shown.status()) {
        return Some((need.state().to_string(), SessionTone::Failed));
    }
    if shown.spend().format_change().is_some() {
        let changed = i18n::t(Key::AgentsSourceFormatChanged).to_string();
        return Some((changed, SessionTone::Failed));
    }
    if shown.sessions().is_some_and(<[_]>::is_empty) {
        let none = i18n::t(Key::AgentsNoSessions).to_string();
        return Some((none, SessionTone::Muted));
    }

    let (spend, _) = shown.spend().last_good()?;
    let today = short_count(spend.today().total());

    Some((
        i18n::filled(Key::TplAgentsAmountToday, &[("amount", &today)]),
        SessionTone::Muted,
    ))
}

/// The number of `account` and its color: "login expired", the most used
/// window of a subscription ("42 % used"), the credits left ("3 760 left"),
/// the money left its key told ("¥110.00 left") or what a key with a limit
/// used ("$12.30 used"), or the money spent this month by the bill
/// ("$44.87 this month") or by an estimate over the tokens of its profiles
/// ("~ $42.10 this month"), or "hidden" for an account the user hid, which
/// is not read. `None` while nothing is known yet.
pub(crate) fn account_number(
    state: &AgentsState,
    account: &Account,
) -> Option<(String, SessionTone)> {
    if state.profiles().is_account_hidden(account) {
        return Some((hidden_word(), SessionTone::Muted));
    }

    match account.kind() {
        AccountKind::Money => balance_now(state, account)
            .or_else(|| money_this_month(state, account)),
        _ => limits_now(state, account),
    }
}

/// The word of a hidden account or profile in the lists: "hidden".
pub(crate) fn hidden_word() -> String {
    i18n::t(Key::AgentsHidden).to_string()
}

/// The most used window or the credits left of a subscription, as the
/// latest good read through one of its profiles, or with its own login,
/// says, or "login expired" in the error color when the latest read could
/// not use the login.
fn limits_now(
    state: &AgentsState,
    account: &Account,
) -> Option<(String, SessionTone)> {
    let limits = state.profiles().account_limits(account)?;
    if limits.is_login_expired() {
        let expired = i18n::t(Key::AgentsSourceLoginExpired).to_string();
        return Some((expired, SessionTone::Failed));
    }

    let (answer, _) = limits.last_good()?;
    match answer.value().billing() {
        Billing::UsageWindows(usage) => {
            let percent = usage.most_used()?.percent_used().rounded();
            let tone = if percent >= NEARLY_USED {
                SessionTone::Waiting
            } else {
                SessionTone::Muted
            };

            Some((
                i18n::with_count(Key::TplAgentsPercentUsed, u64::from(percent)),
                tone,
            ))
        },
        Billing::Credits(credits) => {
            Some((credits_left(credits)?, SessionTone::Muted))
        },
        _ => None,
    }
}

/// What the key of an account told at its last good read: the money left
/// on it ("¥110.00 left"), else what a key with a limit used ("$12.30
/// used").
fn balance_now(
    state: &AgentsState,
    account: &Account,
) -> Option<(String, SessionTone)> {
    let (balance, _) = state.profiles().own_reads().of(account)?.balance()?;

    if let Some(left) = balance.left() {
        let amount = amount_text(left);
        let text =
            i18n::filled(Key::TplAgentsCreditsLeft, &[("amount", &amount)]);
        return Some((text, SessionTone::Muted));
    }

    let used = amount_text(balance.key_limit()?.used());
    let text = i18n::filled(Key::TplAgentsAmountUsed, &[("amount", &used)]);
    Some((text, SessionTone::Muted))
}

/// The money an account paid per token spent this month: its bill when
/// one of its profiles, or its own admin key, read it, else the money of the tokens of the
/// profiles it was found in: the cost their agent wrote, else an estimate
/// at the list prices, with "~".
fn money_this_month(
    state: &AgentsState,
    account: &Account,
) -> Option<(String, SessionTone)> {
    let bill = state
        .profiles()
        .shown()
        .iter()
        .filter(|shown| shown.pays_through(account))
        .find_map(|shown| shown.bill().answer())
        .or_else(|| {
            let own = state.profiles().own_reads().of(account)?;
            own.bill().map(|(bill, _)| bill)
        });
    if let Some(bill) = bill {
        return Some(this_month(
            Key::TplAgentsAmountThisMonth,
            bill.this_month(),
        ));
    }

    let month = account_estimate(state, account)?.this_month();
    let key = if month.is_estimate() {
        Key::TplAgentsAboutThisMonth
    } else {
        Key::TplAgentsAmountThisMonth
    };

    Some(this_month(key, month.cost()))
}

/// `amount` written with the template `key`, such as "$44.87 this month".
fn this_month(key: Key, amount: Dollars) -> (String, SessionTone) {
    let amount = dollars_text(amount);

    (
        i18n::filled(key, &[("amount", &amount)]),
        SessionTone::Muted,
    )
}

#[cfg(test)]
mod tests {
    use otty_agents::fixtures::format_change;
    use otty_agents::{
        Credits, DaySpend, Limits, ModelTokens, ProfileStatus, ProviderRef,
        SavedProfiles, SourceError, Spend, TokenCount,
    };

    use super::*;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, bill_loaded, bill_of, chatgpt_login, chatgpt_login_in,
        claude, codex, codex_work, connected, five_hour_limits, limits_answer,
        limits_loaded, organization, price_table, read, saved_with_admin_key,
        session, set_prices, spend_loaded, spend_of, with_accounts,
        with_profiles,
    };
    use crate::types::SpendCount;

    fn credits(left: f64) -> Limits {
        Limits::new(
            Billing::Credits(Credits::new(Some(left), None, None, None)),
            None,
        )
    }

    /// Codex and Codex · Work, both logged in to the [`chatgpt_login`],
    /// with the answer `limits` read through Codex.
    fn chatgpt_with_limits(limits: Limits) -> AgentsState {
        let mut state = with_accounts(
            &[
                (
                    codex(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex")],
                ),
                (
                    codex_work(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex-work")],
                ),
            ],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    otty_agents::SourceId::ProviderPart,
                    Ok(limits),
                )]),
                at(10),
            ),
        );

        state
    }

    fn chatgpt_number(state: &AgentsState) -> Option<(String, SessionTone)> {
        account_number(state, &chatgpt_login())
    }

    fn profile_of(state: &AgentsState) -> Option<(String, SessionTone)> {
        profile_number(&state.profiles().shown()[0])
    }

    fn muted(text: String) -> Option<(String, SessionTone)> {
        Some((text, SessionTone::Muted))
    }

    /// The short number of the credits left, already written with its
    /// digit groups.
    fn credits_left(amount: &str) -> String {
        i18n::filled(Key::TplAgentsCreditsLeft, &[("amount", amount)])
    }

    /// The short number of what was spent today, already written short.
    fn today(amount: &str) -> String {
        i18n::filled(Key::TplAgentsAmountToday, &[("amount", amount)])
    }

    /// `amount` input tokens of Claude Sonnet sent to Anthropic today,
    /// 2026-11-16.
    fn anthropic_spend(input: u64) -> Spend {
        let tokens = ModelTokens::new(
            Some("claude-sonnet-4-5"),
            TokenCount::new(input, 0, 0),
            1,
        )
        .with_provider(Some(ProviderRef::of_id("anthropic")));
        let day = DaySpend::new(jiff::civil::date(2026, 11, 16), vec![tokens]);

        Spend::new(vec![day], Vec::new())
    }

    #[test]
    fn a_subscription_shows_its_most_used_window_with_its_word() {
        let state = chatgpt_with_limits(five_hour_limits(41.6));

        assert_eq!(
            chatgpt_number(&state),
            muted(i18n::with_count(Key::TplAgentsPercentUsed, 42))
        );
        assert!(
            i18n::with_count(Key::TplAgentsPercentUsed, 42).contains("used")
        );
    }

    #[test]
    fn a_window_nearly_used_up_is_drawn_in_the_waiting_color() {
        let state = chatgpt_with_limits(five_hour_limits(86.0));

        assert_eq!(
            chatgpt_number(&state),
            Some((
                i18n::with_count(Key::TplAgentsPercentUsed, 86),
                SessionTone::Waiting
            ))
        );
    }

    #[test]
    fn credits_show_what_is_left_in_groups_of_three_digits() {
        let state = chatgpt_with_limits(credits(1_234_567.4));
        let small = chatgpt_with_limits(credits(3760.0));

        assert_eq!(chatgpt_number(&state), muted(credits_left("1 234 567")));
        assert_eq!(chatgpt_number(&small), muted(credits_left("3 760")));
    }

    #[test]
    fn unlimited_credits_say_so() {
        let state = chatgpt_with_limits(Limits::new(
            Billing::Credits(Credits::unlimited()),
            None,
        ));

        assert_eq!(
            chatgpt_number(&state),
            muted(i18n::t(Key::AgentsCreditsUnlimited).to_string())
        );
    }

    #[test]
    fn an_organization_with_a_bill_shows_the_month_of_the_bill() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            saved_with_admin_key(),
        );
        let before = account_number(&state, &organization());

        let _ = reduce(&mut state, bill_loaded(&claude(), Ok(bill_of(40, 2))));

        assert_eq!(before, None, "nothing was read yet");
        assert_eq!(
            account_number(&state, &organization()),
            muted(i18n::filled(
                Key::TplAgentsAmountThisMonth,
                &[("amount", "$42.00")]
            ))
        );
    }

    /// With no bill the money is an estimate over the tokens its profiles
    /// sent to its provider, marked with "~".
    #[test]
    fn an_organization_with_no_bill_shows_an_estimate_of_the_month() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            SavedProfiles::default(),
        );
        set_prices(&mut state, price_table());

        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                SpendCount::Counted(anthropic_spend(2_000_000)),
                at(10),
            ),
        );

        assert_eq!(
            account_number(&state, &organization()),
            muted(i18n::filled(
                Key::TplAgentsAboutThisMonth,
                &[("amount", "$6.00")]
            ))
        );
    }

    /// An agent that writes what each answer cost: money made of that cost
    /// is exact, with no "~".
    #[test]
    fn money_the_agent_wrote_shows_with_no_tilde() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            SavedProfiles::default(),
        );
        set_prices(&mut state, price_table());
        let tokens = ModelTokens::new(
            Some("claude-sonnet-4-5"),
            TokenCount::new(2_000_000, 0, 0),
            1,
        )
        .with_provider(Some(ProviderRef::of_id("anthropic")))
        .with_cost(Some(Dollars::from_micro_dollars(1_500_000)));
        let day = DaySpend::new(jiff::civil::date(2026, 11, 16), vec![tokens]);
        let spend = Spend::new(vec![day], Vec::new());

        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), SpendCount::Counted(spend), at(10)),
        );

        assert_eq!(
            account_number(&state, &organization()),
            muted(i18n::filled(
                Key::TplAgentsAmountThisMonth,
                &[("amount", "$1.50")]
            ))
        );
    }

    /// Spec 2026-11-19, Decisions 1 and 3: a login that expired wins over
    /// the last good numbers, in the error color.
    #[test]
    fn an_expired_login_shows_login_expired_over_the_last_numbers() {
        let mut state = chatgpt_with_limits(five_hour_limits(42.0));

        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    otty_agents::SourceId::ProviderPart,
                    Err(SourceError::NotLoggedIn),
                )]),
                at(20),
            ),
        );

        assert_eq!(
            chatgpt_number(&state),
            Some((
                i18n::t(Key::AgentsSourceLoginExpired).to_string(),
                SessionTone::Failed
            ))
        );
    }

    #[test]
    fn an_account_with_nothing_read_shows_no_number() {
        let state = with_accounts(
            &[(
                codex(),
                connected(),
                vec![chatgpt_login_in("/home/user/.codex")],
            )],
            SavedProfiles::default(),
        );

        assert_eq!(chatgpt_number(&state), None);
    }

    #[test]
    fn a_profile_shows_the_tokens_it_spent_today() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let before = profile_of(&state);

        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                SpendCount::Counted(spend_of(1_800_000, 1)),
                at(10),
            ),
        );

        assert_eq!(before, None, "no tokens were counted yet");
        assert_eq!(profile_of(&state), muted(today("1.8M")));
    }

    /// Spec Decision 1: a profile whose agent has no sessions says so.
    #[test]
    fn a_profile_with_no_sessions_says_so() {
        let mut state = with_profiles(&[(claude(), connected())]);
        read(&mut state, &claude(), Vec::new());

        assert_eq!(
            profile_of(&state),
            muted(i18n::t(Key::AgentsNoSessions).to_string())
        );
    }

    #[test]
    fn a_profile_with_sessions_and_no_count_yet_shows_no_number() {
        let mut state = with_profiles(&[(claude(), connected())]);
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );

        assert_eq!(profile_of(&state), None);
    }

    #[test]
    fn a_state_that_needs_the_user_wins_in_the_error_color() {
        let cases = [
            (ProfileStatus::NotLoggedIn, Key::AgentsStateNeedsLogin),
            (
                ProfileStatus::ProgramNotFound,
                Key::AgentsStateProgramNotFound,
            ),
            (
                ProfileStatus::ConfigDirNotFound,
                Key::AgentsStateConfigDirNotFound,
            ),
        ];

        for (status, word) in cases {
            let mut state = with_profiles(&[(claude(), status)]);
            read(&mut state, &claude(), Vec::new());

            assert_eq!(
                profile_of(&state),
                Some((i18n::t(word).to_string(), SessionTone::Failed))
            );
        }
    }

    /// Spec 2026-11-19, Decisions 1 and 9: a profile whose session files
    /// OTTY no longer understands says "format changed", not fewer tokens.
    #[test]
    fn a_profile_whose_files_changed_shows_format_changed() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let error =
            SourceError::FormatChanged(Box::new(format_change(None, 40, 0)));
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );
        for (spend, counted_at) in [
            (SpendCount::Counted(spend_of(1_800_000, 1)), at(10)),
            (SpendCount::Failed(error), at(20)),
        ] {
            let _ =
                reduce(&mut state, spend_loaded(&claude(), spend, counted_at));
        }

        assert_eq!(
            profile_of(&state),
            Some((
                i18n::t(Key::AgentsSourceFormatChanged).to_string(),
                SessionTone::Failed
            ))
        );
    }

    /// Spec Decision 1: "¥110 left" in the lists for a key whose money
    /// left was read with its own key, "$12.30 used" for a key with a limit.
    #[test]
    fn a_key_shows_the_money_left_or_what_it_used() {
        let number = |provider: ProviderRef, numbers| {
            let key = otty_agents::AdminKeyRef::EnvVar(String::from("KEY"));
            let account = Account::added(provider.clone(), key.clone());
            let mut state = with_accounts(
                &[(codex(), connected(), vec![chatgpt_login()])],
                SavedProfiles::default().with_account_added(provider, key),
            );
            let _ = reduce(
                &mut state,
                crate::event::AgentsIntent::AccountNumbersLoaded(
                    crate::types::LoadedAccountNumbers {
                        account: account.clone(),
                        numbers: Ok(numbers),
                        read_at: at(10),
                    },
                ),
            );
            account_number(&state, &account)
        };
        let left = otty_agents::Balance::new(
            otty_agents::Amount::new(110.0, "CNY"),
            None,
        );
        let used = otty_agents::Balance::new(
            None,
            Some(otty_agents::KeyLimit::new(
                otty_agents::Amount::new(12.3, "USD").expect("a number"),
                None,
            )),
        );

        assert_eq!(
            number(
                ProviderRef::of_id("deepseek"),
                otty_agents::AccountNumbers::Balance(left)
            ),
            muted(credits_left("¥110.00"))
        );
        assert_eq!(
            number(
                ProviderRef::of_id("openrouter"),
                otty_agents::AccountNumbers::Balance(used)
            ),
            muted(i18n::filled(
                Key::TplAgentsAmountUsed,
                &[("amount", "$12.30")]
            ))
        );
    }
}
