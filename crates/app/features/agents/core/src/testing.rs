//! Helpers that the tests of several files share: made-up profiles and
//! sessions, and Agents mode with them read.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use otty_agents::{
    AgentEnv, AgentId, AgentSession, Answer, Billing, CalendarDate, DaySpend,
    LimitSource, LimitWindow, Limits, ModelTokens, Profile, ProfileStatus,
    SavedProfiles, SessionId, SourceError, SourceFailure, Spend, TokenCount,
    UsageWindows,
};

use crate::event::AgentsIntent;
use crate::reducer::reduce;
use crate::state::AgentsState;
use crate::types::{
    AgentList, LoadedLimits, LoadedProfile, LoadedProfiles, LoadedSessions,
    LoadedSpend,
};

pub(crate) const CLAUDE: AgentId = AgentId::new("claude-code");
pub(crate) const CODEX: AgentId = AgentId::new("codex");

/// A session source that never answers.
struct NoFiles;

impl otty_agents::SessionSource for NoFiles {
    fn name(&self) -> &'static str {
        "files"
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
    name: &'static str,
    answer: Result<Limits, SourceError>,
}

impl LimitSource for FixedLimits {
    fn name(&self) -> &'static str {
        self.name
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
    AgentsIntent::ProfilesLoaded(LoadedProfiles {
        profiles: profiles
            .iter()
            .map(|(profile, status)| LoadedProfile {
                profile: profile.clone(),
                status: status.clone(),
            })
            .collect(),
        saved: Some(saved),
        problem: None,
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

/// What the walk over `sources`, each a name and its answer, gives: the
/// answer of the first one that answers, or every failure.
pub(crate) fn limits_answer(
    sources: Vec<(&'static str, Result<Limits, SourceError>)>,
) -> Result<Answer<Limits>, Vec<SourceFailure>> {
    let sources: Vec<Box<dyn LimitSource>> = sources
        .into_iter()
        .map(|(name, answer)| {
            Box::new(FixedLimits { name, answer }) as Box<dyn LimitSource>
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
    sources: Vec<&'static str>,
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
