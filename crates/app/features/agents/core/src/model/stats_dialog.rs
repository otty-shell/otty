//! What the statistics dialog of a session or of a project shows: the
//! facts as a list, the tokens of each model, the usage windows of a Codex
//! session, and the profiles of a project; the money only for a profile
//! that pays with an API key (spec Decisions 9 and 14).

use std::time::{Duration, SystemTime};

use otty_agents::{
    LocalNow, ModelTokens, MoneyTotal, PriceTable, Profile, ProjectStats,
    SessionStats, TokenCount,
};
use otty_app_i18n::{self as i18n, Key};

use super::super::state::AgentsState;
use super::super::stats_dialog::{StatsRead, StatsSubject};
use super::folder_name;
use super::numbers::{digit_groups, dollars_text, short_count};
use super::reset_time::{clock_text, duration_text};

/// What stands for a time or a length the files did not tell.
const UNKNOWN: &str = "—";

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
            StatsSubject::Project(folder) => folder_name(folder),
        };
        // While the read is on its way, the spinner.
        let body = match (dialog.subject(), dialog.read()) {
            (_, StatsRead::Failed(reason)) => StatsBodyViewModel::Failed(
                i18n::with_error(Key::TplAgentsStatsFailed, reason),
            ),
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
    pub(crate) facts: Vec<(String, String)>,
    /// The tokens of each model.
    pub(crate) models: ModelsTableViewModel,
    /// Each usage window as its name and "12 % → 31 %"; empty for an agent
    /// that writes no limits into its sessions.
    pub(crate) limits: Vec<(String, String)>,
}

impl SessionStatsViewModel {
    /// The statistics `stats` of a session of `profile`; the money only
    /// when the profile pays with an API key.
    fn new(
        state: &AgentsState,
        profile: &Profile,
        stats: &SessionStats,
    ) -> Self {
        let offset = state.utc_offset_seconds();
        let prices = is_api_key(state, profile).then(|| state.prices());
        let length = stats
            .length()
            .map_or_else(|| String::from(UNKNOWN), length_text);

        let facts = vec![
            fact(
                Key::AgentsFieldAgent,
                state.agent_name(profile.agent()).to_string(),
            ),
            fact(Key::AgentsColumnProfile, profile.name().to_string()),
            fact(
                Key::AgentsStatsStarted,
                time_text(stats.started_at(), offset),
            ),
            fact(
                Key::AgentsStatsLastActivity,
                time_text(stats.last_active_at(), offset),
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
                let text = i18n::agents_template(
                    Key::TplAgentsLimitChange,
                    &[("start", &start), ("end", &end)],
                );
                (change.name().to_string(), text)
            })
            .collect();

        Self {
            facts,
            models: ModelsTableViewModel::new(stats.models(), prices),
            limits,
        }
    }
}

