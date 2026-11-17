//! Helpers that the tests of several files share: made-up profiles and
//! sessions, and Agents mode with them read.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use otty_agents::{
    AdminKeyRef, AgentEnv, AgentId, AgentSession, Answer, Bill, Billing,
    CalendarDate, DayCost, DaySpend, Dollars, FetchedPrices, LimitSource,
    LimitWindow, Limits, ModelPrice, ModelTokens, PriceTable, Profile,
    ProfileStatus, SavedProfiles, SessionId, SourceError, SourceFailure,
    SourceId, Spend, TokenCount, UsageWindows,
};

use crate::event::AgentsIntent;
use crate::reducer::reduce;
use crate::state::AgentsState;
use crate::types::{
    AgentList, LoadedBill, LoadedLimits, LoadedPrices, LoadedProfile,
    LoadedProfiles, LoadedSessions, LoadedSpend,
};

pub(crate) const CLAUDE: AgentId = AgentId::new("claude-code");
pub(crate) const CODEX: AgentId = AgentId::new("codex");

/// A session source that never answers.
struct NoFiles;

impl otty_agents::SessionSource for NoFiles {
    fn id(&self) -> SourceId {
        SourceId::SessionFiles
    }

    fn sessions(
        &self,
        _profile: &Profile,
    ) -> Result<Vec<AgentSession>, SourceError> {
        Err(SourceError::NotAvailable(String::from(
            "no projects folder",
        )))
    }
}

/// A limit source that gives the same answer each time.
struct FixedLimits {
    id: SourceId,
    answer: Result<Limits, SourceError>,
}

impl LimitSource for FixedLimits {
    fn id(&self) -> SourceId {
        self.id
    }

    fn limits(&self, _profile: &Profile) -> Result<Limits, SourceError> {
        self.answer.clone()
    }
}

/// The time the tests look at the sessions from.
pub(crate) fn now() -> SystemTime {
    at(1_000_000)
}

/// The time `seconds` after 1970.
pub(crate) fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

/// The agents OTTY knows: Claude Code and Codex.
pub(crate) fn agents() -> AgentList {
    Arc::from(otty_agents::known_agents())
}

/// The environment of the tests: the home folder `/home/user`, no
/// variable of an agent.
pub(crate) fn env() -> AgentEnv {
    AgentEnv::new(PathBuf::from("/home/user"), None, None)
}

/// The default Claude Code profile in `/home/user/.claude`.
pub(crate) fn claude() -> Profile {
    Profile::default_for(
        CLAUDE,
        "Claude Code",
        PathBuf::from("/home/user/.claude"),
    )
}

/// A Claude Code profile the user added, in `/home/user/.claude-work`.
pub(crate) fn claude_work() -> Profile {
    Profile::added(
        CLAUDE,
        "Claude Code · Work",
        PathBuf::from("/home/user/.claude-work"),
    )
}

/// The default Codex profile in `/home/user/.codex`.
pub(crate) fn codex() -> Profile {
    Profile::default_for(CODEX, "Codex", PathBuf::from("/home/user/.codex"))
}

/// A Codex profile the user added, in `/home/user/.codex-work`.
pub(crate) fn codex_work() -> Profile {
    Profile::added(
        CODEX,
        "Codex · Work",
        PathBuf::from("/home/user/.codex-work"),
    )
}

/// The status of a profile that can be used.
pub(crate) fn connected() -> ProfileStatus {
    ProfileStatus::Connected {
        account: None,
        plan: None,
        billing: None,
    }
}

/// The status of a profile that pays per token with an API key.
pub(crate) fn api_key() -> ProfileStatus {
    ProfileStatus::Connected {
        account: None,
        plan: None,
        billing: Some(otty_agents::BillingKind::ApiKey),
    }
}

