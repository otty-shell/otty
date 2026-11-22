//! What the statistics popup of a session or of a project shows (spec
//! 2026-11-19, Decision 6): the facts as a list, the usage windows of a
//! session whose agent writes them, and the usage block of the session or of every profile
//! in the project. A session names the account that paid for it; the
//! money of a project counts only the profiles that pay per token and
//! names the others.

use std::time::{Duration, SystemTime};

use jiff::tz::TimeZone;
use otty_agents::{Percent, Profile, ProjectStats, SessionStats};
use otty_app_domain::folder_path::folder_name;
use otty_app_i18n::{self as i18n, Key};

use super::super::failure_text::{named_failure, source_error_text};
use super::super::profile_state::ProfileState;
use super::super::project_summary::{ProjectSummary, StatsCost};
use super::super::state::AgentsState;
use super::super::stats_dialog::{StatsRead, StatsSubject};
use super::super::types::ProjectStatsFailure;
use super::super::usage::{UsageChoice, UsageTab};
use super::account_numbers::shown_total;
use super::numbers::{digit_groups, short_count};
use super::profile_page::{account_reason, money_reason};
use super::reset_time::{date_time_text, duration_text};
use super::session_account::session_account_text;
use super::usage_block::{UsageBlockViewModel, UsageSubject};
use super::usage_groups::UsageSource;
use super::window_name::window_name;
use super::{FactViewModel, UNKNOWN};

/// The statistics dialog: its title and what it shows under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatsDialogViewModel {
    /// The session title, or the folder name of the project.
    pub(crate) title: String,
    /// What the dialog shows.
    pub(crate) body: StatsBodyViewModel,
}

impl StatsDialogViewModel {
    /// The open statistics dialog at `now`; `None` while none is open. The
    /// times are written in the user's time zone.
    pub(crate) fn from_state(
        state: &AgentsState,
        now: SystemTime,
    ) -> Option<Self> {
        let dialog = state.stats()?;
        let choice = dialog.usage();

        let title = match dialog.subject() {
            StatsSubject::Session { title, .. } => title.clone(),
            StatsSubject::Project(folder) => {
                folder_name(folder, Some(state.env().home()))
            },
        };
        // While the read is on its way, the spinner.
        let body = match (dialog.subject(), dialog.read()) {
            (_, StatsRead::SessionFailed(error)) => {
                failed_body(&source_error_text(error))
            },
            (_, StatsRead::ProjectFailed(failure)) => {
                failed_body(&project_failure_text(failure))
            },
            (
                StatsSubject::Session { profile, .. },
                StatsRead::Session(stats),
            ) => StatsBodyViewModel::Session(SessionStatsViewModel::new(
                state, profile, stats, choice, now,
            )),
            (StatsSubject::Project(_), StatsRead::Project(parts)) => {
                StatsBodyViewModel::Project(ProjectStatsViewModel::new(
                    state, parts, choice, now,
                ))
            },
            _ => StatsBodyViewModel::Reading,
        };

        Some(Self { title, body })
    }
}

/// What the statistics dialog shows under its title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StatsBodyViewModel {
    /// The spinner: the statistics are read.
    Reading,
    /// Why the statistics could not be read.
    Failed(String),
    /// The statistics of a session.
    Session(SessionStatsViewModel),
    /// The statistics of a project.
    Project(ProjectStatsViewModel),
}

/// The statistics of a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionStatsViewModel {
    /// Agent, profile, first and last answer, length, answers, tokens and
    /// the account that paid.
    pub(crate) facts: Vec<FactViewModel>,
    /// Each usage window as its name and "12 % → 31 %"; empty for an agent
    /// that writes no limits into its sessions.
    pub(crate) limits: Vec<FactViewModel>,
    /// The tokens of the session by model or by account.
    pub(crate) usage: UsageBlockViewModel,
}