/// The statistics of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectStatsViewModel {
    /// Sessions, first and last activity, and the money of the API key
    /// profiles when there is one.
    pub(crate) facts: Vec<(String, String)>,
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
        let offset = state.utc_offset_seconds();
        let project = state.selected_project();
        let sessions_of = |profile: Option<&Profile>| {
            project.map_or(0, |project| {
                project
                    .sessions()
                    .iter()
                    .filter(|entry| {
                        profile.is_none_or(|p| entry.profile() == p)
                    })
                    .count()
            })
        };
        let all = ProjectStats::merged(parts.iter().map(|(_, part)| part));
        let money: Vec<Option<MoneyTotal>> = parts
            .iter()
            .map(|(profile, part)| {
                is_api_key(state, profile).then(|| {
                    MoneyTotal::of_models(part.models(), state.prices())
                })
            })
            .collect();
        let has_cost = money.iter().any(Option::is_some);

        let mut facts = vec![
            fact(Key::AgentsFieldSessions, format!("{}", sessions_of(None))),
            fact(
                Key::AgentsStatsFirstActivity,
                time_text(all.first_at(), offset),
            ),
            fact(
                Key::AgentsStatsLastActivity,
                time_text(all.last_at(), offset),
            ),
        ];
        if has_cost {
            let sum = money
                .iter()
                .flatten()
                .fold(MoneyTotal::default(), |sum, total| sum + *total);
            facts.push(fact(Key::AgentsColumnCost, dollars_text(sum.cost())));
        }
        let rows = parts
            .iter()
            .zip(&money)
            .map(|((profile, part), money)| ProfileStatsRowViewModel {
                name: profile.name().to_string(),
                sessions: format!("{}", sessions_of(Some(profile))),
                tokens: short_count(part.tokens().total()),
                cost: money.map(cost_text).unwrap_or_default(),
            })
            .collect();

        Self {
            facts,
            models: ModelsTableViewModel::new(all.models(), None),
            profiles: ProfilesTableViewModel { has_cost, rows },
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
    /// The table of `models`, priced at `prices` when there are prices:
    /// a model with no price says so, and the row of all models sums the
    /// money of those with one.
    fn new(models: &[ModelTokens], prices: Option<&PriceTable>) -> Self {
        let cost_of = |models: &[ModelTokens]| {
            prices
                .map(|prices| MoneyTotal::of_models(models, prices))
                .map(cost_text)
                .unwrap_or_default()
        };

        let mut rows: Vec<ModelRowViewModel> = models
            .iter()
            .map(|model| {
                let name =
                    model.model().unwrap_or(i18n::t(Key::AgentsUnknownModel));
                let cost = cost_of(std::slice::from_ref(model));
                ModelRowViewModel::new(name, model.tokens(), cost)
            })
            .collect();
        if models.len() > 1 {
            let sum = models
                .iter()
                .fold(TokenCount::default(), |sum, model| sum + model.tokens());
            let cost = prices
                .map(|prices| MoneyTotal::of_models(models, prices).cost())
                .map(dollars_text)
                .unwrap_or_default();
            rows.push(ModelRowViewModel::new(
                i18n::t(Key::AgentsStatsAllModels),
                sum,
                cost,
            ));
        }

        Self {
            has_cost: prices.is_some(),
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
    /// What they cost at list prices, "no price", or empty when the table
    /// has no Cost column.
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

/// The fact `key` with its `value`.
fn fact(key: Key, value: String) -> (String, String) {
    (i18n::t(key).to_string(), value)
}

/// Whether `profile` pays per token with an API key, as its files tell.
fn is_api_key(state: &AgentsState, profile: &Profile) -> bool {
    state
        .profile_state(profile)
        .is_some_and(|shown| shown.is_api_key())
}

/// What a total cost: its dollars, or "no price" when a request in it had
/// none.
fn cost_text(total: MoneyTotal) -> String {
    if total.unpriced_requests() > 0 {
        return i18n::t(Key::AgentsNoPrice).to_string();
    }

    dollars_text(total.cost())
}

/// The date and the clock time of `time`, such as "Nov 16, 2026, 10:00",
/// in a time zone `utc_offset_seconds` east of UTC, or in UTC when it is
/// not known; "—" for no time.
fn time_text(
    time: Option<SystemTime>,
    utc_offset_seconds: Option<i32>,
) -> String {
    let Some(time) = time else {
        return String::from(UNKNOWN);
    };

    let local = LocalNow::new(time, utc_offset_seconds.unwrap_or(0));
    let date = local.date_of(time);
    let day = format!("{}", date.day());
    let month_day = i18n::agents_template(
        Key::TplAgentsMonthDay,
        &[("month", i18n::agents_month(date.month())), ("day", &day)],
    );
    let year = format!("{}", date.year());
    let clock = clock_text(&local, time, utc_offset_seconds.is_some());

    i18n::agents_template(
        Key::TplAgentsDateTime,
        &[("date", &month_day), ("year", &year), ("time", &clock)],
    )
}

/// How long `length` is, in hours and minutes, such as "2 h 10 min".
fn length_text(length: Duration) -> String {
    duration_text(length.as_secs())
}

/// A percent rounded to a whole one, such as "31".
fn whole_percent(percent: f64) -> String {
    let rounded = percent.clamp(0.0, 100.0).round() as u8;

    format!("{rounded}")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{CalendarDate, LimitChange, ModelPrice, SessionId};

    use super::*;
    use crate::event::{AgentsIntent, SessionAction};
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, claude, codex, connected, read, session, with_profiles,
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
        PriceTable::new(
            CalendarDate::new(2026, 11, 16),
            vec![
                ModelPrice::new("opus", DOLLAR, 0, 0, 0),
                ModelPrice::new("gpt-5", 2 * DOLLAR, 0, 0, 0),
            ],
        )
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
        let mut state =
            with_profiles(&[(claude(), status), (codex(), connected())]);
        state.set_prices(prices());
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
        stats: Result<SessionStats, String>,
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
            session_stats_loaded(Err(String::from("not available: no file"))),
        );

        assert_eq!(
            body(&state),
            StatsBodyViewModel::Failed(i18n::with_error(
                Key::TplAgentsStatsFailed,
                "not available: no file"
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
        let date = i18n::agents_template(
            Key::TplAgentsMonthDay,
            &[("month", i18n::agents_month(11)), ("day", "16")],
        );
        let when = |time: &str| {
            i18n::agents_template(
                Key::TplAgentsDateTime,
                &[("date", &date), ("year", "2026"), ("time", time)],
            )
        };
        assert_eq!(
            vm.facts,
            [
                (Key::AgentsFieldAgent, String::from("Claude Code")),
                (Key::AgentsColumnProfile, String::from("Claude Code")),
                (Key::AgentsStatsStarted, when("10:00")),
                (Key::AgentsStatsLastActivity, when("12:10")),
                (
                    Key::AgentsStatsLength,
                    i18n::agents_template(
                        Key::TplAgentsDurationHoursMinutes,
                        &[("hours", "2"), ("minutes", "10")],
                    )
                ),
                (Key::AgentsStatsMessages, String::from("4")),
                (Key::AgentsStatsAnswers, String::from("5")),
            ]
            .map(|(key, value)| (i18n::t(key).to_string(), value))
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
        let started = &vm.facts[2].1;
        assert_eq!(started, UNKNOWN);
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
                LimitChange::new("5 hours", 12.0, 31.4),
                LimitChange::new("7 days", 40.0, 42.5),
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
            i18n::agents_template(
                Key::TplAgentsLimitChange,
                &[("start", start), ("end", end)],
            )
        };
        assert_eq!(
            vm.limits,
            [
                (String::from("5 hours"), change("12", "31")),
                (String::from("7 days"), change("40", "43")),
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
        let date = i18n::agents_template(
            Key::TplAgentsMonthDay,
            &[("month", i18n::agents_month(11)), ("day", "16")],
        );
        let when = |time: &str| {
            i18n::agents_template(
                Key::TplAgentsDateTime,
                &[("date", &date), ("year", "2026"), ("time", time)],
            )
        };
        assert_eq!(
            vm.facts,
            [
                (Key::AgentsFieldSessions, String::from("2")),
                (Key::AgentsStatsFirstActivity, when("08:00")),
                (Key::AgentsStatsLastActivity, when("10:00")),
                (Key::AgentsColumnCost, String::from("$1.00")),
            ]
            .map(|(key, value)| (i18n::t(key).to_string(), value))
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
        assert_eq!(vm.facts[1].1, UNKNOWN);
        assert_eq!(vm.models.rows.len(), 1);
    }
}
