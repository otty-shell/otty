//! What the statistics dialog of a session or of a project shows: the
//! facts as a list, the tokens of each model, the usage windows of a Codex
//! session, and the profiles of a project; the money only for a profile
//! that pays with an API key (spec Decisions 9 and 14).

use std::time::{Duration, SystemTime};

use jiff::tz::TimeZone;
use otty_agents::{
    ModelTokens, Percent, PriceTable, Profile, ProjectStats, SessionStats,
    TokenCount,
};
use otty_app_domain::folder_path::folder_name;
use otty_app_i18n::{self as i18n, Key};

use super::super::failure_text::{named_failure, source_error_text};
use super::super::project_summary::{ProjectSummary, StatsCost};
use super::super::state::AgentsState;
use super::super::stats_dialog::{StatsRead, StatsSubject};
use super::super::types::ProjectStatsFailure;
use super::numbers::{digit_groups, dollars_text, short_count};
use super::reset_time::{date_time_text, duration_text};
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
    /// The open statistics dialog; `None` while none is open. The times
    /// are written in the user's time zone.
    pub(crate) fn from_state(state: &AgentsState) -> Option<Self> {
        let dialog = state.stats()?;

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
                state, profile, stats,
            )),
            (_, StatsRead::Project(parts)) => StatsBodyViewModel::Project(
                ProjectStatsViewModel::new(state, parts),
            ),
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
    /// Agent, profile, started, last activity, length, messages, answers.
    pub(crate) facts: Vec<FactViewModel>,
    /// The tokens of each model.
    pub(crate) models: ModelsTableViewModel,
    /// Each usage window as its name and "12 % → 31 %"; empty for an agent
    /// that writes no limits into its sessions.
    pub(crate) limits: Vec<FactViewModel>,
}

impl SessionStatsViewModel {
    /// The statistics `stats` of a session of `profile`; the money only
    /// when the profile pays with an API key.
    fn new(
        state: &AgentsState,
        profile: &Profile,
        stats: &SessionStats,
    ) -> Self {
        let time_zone = state.profiles().time_zone();
        let pays_per_token = state.profiles().is_api_key(profile);
        let length = stats
            .length()
            .map_or_else(|| String::from(UNKNOWN), length_text);

        let facts = vec![
            fact(
                Key::AgentsAgent,
                state.agent_name(profile.agent()).to_string(),
            ),
            fact(Key::AgentsColumnProfile, profile.name().to_string()),
            fact(
                Key::AgentsStatsStarted,
                time_text(stats.started_at(), time_zone),
            ),
            fact(
                Key::AgentsStatsLastActivity,
                time_text(stats.last_active_at(), time_zone),
            ),
            fact(Key::AgentsStatsLength, length),
            fact(Key::AgentsStatsMessages, digit_groups(stats.messages())),
            fact(Key::AgentsStatsAnswers, digit_groups(stats.answers())),
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

        Self {
            facts,
            models: ModelsTableViewModel::new(
                stats.models(),
                pays_per_token,
                state.profiles().prices().table(),
            ),
            limits,
        }
    }
}

/// The statistics of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectStatsViewModel {
    /// Sessions, first and last activity, and the money of the API key
    /// profiles when there is one.
    pub(crate) facts: Vec<FactViewModel>,
    /// The tokens of each model, of every profile together.
    pub(crate) models: ModelsTableViewModel,
    /// The sessions and tokens of each profile.
    pub(crate) profiles: ProfilesTableViewModel,
}

impl ProjectStatsViewModel {
    /// The statistics of the selected project from what each profile's
    /// files told, `parts`. The sessions are counted in the project list;
    /// the money is that of the API key profiles only.
    fn new(state: &AgentsState, parts: &[(Profile, ProjectStats)]) -> Self {
        let time_zone = state.profiles().time_zone();
        let summary = ProjectSummary::new(state, parts);
        let all = summary.all();
        let sessions = summary.sessions();

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
        ];
        if summary.cost().is_paid() {
            facts.push(fact(Key::AgentsColumnCost, sum_text(summary.cost())));
        }
        let rows = summary
            .profiles()
            .iter()
            .map(|row| {
                let sessions = row.sessions();
                ProfileStatsRowViewModel {
                    name: row.profile().name().to_string(),
                    sessions: format!("{sessions}"),
                    tokens: short_count(row.tokens().total()),
                    cost: cost_text(row.cost()),
                }
            })
            .collect();