impl SessionStatsViewModel {
    /// The statistics `stats` of a session of `profile`, its usage block
    /// as `choice` picks it at `now`; the money only when the profile pays
    /// per token.
    fn new(
        state: &AgentsState,
        profile: &Profile,
        stats: &SessionStats,
        choice: &UsageChoice,
        now: SystemTime,
    ) -> Self {
        let profiles = state.profiles();
        let time_zone = profiles.time_zone();
        let shown = profiles.get(profile);
        let accounts = shown.map(ProfileState::accounts).unwrap_or_default();
        let agent = state.agent_name(profile.agent());
        let length = stats
            .length()
            .map_or_else(|| String::from(UNKNOWN), length_text);
        let account = session_account_text(
            state.registry(),
            stats.models(),
            accounts,
            &profiles.all_accounts(),
            profiles.saved(),
        );

        let facts = vec![
            fact(Key::AgentsAgent, agent.to_string()),
            fact(Key::AgentsProfile, profile.name().to_string()),
            fact(
                Key::AgentsStatsStarted,
                time_text(stats.started_at(), time_zone),
            ),
            fact(
                Key::AgentsStatsLastActivity,
                time_text(stats.last_active_at(), time_zone),
            ),
            fact(Key::AgentsStatsLength, length),
            fact(Key::AgentsStatsAnswers, digit_groups(stats.answers())),
            fact(Key::AgentsTokens, short_count(stats.tokens().total())),
            fact(Key::AgentsFieldAccount, account),
        ];
        let limits = stats
            .limit_changes()
            .iter()
            .map(|change| {
                let start = whole_percent(change.at_start());
                let end = whole_percent(change.at_end());
                let text = i18n::filled(
                    Key::TplAgentsLimitChange,
                    &[("start", &start), ("end", &end)],
                );
                FactViewModel::new(&window_name(change.window()), text)
            })
            .collect();

        let money_reason = match shown {
            Some(shown) => money_reason(state, shown),
            None => Some(i18n::t(Key::AgentsMoneySubscription).to_string()),
        };
        let account_reason =
            shown.and_then(|shown| account_reason(state, agent, shown));
        let sources = [UsageSource {
            name: profile.name(),
            spend: stats.spend(),
            accounts,
            is_priced: money_reason.is_none(),
        }];
        let subject = UsageSubject {
            sources: &sources,
            project: None,
            tabs: &UsageTab::SESSION_POPUP,
            prices: profiles.prices().last_good(),
            can_show_money: money_reason.is_none(),
            account_reason: account_reason.as_deref(),
            left_out: None,
            home: state.env().home(),
            saved: profiles.saved(),
            registry: state.registry(),
            time_zone,
        };
        let usage = UsageBlockViewModel::of(&subject, choice, now)
            .in_popup(choice, money_reason);

        Self {
            facts,
            limits,
            usage,
        }
    }
}

/// The statistics of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectStatsViewModel {
    /// Sessions, first and last activity, profiles, tokens, and the money
    /// of the API key profiles when there is one.
    pub(crate) facts: Vec<FactViewModel>,
    /// The tokens spent in the project by profile, by model or by account.
    pub(crate) usage: UsageBlockViewModel,
}

impl ProjectStatsViewModel {
    /// The statistics of the selected project from what each profile's
    /// files told, `parts`, its usage block as `choice` picks it at `now`.
    /// The sessions are counted in the project list; the money is that of
    /// the API key profiles only.
    fn new(
        state: &AgentsState,
        parts: &[(Profile, ProjectStats)],
        choice: &UsageChoice,
        now: SystemTime,
    ) -> Self {
        let profiles = state.profiles();
        let time_zone = profiles.time_zone();
        let summary = ProjectSummary::new(state, parts);
        let all = summary.all();
        let sessions = summary.sessions();
        let shown: Vec<&ProfileState> = parts
            .iter()
            .filter_map(|(profile, _)| profiles.get(profile))
            .collect();
        let names: Vec<&str> =
            parts.iter().map(|(profile, _)| profile.name()).collect();

        let mut facts = vec![
            fact(Key::AgentsSessions, format!("{sessions}")),
            fact(
                Key::AgentsStatsFirstActivity,
                time_text(all.first_at(), time_zone),
            ),
            fact(
                Key::AgentsStatsLastActivity,
                time_text(all.last_at(), time_zone),
            ),
            fact(Key::AgentsSectionProfiles, names.join(", ")),
            fact(Key::AgentsTokens, short_count(all.tokens().total())),
        ];
        if summary.cost().is_paid() {
            let cost = sum_text(state, summary.cost());
            facts.push(fact(Key::AgentsColumnCost, cost));
        }

        let sources: Vec<UsageSource> = shown
            .iter()
            .filter_map(|shown| {
                let (spend, _) = shown.spend().last_good()?;
                Some(UsageSource {
                    name: shown.profile().name(),
                    spend,
                    accounts: shown.accounts(),
                    is_priced: shown.is_api_key(),
                })
            })
            .collect();
        let money_reason = project_money_reason(state, &shown);
        let left_out = left_out_text(&shown);
        let account_reason = shown.iter().find_map(|shown| {
            account_reason(
                state,
                state.agent_name(shown.profile().agent()),
                shown,
            )
        });
        let folder = state.selected_project().map(|project| project.folder());
        let subject = UsageSubject {
            sources: &sources,
            project: folder,
            tabs: &UsageTab::PROJECT_POPUP,
            prices: profiles.prices().last_good(),
            can_show_money: money_reason.is_none(),
            account_reason: account_reason.as_deref(),
            left_out,
            home: state.env().home(),
            saved: profiles.saved(),
            registry: state.registry(),
            time_zone,
        };
        let usage = UsageBlockViewModel::of(&subject, choice, now)
            .in_popup(choice, money_reason);

        Self { facts, usage }
    }
}

