//! The short numbers of the rows of the left panel and of the All
//! accounts and All profiles tables (spec Decision 1): each number with
//! its word. An account shows what is used, left or spent of it ("62 %
//! used", "3 760 left", "~ $42.10 this month"); a profile shows the tokens
//! it spent today ("30.6M today", "no sessions"), or the state that needs
//! the user.

use otty_agents::{Account, AccountKind, Billing, Dollars};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::account_numbers::account_estimate;
use super::limits::credits_left;
use super::numbers::{dollars_text, short_count};
use super::user_need::UserNeed;

/// From this percent on, the most used window is drawn in the waiting
/// color: the window is nearly used up.
const NEARLY_USED: u8 = 80;

/// The number of the profile `shown` and its color: the state that needs
/// the user ("needs login", "program not found") in the error color, "no
/// sessions" when its agent has none, else the tokens it spent today
/// ("1.8M today"). `None` while nothing is known yet.
pub(crate) fn profile_number(
    shown: &ProfileState,
) -> Option<(String, SessionTone)> {
    if let Some(need) = UserNeed::of(shown.status()) {
        return Some((need.state().to_string(), SessionTone::Failed));
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
/// window of a subscription ("42 % used"), the credits left ("3 760 left"), or the
/// money spent this month by the bill ("$44.87 this month") or by an
/// estimate over the tokens of its profiles ("~ $42.10 this month").
/// `None` while nothing is known yet.
pub(crate) fn account_number(
    state: &AgentsState,
    account: &Account,
) -> Option<(String, SessionTone)> {
    match account.kind() {
        AccountKind::Money => money_this_month(state, account),
        _ => limits_now(state, account),
    }
}

/// The most used window or the credits left of a subscription, as the
/// latest good read through one of its profiles says, or "login expired"
/// in the error color when the latest read could not use the login.
fn limits_now(
    state: &AgentsState,
    account: &Account,
) -> Option<(String, SessionTone)> {
    let shown = state.profiles().with_limits_of(account)?;
    if shown.limits().is_login_expired() {
        let expired = i18n::t(Key::AgentsSourceLoginExpired).to_string();
        return Some((expired, SessionTone::Failed));
    }

    let (answer, _) = shown.limits().last_good()?;
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

/// The money an account paid per token spent this month: its bill when
/// one of its profiles read it, else an estimate at the list prices over
/// the tokens of the profiles it was found in.
fn money_this_month(
    state: &AgentsState,
    account: &Account,
) -> Option<(String, SessionTone)> {
    let bill = state
        .profiles()
        .shown()
        .iter()
        .filter(|shown| shown.pays_through(account))
        .find_map(|shown| shown.bill().answer());
    if let Some(bill) = bill {
        return Some(this_month(
            Key::TplAgentsAmountThisMonth,
            bill.this_month(),
        ));
    }

    let estimate = account_estimate(state, account)?;

    Some(this_month(
        Key::TplAgentsAboutThisMonth,
        estimate.this_month().cost(),
    ))
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
    use otty_agents::{
        Credits, DaySpend, Limits, ModelTokens, ProfileStatus, Provider,
        SavedProfiles, SourceError, SourceId, Spend, TokenCount,
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
                limits_answer(vec![(SourceId::ChatGptUsageApi, Ok(limits))]),
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
        .with_provider(Some(Provider::Anthropic));
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
                    SourceId::ChatGptUsageApi,
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
}