/// A session `id` titled after it, in `folder`, changed at `seconds`.
pub(crate) fn session(id: &str, folder: &str, seconds: u64) -> AgentSession {
    AgentSession::new(
        SessionId::new(id),
        PathBuf::from(folder),
        format!("Title of {id}"),
        at(seconds),
    )
}

/// A session `id` titled `title` in `/work/otty`, changed at `seconds`.
pub(crate) fn titled_session(
    id: &str,
    title: &str,
    seconds: u64,
) -> AgentSession {
    AgentSession::new(
        SessionId::new(id),
        PathBuf::from("/work/otty"),
        title,
        at(seconds),
    )
}

/// A failure of the session files of a profile.
pub(crate) fn no_files() -> SourceFailure {
    let sources: Vec<Box<dyn otty_agents::SessionSource>> =
        vec![Box::new(NoFiles)];
    let profile = claude();

    match otty_agents::read_sessions(&sources, &profile) {
        Err(failures) => failures.into_iter().next().expect("one failure"),
        Ok(_) => unreachable!("the source never answers"),
    }
}

/// Agents mode entered, with `profiles` read.
pub(crate) fn with_profiles(
    profiles: &[(Profile, ProfileStatus)],
) -> AgentsState {
    with_saved_profiles(profiles, SavedProfiles::default())
}

/// Agents mode entered, with `profiles` read from `saved`.
pub(crate) fn with_saved_profiles(
    profiles: &[(Profile, ProfileStatus)],
    saved: SavedProfiles,
) -> AgentsState {
    let mut state = AgentsState::new(agents(), env());
    let _ = reduce(&mut state, AgentsIntent::Enter);
    let _ = reduce(&mut state, profiles_loaded_from(profiles, saved));

    state
}

/// The answer of a profiles read that found `profiles`.
pub(crate) fn profiles_loaded(
    profiles: &[(Profile, ProfileStatus)],
) -> AgentsIntent {
    profiles_loaded_from(profiles, SavedProfiles::default())
}

/// The answer of a profiles read that found `profiles` and `saved` in
/// `agents.json`.
pub(crate) fn profiles_loaded_from(
    profiles: &[(Profile, ProfileStatus)],
    saved: SavedProfiles,
) -> AgentsIntent {
    AgentsIntent::ProfilesLoaded(loaded_profiles(profiles, saved))
}

/// The answer of a profiles read that found `profiles` just now.
pub(crate) fn fresh_profiles_loaded(
    profiles: &[(Profile, ProfileStatus)],
) -> AgentsIntent {
    AgentsIntent::ProfilesLoaded(LoadedProfiles {
        read_at: SystemTime::now(),
        ..loaded_profiles(profiles, SavedProfiles::default())
    })
}

/// The answer of a profiles read that found `profiles` and could not read
/// `agents.json` for the reason `problem`, when there is one.
pub(crate) fn profiles_loaded_with_problem(
    profiles: &[(Profile, ProfileStatus)],
    problem: Option<&str>,
) -> AgentsIntent {
    AgentsIntent::ProfilesLoaded(LoadedProfiles {
        problem: problem.map(str::to_string),
        ..loaded_profiles(profiles, SavedProfiles::default())
    })
}

/// The answer of a sessions read of `profile` that found `sessions`.
pub(crate) fn sessions_loaded(
    profile: &Profile,
    sessions: Vec<AgentSession>,
) -> AgentsIntent {
    AgentsIntent::SessionsLoaded(LoadedSessions {
        profile: profile.clone(),
        sessions: Ok(sessions),
        missing_folders: Vec::new(),
    })
}

/// Read `sessions` of `profile` into `state`.
pub(crate) fn read(
    state: &mut AgentsState,
    profile: &Profile,
    sessions: Vec<AgentSession>,
) {
    let _ = reduce(state, sessions_loaded(profile, sessions));
}