        Self {
            facts,
            models: ModelsTableViewModel::new(all.models(), false, None),
            profiles: ProfilesTableViewModel {
                has_cost: summary.cost().is_paid(),
                rows,
            },
        }
    }
}

/// The table of the tokens of each model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelsTableViewModel {
    /// Whether the table has the Cost column: only for an API key profile.
    pub(crate) has_cost: bool,
    /// One row for each model, the most first, and the row of all models
    /// when there are several; empty when the files have no tokens.
    pub(crate) rows: Vec<ModelRowViewModel>,
}

impl ModelsTableViewModel {
    /// The table of `models` of a profile that `pays_per_token`, with
    /// the Cost column only then: priced at `prices`, a model with no
    /// price says so and the row of all models sums the money of those
    /// with one; with no prices, each cost is a dash.
    fn new(
        models: &[ModelTokens],
        pays_per_token: bool,
        prices: Option<&PriceTable>,
    ) -> Self {
        let cost_of = |models: &[ModelTokens]| {
            StatsCost::of(models, pays_per_token, prices)
        };

        let mut rows: Vec<ModelRowViewModel> = models
            .iter()
            .map(|model| {
                let name =
                    model.model().unwrap_or(i18n::t(Key::AgentsUnknownModel));
                let cost = cost_text(cost_of(std::slice::from_ref(model)));
                ModelRowViewModel::new(name, model.tokens(), cost)
            })
            .collect();
        if models.len() > 1 {
            let sum = models
                .iter()
                .fold(TokenCount::default(), |sum, model| sum + model.tokens());
            rows.push(ModelRowViewModel::new(
                i18n::t(Key::AgentsStatsAllModels),
                sum,
                sum_text(cost_of(models)),
            ));
        }

        Self {
            has_cost: pays_per_token,
            rows,
        }
    }
}

/// One row of the tokens table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelRowViewModel {
    /// The model, "Unknown model", or "All models" for the last row.
    pub(crate) name: String,
    /// The input tokens, written short.
    pub(crate) input: String,
    /// The input tokens read from the cache, written short.
    pub(crate) cached: String,
    /// The output tokens, written short.
    pub(crate) output: String,
    /// All the tokens, written short.
    pub(crate) total: String,
    /// What they cost at list prices, "no price", a dash while there are
    /// no prices, or empty when the table has no Cost column.
    pub(crate) cost: String,
}

impl ModelRowViewModel {
    /// The row named `name` of `tokens`, costing `cost`.
    fn new(name: &str, tokens: TokenCount, cost: String) -> Self {
        Self {
            name: name.to_string(),
            input: short_count(tokens.input()),
            cached: short_count(tokens.cached()),
            output: short_count(tokens.output()),
            total: short_count(tokens.total()),
            cost,
        }
    }
}

/// The table of the profiles of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfilesTableViewModel {
    /// Whether the table has the Cost column: one of the profiles pays
    /// with an API key.
    pub(crate) has_cost: bool,
    /// One row for each profile, in the order of the profiles.
    pub(crate) rows: Vec<ProfileStatsRowViewModel>,
}

/// One profile of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileStatsRowViewModel {
    /// The profile name.
    pub(crate) name: String,
    /// How many of its sessions ran in the project.
    pub(crate) sessions: String,
    /// The tokens its files tell, written short.
    pub(crate) tokens: String,
    /// What they cost for an API key profile; empty for another one.
    pub(crate) cost: String,
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

/// What a cost reads in a row: its dollars, "no price" when a request in
/// it had none, a dash while there are no prices, and nothing for a
/// profile that does not pay per token.
fn cost_text(cost: StatsCost) -> String {
    match cost {
        StatsCost::Priced(total) if total.unpriced_requests() > 0 => {
            i18n::t(Key::AgentsNoPrice).to_string()
        },
        StatsCost::Priced(total) => dollars_text(total.cost()),
        StatsCost::NoPrices => String::from(UNKNOWN),
        StatsCost::Unpaid => String::new(),
    }
}

