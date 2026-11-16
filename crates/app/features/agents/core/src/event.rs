use std::path::PathBuf;

use otty_agents::{AgentCommand, Profile, SessionId};
use otty_app_domain::mode::Mode;

use super::types::{LoadedProfiles, LoadedSessions};

/// What the agents feature is asked to do: a press in its views, or the
/// answer of a read the binary ran for it.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentsIntent {
    /// The active mode changed: Agents mode enters, any other mode leaves
    /// it.
    ModeChanged(Mode),
    /// Agents mode became active: read the profiles and their sessions.
    Enter,
    /// Another mode became active.
    Leave,
    /// Read the profiles and their sessions again (the Refresh button).
    Refresh,
    /// The profiles were read.
    ProfilesLoaded(LoadedProfiles),
    /// The sessions of one profile were read.
    SessionsLoaded(LoadedSessions),
    /// A project of the left panel was pressed: show its sessions.
    SelectProject(PathBuf),
    /// The Continue button of a session was pressed: continue it in a
    /// terminal tab.
    Continue {
        /// The project folder the session ran in.
        folder: PathBuf,
        /// The session to continue.
        session: SessionId,
    },
    /// The New session button of the selected project was pressed. With
    /// more than one profile, the feature asks which one to start.
    NewSession,
    /// A profile was chosen for the new session.
    NewSessionWith(Profile),
    /// The choice of a profile for the new session was cancelled.
    CancelNewSession,
}

/// What the agents feature reports to the binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentsOutcome {
    /// Read `agents.json`, find the profiles and check each one; answer
    /// [`AgentsIntent::ProfilesLoaded`].
    LoadProfiles,
    /// Read the sessions of this profile; answer
    /// [`AgentsIntent::SessionsLoaded`].
    LoadSessions(Profile),
    /// Open a terminal tab named `title` that runs `command` in `folder`,
    /// then show Terminal mode.
    OpenTab {
        /// The title of the tab: the session title or the profile name.
        title: String,
        /// The project folder the agent works in.
        folder: PathBuf,
        /// The program, arguments and variables that start the agent.
        command: AgentCommand,
    },
    /// Tell the user about an error; the text is ready to show.
    Failed(String),
}