/// The default Claude Code and Codex profiles, connected, with sessions in
/// `/work/otty` from both and in `/work/site` from Claude Code.
pub(crate) fn two_agents_with_sessions() -> AgentsState {
    let mut state =
        with_profiles(&[(claude(), connected()), (codex(), connected())]);
    read(
        &mut state,
        &claude(),
        vec![
            session("c1", "/work/otty", 100),
            session("c2", "/work/site", 300),
        ],
    );
    read(&mut state, &codex(), vec![session("x1", "/work/otty", 200)]);

    state
}

/// The folders of the projects of `state`, in their order.
pub(crate) fn folders(state: &AgentsState) -> Vec<&Path> {
    state
        .projects()
        .iter()
        .map(|project| project.folder())
        .collect()
}

/// Limits with one window, "5 hours", `percent` used, that resets an hour
/// after [`now`].
pub(crate) fn five_hour_limits(percent: f64) -> Limits {
    let resets_at = now() + Duration::from_secs(3600);

    let window = LimitWindow::new("5 hours", percent, Some(resets_at));

    Limits::new(
        Billing::UsageWindows(UsageWindows::new(vec![window], None)),
        None,
    )
}

/// What the walk over `sources`, each an id and its answer, gives: the
/// answer of the first one that answers, or every failure.
pub(crate) fn limits_answer(
    sources: Vec<(SourceId, Result<Limits, SourceError>)>,
) -> Result<Answer<Limits>, Vec<SourceFailure>> {
    let sources: Vec<Box<dyn LimitSource>> = sources
        .into_iter()
        .map(|(id, answer)| {
            Box::new(FixedLimits { id, answer }) as Box<dyn LimitSource>
        })
        .collect();

    otty_agents::read_limits(&sources, &claude())
}

/// The answer of a limits read of `profile` that ended at `read_at`, whose
/// agent has only the sources the walk asked.
pub(crate) fn limits_loaded(
    profile: &Profile,
    limits: Result<Answer<Limits>, Vec<SourceFailure>>,
    read_at: SystemTime,
) -> AgentsIntent {
    let sources = match &limits {
        Ok(answer) => answer
            .earlier_failures()
            .iter()
            .map(SourceFailure::source)
            .chain([answer.source()])
            .collect(),
        Err(failures) => failures.iter().map(SourceFailure::source).collect(),
    };

    limits_loaded_from(profile, sources, limits, read_at)
}

/// The answer of a limits read of `profile` over `sources`, best first,
/// that ended at `read_at`.
pub(crate) fn limits_loaded_from(
    profile: &Profile,
    sources: Vec<SourceId>,
    limits: Result<Answer<Limits>, Vec<SourceFailure>>,
    read_at: SystemTime,
) -> AgentsIntent {
    AgentsIntent::LimitsLoaded(LoadedLimits {
        profile: profile.clone(),
        limits,
        sources,
        read_at,
    })
}

/// Tokens spent: `today` input tokens today, 2026-11-16, and `earlier`
/// input tokens five days before.
pub(crate) fn spend_of(today: u64, earlier: u64) -> Spend {
    let day = |day: u8, input: u64| {
        let tokens = TokenCount::new(input, 0, 0);
        DaySpend::new(
            CalendarDate::new(2026, 11, day),
            vec![ModelTokens::new(Some("gpt-5"), tokens, 1)],
        )
    };

    Spend::new(vec![day(11, earlier), day(16, today)], Vec::new())
}

/// The answer of a count of the tokens of `profile` that ended at
/// `counted_at`.
pub(crate) fn spend_loaded(
    profile: &Profile,
    spend: Option<Result<Spend, SourceError>>,
    counted_at: SystemTime,
) -> AgentsIntent {
    AgentsIntent::SpendLoaded(LoadedSpend {
        profile: profile.clone(),
        spend,
        counted_at,
    })
}