/// Why the usage of a project with the profiles `shown` can not be shown
/// as money: none of them pays per token, or the list prices are not
/// known. `None` when it can.
fn project_money_reason(
    state: &AgentsState,
    shown: &[&ProfileState],
) -> Option<String> {
    if !shown.iter().any(|shown| shown.is_api_key()) {
        return Some(i18n::t(Key::AgentsMoneySubscription).to_string());
    }
    if state.profiles().prices().table().is_none() {
        return Some(i18n::t(Key::AgentsMoneyNoPrices).to_string());
    }

    None
}

/// The line that names the profiles of `shown` paid by a subscription,
/// which the money of the project leaves out; `None` when every profile
/// pays per token, or none does and the money is off.
fn left_out_text(shown: &[&ProfileState]) -> Option<String> {
    let left_out: Vec<&str> = shown
        .iter()
        .filter(|shown| !shown.is_api_key())
        .map(|shown| shown.profile().name())
        .collect();
    if left_out.is_empty() || left_out.len() == shown.len() {
        return None;
    }

    Some(i18n::filled(
        Key::TplAgentsLeftOutOfMoney,
        &[("names", &left_out.join(", "))],
    ))
}

/// The dialog body that tells why the statistics could not be read.
fn failed_body(reason: &str) -> StatsBodyViewModel {
    StatsBodyViewModel::Failed(i18n::with_error(
        Key::TplAgentsStatsFailed,
        reason,
    ))
}

/// Why the statistics of a project could not be read, after the name of
/// the profile whose files failed, such as "Work: not available: …".
fn project_failure_text(failure: &ProjectStatsFailure) -> String {
    let reason = source_error_text(&failure.error);

    match &failure.profile_name {
        Some(name) => named_failure(name, &reason),
        None => reason,
    }
}

/// The fact `key` with its `value`.
fn fact(key: Key, value: String) -> FactViewModel {
    FactViewModel::new(i18n::t(key), value)
}

/// What a sum of costs reads, in one line of the facts, where there is
/// room for the mark: the dollars of the requests with a price, followed
/// by the mark when some are at the list prices; a dash while there are
/// no prices, and nothing when no profile pays per token.
fn sum_text(state: &AgentsState, cost: StatsCost) -> String {
    match cost {
        StatsCost::Priced(total) => shown_total(state, &total)
            .map_or_else(|| String::from(UNKNOWN), |money| money.line_text()),
        StatsCost::NoPrices => String::from(UNKNOWN),
        StatsCost::Unpaid => String::new(),
    }
}

/// The date and the clock time of `time`, such as "Nov 16, 2026, 10:00",
/// in the user's `time_zone`, or in UTC when it is not known; "—" for no
/// time.
fn time_text(time: Option<SystemTime>, time_zone: Option<&TimeZone>) -> String {
    time.map_or_else(
        || String::from(UNKNOWN),
        |time| date_time_text(time, time_zone),
    )
}

/// How long `length` is, in hours and minutes, such as "2 h 10 min".
fn length_text(length: Duration) -> String {
    duration_text(length.as_secs())
}

