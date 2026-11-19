//! Agents mode in the states the tests start from: made-up profiles and
//! sessions, told to the feature as if they were read. Nothing is read
//! from disk and no agent's program runs.

use std::path::PathBuf;
use std::time::SystemTime;

use otty_agents::{AgentId, AgentSession, Profile, ProfileStatus, SessionId};
use otty_app_agents::{
    AgentsCtx, AgentsIntent, LoadedProfile, LoadedProfiles, LoadedSessions,
    SessionsRead,
};
use otty_app_domain::mode::Mode;
use otty_app_modes::ModesIntent;
use otty_app_slots::Component;

use crate::app::App;
use crate::dispatch;
use crate::message::AppEvent;

/// The project folder of the made-up sessions.
pub(crate) const PROJECT: &str = "/work/otty";
/// The title of the made-up sessions.
pub(crate) const SESSION_TITLE: &str = "Fix the tabs";

/// The default Claude Code profile in a made-up home folder.
pub(crate) fn claude_profile() -> Profile {
    Profile::default_for(
        AgentId::new("claude-code"),
        "Claude Code",
        PathBuf::from("/home/user/.claude"),
    )
}

/// The news that `profiles` were read, each with its state, and an
/// `agents.json` that says the Found by OTTY report was shown, so it does
/// not open over the tests.
pub(crate) fn profiles_loaded(
    profiles: Vec<(Profile, ProfileStatus)>,
) -> AgentsIntent {
    loaded(
        profiles,
        otty_agents::SavedProfiles::default().with_found_report_shown(),
    )
}

/// The news that `profiles` were read on the first start: an empty
/// `agents.json`, so the Found by OTTY report opens.
pub(crate) fn first_start_profiles_loaded(
    profiles: Vec<(Profile, ProfileStatus)>,
) -> AgentsIntent {
    loaded(profiles, otty_agents::SavedProfiles::default())
}

/// The news that `profile` has one session, `id`, titled
/// [`SESSION_TITLE`] in [`PROJECT`].
pub(crate) fn one_session_loaded(profile: Profile, id: &str) -> AgentsIntent {
    let session = AgentSession::new(
        SessionId::new(id),
        PathBuf::from(PROJECT),
        SESSION_TITLE,
        SystemTime::UNIX_EPOCH,
    );

    AgentsIntent::SessionsLoaded(LoadedSessions {
        profile,
        sessions: SessionsRead::Read(vec![session]),
        missing_folders: Vec::new(),
    })
}

/// Show Agents mode in `app` and tell it that [`claude_profile`] was read,
/// not logged in: a session can start with it.
pub(crate) fn show_claude(app: &mut App) {
    let _ = dispatch::dispatch(
        app,
        AppEvent::Modes(ModesIntent::Select(Mode::Agents)),
    );
    let loaded =
        profiles_loaded(vec![(claude_profile(), ProfileStatus::NotLoggedIn)]);

    let _ = dispatch::dispatch(app, AppEvent::Agents(loaded));
}

/// Show Agents mode in `app` as [`show_claude`] does, with one Claude Code
/// session, `c1`, in [`PROJECT`].
pub(crate) fn show_claude_with_a_session(app: &mut App) {
    show_claude(app);

    let _ = app.agents.reduce(
        one_session_loaded(claude_profile(), "c1"),
        AgentsCtx::default(),
    );
}

/// The news that `profiles` were read, each with its state, and `saved`
/// in `agents.json`.
fn loaded(
    profiles: Vec<(Profile, ProfileStatus)>,
    saved: otty_agents::SavedProfiles,
) -> AgentsIntent {
    let profiles = profiles
        .into_iter()
        .map(|(profile, status)| LoadedProfile {
            profile,
            status,
            accounts: Vec::new(),
            version: None,
        })
        .collect();

    AgentsIntent::ProfilesLoaded(LoadedProfiles {
        profiles,
        hidden_profiles: Vec::new(),
        saved: Some(saved),
        problem: None,
        time_zone: None,
        read_at: SystemTime::UNIX_EPOCH,
    })
}