/// What a sum of costs reads: the dollars of the requests with a price, a
/// dash while there are no prices, and nothing when no profile pays per
/// token.
fn sum_text(cost: StatsCost) -> String {
    match cost {
        StatsCost::Priced(total) => dollars_text(total.cost()),
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
        Dollars, LimitChange, ModelPrice, SessionId, SourceError, TokenPrices,
        Unavailable, WindowId,
    };
    use otty_app_i18n::Locale;
    use otty_app_testing::alone::is_alone;

    use super::*;
    use crate::event::{AgentsIntent, SessionAction};
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, claude, codex, connected, read, session, set_prices,
        with_profiles,
    };
    use crate::types::{LoadedProjectStats, LoadedSessionStats};

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
        StatsDialogViewModel::from_state(state)
            .expect("the dialog is open")
            .body
    }

    #[test]
    fn while_the_statistics_are_read_the_dialog_shows_the_spinner() {
        let mut state = state_with(connected());
        let closed = StatsDialogViewModel::from_state(&state);

        session_dialog(&mut state);
        let vm = StatsDialogViewModel::from_state(&state);

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
    fn a_session_shows_its_facts_and_tokens_and_no_money_on_a_plan() {
        let mut state = state_with(connected());
        session_dialog(&mut state);
        let stats = SessionStats::new(
            Some(at(NOW)),
            Some(at(NOW + 2 * 3600 + 600)),
            4,
            vec![million_in("opus", 3), million_in("mystery", 2)],
        );

        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));

        let StatsBodyViewModel::Session(vm) = body(&state) else {
            panic!("the session statistics show");
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
        assert_eq!(
            vm.facts,
            [
                (Key::AgentsAgent, String::from("Claude Code")),
                (Key::AgentsColumnProfile, String::from("Claude Code")),
                (Key::AgentsStatsStarted, when("10:00")),
                (Key::AgentsStatsLastActivity, when("12:10")),
                (
                    Key::AgentsStatsLength,
                    i18n::filled(
                        Key::TplAgentsDurationHoursMinutes,
                        &[("hours", "2"), ("minutes", "10")]
                    )
                ),
                (Key::AgentsStatsMessages, String::from("4")),
                (Key::AgentsStatsAnswers, String::from("5")),
            ]
            .map(|(key, value)| FactViewModel::new(i18n::t(key), value))
        );
        let row = |name: &str, input: &str, cached: &str, output: &str| {
            ModelRowViewModel {
                name: String::from(name),
                input: String::from(input),
                cached: String::from(cached),
                output: String::from(output),
                total: String::from(input),
                cost: String::new(),
            }
        };
        assert_eq!(
            vm.models,
            ModelsTableViewModel {
                has_cost: false,
                rows: vec![
                    row("opus", "1M", "500", "2K"),
                    row("mystery", "1M", "500", "2K"),
                    row(i18n::t(Key::AgentsStatsAllModels), "2M", "1K", "4K"),
                ],
            }
        );
        assert!(vm.limits.is_empty());
    }

    #[test]
    fn an_api_key_session_prices_each_model_and_their_sum() {
        let mut state = state_with(api_key());
        session_dialog(&mut state);
        let stats = SessionStats::new(
            None,
            None,
            0,
            vec![million_in("opus", 3), million_in("mystery", 2)],
        );

        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));

        let StatsBodyViewModel::Session(vm) = body(&state) else {
            panic!("the session statistics show");
        };
        let costs: Vec<&str> =
            vm.models.rows.iter().map(|row| row.cost.as_str()).collect();
        assert!(vm.models.has_cost);
        assert_eq!(costs, ["$1.00", i18n::t(Key::AgentsNoPrice), "$1.00"]);
        let started = &vm.facts[2].value;
        assert_eq!(started, UNKNOWN);
    }

    /// A profile whose files tell no API key, while its limits answer
    /// says it pays with one, pays per token in the dialog as on its
    /// dashboard.
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
                    otty_agents::SourceId::AnthropicUsageApi,
                    Ok(api_key_limits),
                )]),
                at(NOW),
            ),
        );
        session_dialog(&mut state);
        let stats =
            SessionStats::new(None, None, 0, vec![million_in("opus", 3)]);

        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));

        let StatsBodyViewModel::Session(vm) = body(&state) else {
            panic!("the session statistics show");
        };
        assert!(vm.models.has_cost);
        assert_eq!(vm.models.rows[0].cost, "$1.00");
    }

    #[test]
    fn with_no_prices_an_api_key_session_shows_a_dash_for_its_money() {
        let mut state = state_with_no_prices(api_key());
        session_dialog(&mut state);
        let stats = SessionStats::new(
            None,
            None,
            0,
            vec![million_in("opus", 3), million_in("mystery", 2)],
        );

        let _ = reduce(&mut state, session_stats_loaded(Ok(stats)));

        let StatsBodyViewModel::Session(vm) = body(&state) else {
            panic!("the session statistics show");
        };
        let costs: Vec<&str> =
            vm.models.rows.iter().map(|row| row.cost.as_str()).collect();
        assert!(vm.models.has_cost);
        assert_eq!(costs, [UNKNOWN, UNKNOWN, UNKNOWN]);
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

        let StatsBodyViewModel::Project(vm) = body(&state) else {
            panic!("the project statistics show");
        };
        assert_eq!(
            vm.facts.last(),
            Some(&FactViewModel::new(
                i18n::t(Key::AgentsColumnCost),
                String::from(UNKNOWN)
            ))
        );
        let costs: Vec<&str> = vm
            .profiles
            .rows
            .iter()
            .map(|row| row.cost.as_str())
            .collect();
        assert_eq!(costs, [UNKNOWN, ""], "only the API key profile has money");
    }

    #[test]
    fn a_codex_session_lists_each_window_from_start_to_end() {
        let mut state = state_with(connected());
        let _ = reduce(&mut state, AgentsIntent::OpenSessionMenu(0));
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::ShowStatistics),
        );
        let stats = SessionStats::new(None, None, 0, Vec::new())
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

        let StatsBodyViewModel::Session(vm) = body(&state) else {
            panic!("the session statistics show");
        };
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
        assert!(vm.models.rows.is_empty());
    }

    /// The project sums both profiles and both agents; only the API key
    /// profile has money.
    #[test]
    fn a_project_sums_its_profiles_and_prices_only_the_api_key_ones() {
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

        let vm = StatsDialogViewModel::from_state(&state)
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
        assert_eq!(
            vm.facts,
            [
                (Key::AgentsSessions, String::from("2")),
                (Key::AgentsStatsFirstActivity, when("08:00")),
                (Key::AgentsStatsLastActivity, when("10:00")),
                (Key::AgentsColumnCost, String::from("$1.00")),
            ]
            .map(|(key, value)| FactViewModel::new(i18n::t(key), value))
        );
        let names: Vec<&str> =
            vm.models.rows.iter().map(|row| row.name.as_str()).collect();
        assert!(!vm.models.has_cost);
        assert_eq!(
            names,
            ["opus", "gpt-5", i18n::t(Key::AgentsStatsAllModels)]
        );
        assert_eq!(vm.models.rows[0].input, "2M");
        assert_eq!(
            vm.profiles,
            ProfilesTableViewModel {
                has_cost: true,
                rows: vec![
                    ProfileStatsRowViewModel {
                        name: String::from("Claude Code"),
                        sessions: String::from("1"),
                        tokens: String::from("1M"),
                        cost: String::from("$1.00"),
                    },
                    ProfileStatsRowViewModel {
                        name: String::from("Codex"),
                        sessions: String::from("1"),
                        tokens: String::from("2M"),
                        cost: String::new(),
                    },
                ],
            }
        );
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

        let StatsBodyViewModel::Project(vm) = body(&state) else {
            panic!("the project statistics show");
        };
        assert!(!vm.profiles.has_cost);
        assert_eq!(vm.facts.len(), 3);
        assert_eq!(vm.facts[1].value, UNKNOWN);
        assert_eq!(vm.models.rows.len(), 1);
    }
}
