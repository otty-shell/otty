//! The short number on the right of a profile row in the left panel (spec
//! Decisions 6 and 9): what the billing of the profile says in a few
//! characters, or the state that needs the user.

use otty_agents::{Billing, BillingKind, Credits, ProfileStatus};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::state::ProfileState;
use super::numbers::{credits_text, dollars_text, short_count};

/// From this percent on, the most used window is drawn in the waiting
/// color: the window is nearly used up.
const NEARLY_USED: u8 = 80;

/// The short number of `shown` and its color: the state that needs the
/// user ("needs login", "program not found") in the error color, else the
/// number of its billing: the most used window of a subscription ("42 %"),
/// the credits left ("3 760 left"), the tokens spent today when the plan
/// tells no number of credits ("1.8M today"), or the money an API key
/// profile spent today by its own estimate ("$12.40 today"). `None` while
/// nothing is known yet.
pub(crate) fn short_number(
    shown: &ProfileState,
) -> Option<(String, SessionTone)> {
    let needs_user = match shown.status() {
        ProfileStatus::NotLoggedIn => Some(Key::AgentsStateNeedsLogin),
        ProfileStatus::ProgramNotFound => Some(Key::AgentsStateProgramNotFound),
        ProfileStatus::ConfigDirNotFound => {
            Some(Key::AgentsStateConfigDirNotFound)
        },
        _ => None,
    };
    if let Some(word) = needs_user {
        return Some((i18n::t(word).to_string(), SessionTone::Failed));
    }

    let is_api_key = matches!(
        shown.status(),
        ProfileStatus::Connected {
            billing: Some(BillingKind::ApiKey),
            ..
        }
    );
    if is_api_key {
        return money_today(shown);
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
                i18n::with_count(Key::TplAgentsMostUsed, u64::from(percent)),
                tone,
            ))
        },
        Billing::Credits(credits) if credits.is_only_available() => {
            tokens_today(shown)
        },
        Billing::Credits(credits) => {
            Some((credits_left(credits)?, SessionTone::Muted))
        },
        Billing::ApiKey => money_today(shown),
    }
}

/// The money an API key profile spent today, such as "$12.40 today",
/// estimated from its own tokens; `None` before they are counted. The
/// bill is left out even when it answered: it is the money of the whole
/// organization, and the row is about this profile.
fn money_today(shown: &ProfileState) -> Option<(String, SessionTone)> {
    let today = shown.money()?.today().cost();

    Some((
        i18n::agents_money_today(&dollars_text(today)),
        SessionTone::Muted,
    ))
}

/// The tokens the profile spent today, such as "1.8M today", written as
/// the dashboard writes them; `None` before they are counted.
fn tokens_today(shown: &ProfileState) -> Option<(String, SessionTone)> {
    let (spend, _) = shown.spend().last_good()?;
    let today = short_count(spend.today().total());

    Some((i18n::agents_tokens_today(&today), SessionTone::Muted))
}

/// The credits left, such as "3 760 left", or that they have no limit;
/// `None` when the source told neither.
fn credits_left(credits: &Credits) -> Option<String> {
    if credits.is_unlimited() {
        return Some(i18n::t(Key::AgentsCreditsUnlimited).to_string());
    }

    let left = credits.left()?;
    Some(i18n::agents_credits_left(&credits_text(left)))
}

#[cfg(test)]
mod tests {
    use otty_agents::{Limits, ProfileStatus};