/// A percent rounded to a whole one, such as "31".
fn whole_percent(percent: Percent) -> String {
    let rounded = percent.rounded();

    format!("{rounded}")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        DaySpend, Dollars, LimitChange, ModelPrice, ModelTokens, PriceTable,
        ProjectModelTokens, ProviderRef, SavedProfiles, SessionId, SourceError,
        Spend, TokenCount, TokenPrices, Unavailable, WindowId,
    };
    use otty_app_i18n::Locale;
    use otty_app_testing::alone::is_alone;

    use super::*;
    use crate::event::{AgentsIntent, SessionAction};
    use crate::model::UsageTableRowViewModel;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, chatgpt_login, claude, codex, connected, month_spend, now,
        organization, read, session, set_prices, spend_loaded, with_accounts,
        with_profiles,
    };
    use crate::types::{LoadedProjectStats, LoadedSessionStats, SpendCount};
    use crate::usage::{UsageMeasure, UsagePeriod, UsageTab};

    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;
    /// A dollar in micro-dollars.
    const DOLLAR: u64 = 1_000_000;

    fn otty() -> PathBuf {
        PathBuf::from("/work/otty")
    }

    /// Made-up prices: `opus` costs $1 per million input tokens, `gpt-5`
    /// $2.
    fn prices() -> PriceTable {
        let input = |micro_dollars| TokenPrices {
            input: Dollars::from_micro_dollars(micro_dollars),
            ..TokenPrices::default()
        };

        PriceTable::new(vec![
            ModelPrice::new("opus", input(DOLLAR)),
            ModelPrice::new("gpt-5", input(2 * DOLLAR)),
        ])
    }

    fn million_in(model: &str, requests: u64) -> ModelTokens {
        ModelTokens::new(
            Some(model),
            TokenCount::new(1_000_000, 500, 2_000),
            requests,
        )
    }

    /// Claude Code with the `status` and the session c1 in /work/otty,
    /// and Codex, connected, with the session x1 there, the project
    /// selected, at the made-up prices.
    fn state_with(status: otty_agents::ProfileStatus) -> AgentsState {
        let mut state = state_with_no_prices(status);
        set_prices(&mut state, prices());
        state
    }

    /// The same as [`state_with`], before any prices were read.
    fn state_with_no_prices(status: otty_agents::ProfileStatus) -> AgentsState {
        let mut state =
            with_profiles(&[(claude(), status), (codex(), connected())]);
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );
        read(&mut state, &codex(), vec![session("x1", "/work/otty", 200)]);
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));

        state
    }

    /// `input` tokens of `model` through `provider` today, 2026-11-16, in
    /// /work/otty and as many in /work/site.
    fn spend_in_otty(model: &str, provider: ProviderRef, input: u64) -> Spend {
        let tokens =
            ModelTokens::new(Some(model), TokenCount::new(input, 0, 0), 1)
                .with_provider(Some(provider));
        let part = |folder: &str| {
            ProjectModelTokens::new(Some(PathBuf::from(folder)), tokens.clone())
        };

        Spend::new(
            vec![
                DaySpend::new(
                    jiff::civil::date(2026, 11, 16),
                    vec![tokens.clone()],
                )
                .with_parts(vec![part("/work/otty"), part("/work/site")]),
            ],
            Vec::new(),
        )
    }

    /// Claude Code paid per token by the made-up organization, and Codex
    /// on a ChatGPT subscription, each with a session in /work/otty and
    /// its tokens counted, at the made-up prices; the project selected.
    fn state_with_accounts() -> AgentsState {
        let mut state = with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (codex(), connected(), vec![chatgpt_login()]),
            ],
            SavedProfiles::default(),
        );
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );
        read(&mut state, &codex(), vec![session("x1", "/work/otty", 200)]);
        let counted = [
            (
                claude(),
                spend_in_otty(
                    "opus",
                    ProviderRef::of_id("anthropic"),
                    1_000_000,
                ),
            ),
            (
                codex(),
                spend_in_otty("gpt-5", ProviderRef::of_id("openai"), 2_000),
            ),
        ];
        for (profile, spend) in counted {
            let _ = reduce(
                &mut state,
                spend_loaded(&profile, SpendCount::Counted(spend), at(NOW)),
            );
        }
        set_prices(&mut state, prices());
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));

        state
    }

    /// The project statistics of /work/otty open, with what both profiles
    /// read.
    fn project_dialog(state: &mut AgentsState) {
        let _ = reduce(state, AgentsIntent::ShowProjectStats);
        let part = ProjectStats::new(
            Some(at(NOW - 3600)),
            Some(at(NOW)),
            vec![million_in("opus", 1)],
        );
        let _ = reduce(
            state,
            AgentsIntent::ProjectStatsLoaded(LoadedProjectStats {
                folder: otty(),
                parts: Ok(vec![(claude(), part.clone()), (codex(), part)]),
            }),
        );
    }

    /// The statistics dialog of the session c1 of Claude Code.
    fn session_dialog(state: &mut AgentsState) {
        let _ = reduce(state, AgentsIntent::OpenSessionMenu(1));
        let _ = reduce(
            state,
            AgentsIntent::SessionMenu(SessionAction::ShowStatistics),
        );
    }

    fn session_stats_loaded(
        stats: Result<SessionStats, SourceError>,
    ) -> AgentsIntent {
        AgentsIntent::SessionStatsLoaded(LoadedSessionStats {
            profile: claude(),
            session: SessionId::new("c1"),
            stats,
        })
    }

    fn body(state: &AgentsState) -> StatsBodyViewModel {
        StatsDialogViewModel::from_state(state, now())
            .expect("the dialog is open")
            .body
    }

    fn project_body(state: &AgentsState) -> ProjectStatsViewModel {
        match body(state) {
            StatsBodyViewModel::Project(vm) => vm,
            other => panic!("the project statistics show, not {other:?}"),
        }
    }

    fn session_body(state: &AgentsState) -> SessionStatsViewModel {
        match body(state) {
            StatsBodyViewModel::Session(vm) => vm,
            other => panic!("the session statistics show, not {other:?}"),
        }
    }

    fn names(rows: &[UsageTableRowViewModel]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    #[test]
    fn while_the_statistics_are_read_the_dialog_shows_the_spinner() {
        let mut state = state_with(connected());
        let closed = StatsDialogViewModel::from_state(&state, now());

        session_dialog(&mut state);
        let vm = StatsDialogViewModel::from_state(&state, now());

        assert_eq!(closed, None);
        assert_eq!(
            vm,
            Some(StatsDialogViewModel {
                title: String::from("Title of c1"),
                body: StatsBodyViewModel::Reading,
            })
        );
    }

    #[test]
    fn a_failure_shows_its_reason_in_the_dialog() {
        let mut state = state_with(connected());
        session_dialog(&mut state);

        let _ = reduce(
            &mut state,
            session_stats_loaded(Err(SourceError::NotAvailable(
                Unavailable::NoSessionFile(SessionId::new("c1")),
            ))),
        );

        assert_eq!(
            body(&state),
            StatsBodyViewModel::Failed(i18n::with_error(
                Key::TplAgentsStatsFailed,
                "not available: no session file of c1"
            ))
        );
    }

    #[test]
    fn a_failed_project_read_names_the_profile_whose_files_failed() {
        let mut state = state_with(connected());
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);
        let mut fail = |profile_name| {
            let _ = reduce(
                &mut state,
                AgentsIntent::ProjectStatsLoaded(LoadedProjectStats {
                    folder: otty(),
                    parts: Err(ProjectStatsFailure {
                        profile_name,
                        error: SourceError::NotAvailable(
                            Unavailable::ReadStopped,
                        ),
                    }),
                }),
            );
            body(&state)
        };

        let of_profile = fail(Some(claude().name().to_string()));
        let stopped = fail(None);

        let reason = "not available: the read stopped";
        let claude = claude();
        let name = claude.name();
        assert_eq!(of_profile, failed_body(&format!("{name}: {reason}")));
        assert_eq!(stopped, failed_body(reason));
    }

    /// The reason is kept as it was read, not as text: a switch of the
    /// language after the failure writes it in the new language.
    #[test]
    fn a_failure_follows_the_language_of_the_app() {
        if !is_alone("a_failure_follows_the_language_of_the_app") {
            return;
        }
        let mut state = state_with(connected());
        session_dialog(&mut state);
        let no_file = SourceError::NotAvailable(Unavailable::NoSessionFile(
            SessionId::new("c1"),
        ));
        let _ = reduce(&mut state, session_stats_loaded(Err(no_file.clone())));

        i18n::set_locale(Locale::Ru);

        let reason = crate::failure_text::source_error_text(&no_file);
        assert_ne!(reason, "not available: no session file of c1");
        assert_eq!(
            body(&state),
            StatsBodyViewModel::Failed(i18n::with_error(
                Key::TplAgentsStatsFailed,
                &reason
            ))
        );
    }

    #[test]
    fn a_session_shows_its_facts_and_its_usage_by_model_or_account() {
        let mut state = state_with(connected());
        session_dialog(&mut state);
        let stats = SessionStats::new(
            Some(at(NOW)),
            Some(at(NOW + 2 * 3600 + 600)),
            vec![million_in("opus", 3), million_in("mystery", 2)],
        );

        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));

        let vm = session_body(&state);
        let date = i18n::filled(
            Key::TplAgentsMonthDay,
            &[("month", i18n::agents_month(11)), ("day", "16")],
        );
        let when = |time: &str| {
            i18n::filled(
                Key::TplAgentsDateTime,
                &[("date", &date), ("year", "2026"), ("time", time)],
            )
        };
        assert_eq!(
            vm.facts,
            [
                (Key::AgentsAgent, String::from("Claude Code")),
                (Key::AgentsProfile, String::from("Claude Code")),
                (Key::AgentsStatsStarted, when("10:00")),
                (Key::AgentsStatsLastActivity, when("12:10")),
                (
                    Key::AgentsStatsLength,
                    i18n::filled(
                        Key::TplAgentsDurationHoursMinutes,
                        &[("hours", "2"), ("minutes", "10")]
                    )
                ),
                (Key::AgentsStatsAnswers, String::from("5")),
                (Key::AgentsTokens, short_count(2_005_000)),
                (Key::AgentsFieldAccount, String::from(UNKNOWN)),
            ]
            .map(|(key, value)| FactViewModel::new(i18n::t(key), value))
        );
        assert_eq!(vm.usage.tabs, UsageTab::SESSION_POPUP);
        assert!(vm.usage.lists.is_some(), "the lists open in the popup");
        assert!(vm.limits.is_empty());
    }

    /// Spec 2026-11-19, Decision 6: the Account line of a session names
    /// the account that paid, here the one account of the profile.
    #[test]
    fn a_session_names_the_account_that_paid_for_it() {
        let mut state = state_with_accounts();
        session_dialog(&mut state);
        let stats = SessionStats::new(
            None,
            None,
            vec![
                million_in("opus", 1)
                    .with_provider(Some(ProviderRef::of_id("anthropic"))),
            ],
        );

        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));

        let account = session_body(&state)
            .facts
            .into_iter()
            .find(|fact| fact.label == i18n::t(Key::AgentsFieldAccount))
            .expect("the Account line shows");
        assert_eq!(
            account.value,
            i18n::filled(
                Key::TplAgentsAccountOfProfile,
                &[("account", "Anthropic · dev@example.com")]
            )
        );
    }

    /// The session's own tokens by day fill its usage block.
    #[test]
    fn the_usage_of_a_session_is_its_own_tokens() {
        let mut state = state_with_accounts();
        session_dialog(&mut state);
        let stats = SessionStats::new(None, None, Vec::new()).with_spend(
            spend_in_otty("opus", ProviderRef::of_id("anthropic"), 3_000),
        );

        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));
        // Its tokens are today, so it opens on the hours of today, which
        // this made-up count does not have.
        let _ = reduce(
            &mut state,
            AgentsIntent::PickPeriod(crate::usage::UsagePeriod::SevenDays),
        );
        let by_model = session_body(&state).usage;
        let _ =
            reduce(&mut state, AgentsIntent::ShowUsageTab(UsageTab::ByAccount));
        let by_account = session_body(&state).usage;

        assert_eq!(names(&by_model.rows), ["opus"]);
        assert_eq!(by_model.rows[0].value, "6K", "otty and site");
        assert_eq!(names(&by_account.rows), ["Anthropic · dev@example.com"]);
    }

    #[test]
    fn money_shows_for_a_session_of_an_api_key_once_the_prices_are_known() {
        let mut priced = state_with(api_key());
        let mut no_prices = state_with_no_prices(api_key());
        let mut plan = state_with(connected());
        let stats = SessionStats::new(None, None, vec![million_in("opus", 3)]);
        let money_reason = |state: &mut AgentsState| {
            session_dialog(state);
            let _ = reduce(state, session_stats_loaded(Ok(stats.clone())));
            session_body(state)
                .usage
                .lists
                .expect("the lists open in the popup")
                .money_reason
        };

        assert_eq!(money_reason(&mut priced), None);
        assert_eq!(
            money_reason(&mut no_prices).as_deref(),
            Some(i18n::t(Key::AgentsMoneyNoPrices))
        );
        assert_eq!(
            money_reason(&mut plan).as_deref(),
            Some(i18n::t(Key::AgentsMoneySubscription))
        );
    }

    /// A profile whose files tell no API key, while its limits answer
    /// says it pays with one, pays per token in the popup as on its page.
    #[test]
    fn a_profile_whose_limits_say_api_key_prices_its_session() {
        let mut state = state_with(connected());
        let api_key_limits =
            otty_agents::Limits::new(otty_agents::Billing::ApiKey, None);
        let _ = reduce(
            &mut state,
            crate::testing::limits_loaded(
                &claude(),
                crate::testing::limits_answer(vec![(
                    otty_agents::SourceId::ProviderPart,
                    Ok(api_key_limits),
                )]),
                at(NOW),
            ),
        );
        session_dialog(&mut state);
        let stats = SessionStats::new(None, None, vec![million_in("opus", 3)]);

        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));

        let lists = session_body(&state).usage.lists.expect("the lists");
        assert_eq!(lists.money_reason, None);
    }

    #[test]
    fn with_no_prices_a_project_shows_a_dash_for_its_money() {
        let mut state = state_with_no_prices(api_key());
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);
        let part = ProjectStats::new(None, None, vec![million_in("opus", 1)]);

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectStatsLoaded(LoadedProjectStats {
                folder: otty(),
                parts: Ok(vec![(claude(), part.clone()), (codex(), part)]),
            }),
        );

        assert_eq!(
            project_body(&state).facts.last(),
            Some(&FactViewModel::new(
                i18n::t(Key::AgentsColumnCost),
                String::from(UNKNOWN)
            ))
        );
    }

    #[test]
    fn a_codex_session_lists_each_window_from_start_to_end() {
        let mut state = state_with(connected());
        let _ = reduce(&mut state, AgentsIntent::OpenSessionMenu(0));
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::ShowStatistics),
        );
        let stats = SessionStats::new(None, None, Vec::new())
            .with_limit_changes(vec![
                LimitChange::new(
                    WindowId::hours(5),
                    Percent::new(12.0),
                    Percent::new(31.4),
                ),
                LimitChange::new(
                    WindowId::days(7),
                    Percent::new(40.0),
                    Percent::new(42.5),
                ),
            ]);

        let _ = reduce(
            &mut state,
            AgentsIntent::SessionStatsLoaded(LoadedSessionStats {
                profile: codex(),
                session: SessionId::new("x1"),
                stats: Ok(stats),
            }),
        );

        let vm = session_body(&state);
        let change = |start: &str, end: &str| {
            i18n::filled(
                Key::TplAgentsLimitChange,
                &[("start", start), ("end", end)],
            )
        };
        assert_eq!(
            vm.limits,
            [
                FactViewModel::new("5 hours", change("12", "31")),
                FactViewModel::new("7 days", change("40", "43")),
            ]
        );
        assert!(vm.usage.rows.is_empty());
    }

    /// The project sums both profiles and both agents; only the API key
    /// profile has money.
    #[test]
    fn a_project_shows_its_facts_and_prices_only_the_api_key_profiles() {
        let mut state = state_with(api_key());
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);
        let claude_part = ProjectStats::new(
            Some(at(NOW - 3600)),
            Some(at(NOW)),
            vec![million_in("opus", 1)],
        );
        let codex_part = ProjectStats::new(
            Some(at(NOW - 7200)),
            Some(at(NOW - 60)),
            vec![million_in("gpt-5", 1), million_in("opus", 1)],
        );

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectStatsLoaded(LoadedProjectStats {
                folder: otty(),
                parts: Ok(vec![(claude(), claude_part), (codex(), codex_part)]),
            }),
        );

        let vm = StatsDialogViewModel::from_state(&state, now())
            .expect("the dialog is open");
        assert_eq!(vm.title, "otty");
        let StatsBodyViewModel::Project(vm) = vm.body else {
            panic!("the project statistics show");
        };
        let date = i18n::filled(
            Key::TplAgentsMonthDay,
            &[("month", i18n::agents_month(11)), ("day", "16")],
        );
        let when = |time: &str| {
            i18n::filled(
                Key::TplAgentsDateTime,
                &[("date", &date), ("year", "2026"), ("time", time)],
            )
        };
        // Owner, 2026-11-21: the cost at the list prices has the one mark
        // of an estimate after it; the facts have room for its words.
        let cost = i18n::filled(
            Key::TplAgentsEstimatedAmount,
            &[
                ("amount", "$1.00"),
                ("mark", i18n::t(Key::AgentsEstimateMark)),
            ],
        );
        assert_eq!(
            vm.facts,
            [
                (Key::AgentsSessions, String::from("2")),
                (Key::AgentsStatsFirstActivity, when("08:00")),
                (Key::AgentsStatsLastActivity, when("10:00")),
                (
                    Key::AgentsSectionProfiles,
                    String::from("Claude Code, Codex")
                ),
                (Key::AgentsTokens, short_count(3_007_500)),
                (Key::AgentsColumnCost, cost),
            ]
            .map(|(key, value)| FactViewModel::new(i18n::t(key), value))
        );
        assert_eq!(vm.usage.tabs, UsageTab::PROJECT_POPUP);
    }

    /// Spec 2026-11-19, issue 07: the project sums its usage by profile
    /// and by account, only what was spent in the project.
    #[test]
    fn a_project_sums_its_usage_by_profile_and_by_account() {
        let mut state = state_with_accounts();
        project_dialog(&mut state);

        let _ =
            reduce(&mut state, AgentsIntent::ShowUsageTab(UsageTab::ByProfile));
        let by_profile = project_body(&state).usage;
        let _ =
            reduce(&mut state, AgentsIntent::ShowUsageTab(UsageTab::ByAccount));
        let by_account = project_body(&state).usage;

        assert_eq!(
            by_profile.rows,
            [
                UsageTableRowViewModel {
                    name: String::from("Claude Code"),
                    value: String::from("1M"),
                    share: String::from("100 %"),
                },
                UsageTableRowViewModel {
                    name: String::from("Codex"),
                    value: String::from("2K"),
                    share: String::from("0 %"),
                },
            ]
        );
        assert_eq!(
            names(&by_account.rows),
            ["Anthropic · dev@example.com", "OpenAI · you@example.com"]
        );
    }

    /// Spec 2026-11-19, issue 07: money only from known prices; a line
    /// names the profiles paid by a subscription, which it leaves out.
    #[test]
    fn the_money_of_a_project_leaves_out_and_names_the_subscriptions() {
        let mut state = state_with_accounts();
        project_dialog(&mut state);

        let _ =
            reduce(&mut state, AgentsIntent::ShowUsageTab(UsageTab::ByProfile));
        let _ =
            reduce(&mut state, AgentsIntent::PickMeasure(UsageMeasure::Money));
        let usage = project_body(&state).usage;

        assert_eq!(usage.measure, UsageMeasure::Money);
        assert_eq!(names(&usage.rows), ["Claude Code"]);
        assert_eq!(
            usage.notes,
            [i18n::filled(
                Key::TplAgentsLeftOutOfMoney,
                &[("names", "Codex")]
            )]
        );
        let kind = usage.money_kind.expect("the usage is money");
        let mark = kind.mark().expect("the estimate is marked");
        assert_eq!(mark.label, i18n::t(Key::AgentsEstimateMark));
    }

    /// The total under the title over `count` days, such as "700K in 7
    /// days".
    fn in_days(amount: &str, count: &str) -> Option<String> {
        Some(i18n::filled(
            Key::TplAgentsAmountInDays,
            &[("amount", amount), ("count", count)],
        ))
    }

    /// The total under the title of today, such as "100K today".
    fn of_today(amount: &str) -> Option<String> {
        Some(i18n::filled(
            Key::TplAgentsAmountToday,
            &[("amount", amount)],
        ))
    }

    /// Owner, 2026-11-23: the switches of the session popup change what
    /// it shows. It opens on Today, the day of its last tokens, with a bar
    /// per hour; 30 days has a bar per day; Money writes the sums in
    /// dollars.
    #[test]
    fn the_switches_of_the_session_popup_change_its_numbers() {
        let mut state = state_with(api_key());
        session_dialog(&mut state);
        let stats =
            SessionStats::new(None, None, Vec::new()).with_spend(month_spend());
        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));

        let opened = session_body(&state).usage;
        let _ = reduce(
            &mut state,
            AgentsIntent::PickPeriod(UsagePeriod::ThirtyDays),
        );
        let month = session_body(&state).usage;
        let _ =
            reduce(&mut state, AgentsIntent::PickMeasure(UsageMeasure::Money));
        let money = session_body(&state).usage;

        assert_eq!(opened.period, UsagePeriod::Today);
        assert_eq!(opened.bars.len(), 24);
        assert_eq!(opened.line, of_today("100K"));
        assert_eq!(month.bars.len(), 30);
        assert_eq!(month.line, in_days("3M", "30"));
        assert_eq!(money.measure, UsageMeasure::Money);
        assert_eq!(money.line, in_days("$6.00", "30"));
        assert_eq!(money.rows[0].value, "$6.00");
        assert_eq!(state.usage(), &crate::usage::UsageChoice::default());
    }

    /// Owner, 2026-11-23: the switches of the project popup change what
    /// it shows: 7 days at first, Today with a bar per hour, and Money
    /// the sums in dollars.
    #[test]
    fn the_switches_of_the_project_popup_change_its_numbers() {
        let mut state = state_with(api_key());
        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                SpendCount::Counted(month_spend()),
                at(NOW),
            ),
        );
        project_dialog(&mut state);

        let opened = project_body(&state).usage;
        let _ =
            reduce(&mut state, AgentsIntent::PickPeriod(UsagePeriod::Today));
        let today = project_body(&state).usage;
        let _ =
            reduce(&mut state, AgentsIntent::PickMeasure(UsageMeasure::Money));
        let money = project_body(&state).usage;

        assert_eq!(opened.period, UsagePeriod::SevenDays);
        assert_eq!(opened.bars.len(), 7);
        assert_eq!(opened.line, in_days("700K", "7"));
        assert_eq!(today.bars.len(), 24);
        assert_eq!(today.line, of_today("100K"));
        assert_eq!(money.measure, UsageMeasure::Money);
        assert_eq!(money.line, of_today("$0.20"));
        assert_eq!(state.usage(), &crate::usage::UsageChoice::default());
    }

    #[test]
    fn a_project_with_no_api_key_profile_has_no_money() {
        let mut state = state_with(connected());
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectStatsLoaded(LoadedProjectStats {
                folder: otty(),
                parts: Ok(vec![(
                    claude(),
                    ProjectStats::new(None, None, vec![million_in("opus", 1)]),
                )]),
            }),
        );

        let vm = project_body(&state);
        assert_eq!(vm.facts.len(), 5, "no Cost");
        assert_eq!(vm.facts[1].value, UNKNOWN);
        assert_eq!(
            vm.usage
                .lists
                .and_then(|lists| lists.money_reason)
                .as_deref(),
            Some(i18n::t(Key::AgentsMoneySubscription))
        );
    }
}