/// The default Claude Code profile with its admin key in the variable
/// `ANTHROPIC_ADMIN_KEY`.
pub(crate) fn claude_with_admin_key() -> Profile {
    claude().with_admin_key(Some(AdminKeyRef::EnvVar(String::from(
        "ANTHROPIC_ADMIN_KEY",
    ))))
}

/// `amount` whole dollars.
pub(crate) fn dollars(amount: u64) -> Dollars {
    Dollars::from_micro_dollars(amount * 1_000_000)
}

/// A bill of `today` dollars today, 2026-11-16, and `earlier` dollars
/// five days before.
pub(crate) fn bill_of(today: u64, earlier: u64) -> Bill {
    Bill::new(vec![
        DayCost::new(CalendarDate::new(2026, 11, 11), dollars(earlier)),
        DayCost::new(CalendarDate::new(2026, 11, 16), dollars(today)),
    ])
}

/// The answer of a read of the bill of `profile` that ended at [`now`].
pub(crate) fn bill_loaded(
    profile: &Profile,
    bill: Result<Bill, SourceError>,
) -> AgentsIntent {
    AgentsIntent::BillLoaded(LoadedBill {
        profile: profile.clone(),
        bill,
        read_at: now(),
    })
}

/// Made-up list prices, in micro-dollars per million tokens: `gpt-5`
/// costs $1.25 per million input tokens and $10 per million output
/// tokens, `claude-sonnet-4-5` $3 and $15.
pub(crate) fn price_table() -> PriceTable {
    PriceTable::new(vec![
        ModelPrice::new("gpt-5", 1_250_000, 125_000, 1_250_000, 10_000_000),
        ModelPrice::new(
            "claude-sonnet-4-5",
            3_000_000,
            300_000,
            3_750_000,
            15_000_000,
        ),
    ])
}

/// The made-up prices of [`price_table`], fetched at `fetched_at`.
pub(crate) fn fetched_prices(fetched_at: SystemTime) -> FetchedPrices {
    FetchedPrices::new(price_table(), fetched_at)
}

/// What a walk over the price sources gave: the sources of `failures`
/// failed in order, then `prices` answered, from LiteLLM when nothing
/// failed before, else from the kept file.
pub(crate) fn prices_answer(
    failures: &[(SourceId, SourceError)],
    prices: Option<FetchedPrices>,
) -> Result<Answer<FetchedPrices>, Vec<SourceFailure>> {
    let answering = if failures.is_empty() {
        SourceId::LiteLlmPrices
    } else {
        SourceId::KeptPrices
    };
    let failed = failures
        .iter()
        .map(|(source, error)| (*source, Err(error.clone())));
    let answered = prices.map(|prices| (answering, Ok(prices)));

    otty_agents::walk(failed.chain(answered))
}

/// The answer of a read of the prices that ended at `read_at`.
pub(crate) fn prices_loaded(
    prices: Result<Answer<FetchedPrices>, Vec<SourceFailure>>,
    read_at: SystemTime,
) -> AgentsIntent {
    AgentsIntent::PricesLoaded(LoadedPrices { prices, read_at })
}

/// What a profiles read long ago found: `profiles` and `saved` in
/// `agents.json`.
fn loaded_profiles(
    profiles: &[(Profile, ProfileStatus)],
    saved: SavedProfiles,
) -> LoadedProfiles {
    LoadedProfiles {
        profiles: profiles
            .iter()
            .map(|(profile, status)| LoadedProfile {
                profile: profile.clone(),
                status: status.clone(),
            })
            .collect(),
        saved: Some(saved),
        problem: None,
        utc_offset_seconds: Some(0),
        read_at: at(0),
    }
}

/// A press and a release of the right mouse button where the pointer is.
pub(crate) fn right_click() -> [iced::Event; 2] {
    use iced::mouse::{Button, Event};

    [
        iced::Event::Mouse(Event::ButtonPressed(Button::Right)),
        iced::Event::Mouse(Event::ButtonReleased(Button::Right)),
    ]
}