    use super::*;
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        at, bill_loaded, bill_of, claude, claude_with_admin_key, connected,
        five_hour_limits, limits_answer, limits_loaded, price_table,
        spend_loaded, spend_of, with_profiles,
    };

    fn api_key() -> ProfileStatus {
        ProfileStatus::Connected {
            account: None,
            plan: None,
            billing: Some(BillingKind::ApiKey),
        }
    }

    fn credits(left: f64) -> Limits {
        Limits::new(
            Billing::Credits(Credits::new(Some(left), None, None, None)),
            None,
        )
    }

    /// Claude Code with `status`, and the answer `limits` read.
    fn with_limits(status: ProfileStatus, limits: Limits) -> AgentsState {
        let mut state = with_profiles(&[(claude(), status)]);
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![(
                    otty_agents::SourceId::ChatGptUsageApi,
                    Ok(limits),
                )]),
                at(10),
            ),
        );

        state
    }

    fn number(state: &AgentsState) -> Option<(String, SessionTone)> {
        short_number(&state.profiles()[0])
    }

    fn muted(text: String) -> Option<(String, SessionTone)> {
        Some((text, SessionTone::Muted))
    }

    #[test]
    fn a_subscription_shows_its_most_used_window() {
        let state = with_limits(connected(), five_hour_limits(41.6));

        assert_eq!(
            number(&state),
            muted(i18n::with_count(Key::TplAgentsMostUsed, 42))
        );
    }

    #[test]
    fn a_window_nearly_used_up_is_drawn_in_the_waiting_color() {
        let state = with_limits(connected(), five_hour_limits(86.0));

        assert_eq!(
            number(&state),
            Some((
                i18n::with_count(Key::TplAgentsMostUsed, 86),
                SessionTone::Waiting
            ))
        );
    }

    #[test]
    fn credits_show_what_is_left_in_groups_of_three_digits() {
        let state = with_limits(connected(), credits(1_234_567.4));
        let small = with_limits(connected(), credits(3760.0));

        assert_eq!(
            number(&state),
            muted(i18n::agents_credits_left("1 234 567"))
        );
        assert_eq!(number(&small), muted(i18n::agents_credits_left("3 760")));
    }

    #[test]
    fn a_workspace_pool_shows_what_is_left_of_it() {
        let pool = Limits::new(
            Billing::Credits(Credits::new(
                Some(92_239.0),
                Some(7761.0),
                Some(100_000.0),
                Some(at(10_000)),
            )),
            None,
        );
        let state = with_limits(connected(), pool);

        assert_eq!(number(&state), muted(i18n::agents_credits_left("92 239")));
    }

    #[test]
    fn unlimited_credits_say_so() {
        let state = with_limits(
            connected(),
            Limits::new(Billing::Credits(Credits::unlimited()), None),
        );

        assert_eq!(
            number(&state),
            muted(i18n::t(Key::AgentsCreditsUnlimited).to_string())
        );
    }

    #[test]
    fn credits_with_no_number_show_the_tokens_spent_today() {
        let available =
            Limits::new(Billing::Credits(Credits::available()), None);
        let mut state = with_limits(connected(), available);
        let before = number(&state);

        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_of(1_800_000, 1))), at(10)),
        );

        assert_eq!(before, None, "no tokens were counted yet");
        assert_eq!(number(&state), muted(i18n::agents_tokens_today("1.8M")));
    }

    #[test]
    fn an_api_key_shows_the_money_spent_today() {
        let mut state = with_profiles(&[(claude(), api_key())]);
        state.set_prices(price_table());
        let before = number(&state);

        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_of(10_000_000, 1))), at(10)),
        );

        assert_eq!(before, None, "no tokens were counted yet");
        assert_eq!(number(&state), muted(i18n::agents_money_today("$12.50")));
    }

    #[test]
    fn the_row_shows_the_profiles_own_estimate_even_when_the_bill_answered() {
        let mut state = with_profiles(&[(claude_with_admin_key(), api_key())]);
        state.set_prices(price_table());
        let _ = reduce(
            &mut state,
            bill_loaded(&claude_with_admin_key(), Ok(bill_of(40, 1))),
        );
        let before_count = number(&state);

        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude_with_admin_key(),
                Some(Ok(spend_of(10_000_000, 1))),
                at(10),
            ),
        );

        assert_eq!(
            before_count, None,
            "the bill of the organization is not this profile's money"
        );
        assert_eq!(number(&state), muted(i18n::agents_money_today("$12.50")));
    }

    /// Owner, issue 37: with no prices there is no money to show.
    #[test]
    fn an_api_key_with_no_prices_shows_no_money() {
        let mut state = with_profiles(&[(claude(), api_key())]);

        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_of(10_000_000, 1))), at(10)),
        );

        assert_eq!(number(&state), None);
    }

    #[test]
    fn money_is_estimated_only_for_an_api_key() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_of(10_000_000, 1))), at(10)),
        );

        assert_eq!(state.profiles()[0].money(), None);
        assert_eq!(number(&state), None);
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
            let state = with_limits(status, five_hour_limits(42.0));

            assert_eq!(
                number(&state),
                Some((i18n::t(word).to_string(), SessionTone::Failed))
            );
        }
    }

    #[test]
    fn nothing_known_yet_shows_no_number() {
        let state = with_profiles(&[(claude(), connected())]);

        assert_eq!(number(&state), None);
    }
}
