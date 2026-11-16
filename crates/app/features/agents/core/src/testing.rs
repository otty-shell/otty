//! Helpers that the tests of several files share: made-up profiles and
//! sessions, and Agents mode with them read.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use otty_agents::{
    AgentId, AgentSession, Profile, ProfileStatus, SessionId, SourceError,
    SourceFailure,
};

use crate::event::AgentsIntent;
use crate::reducer::reduce;
use crate::state::AgentsState;
use crate::types::{AgentList, LoadedProfile, LoadedProfiles, LoadedSessions};

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
    let mut state = AgentsState::new(agents());
    let _ = reduce(&mut state, AgentsIntent::Enter);
    let _ = reduce(&mut state, profiles_loaded(profiles));

    state
}

/// The answer of a profiles read that found `profiles`.
pub(crate) fn profiles_loaded(
    profiles: &[(Profile, ProfileStatus)],
) -> AgentsIntent {
    AgentsIntent::ProfilesLoaded(LoadedProfiles {
        profiles: profiles
            .iter()
            .map(|(profile, status)| LoadedProfile {
                profile: profile.clone(),
                status: status.clone(),
            })
            .collect(),
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
