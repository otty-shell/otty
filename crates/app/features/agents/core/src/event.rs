use std::path::PathBuf;

use otty_agents::{AgentCommand, AgentId, Profile, SavedProfiles, SessionId};
use otty_app_domain::mode::Mode;

use super::profile_dialog::ProfileDialogIntent;
use super::types::{LoadedLimits, LoadedProfiles, LoadedSessions, LoadedSpend};

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
    /// The limits of one profile were read.
    LimitsLoaded(LoadedLimits),
    /// The tokens one profile spent were counted.
    SpendLoaded(LoadedSpend),
    /// Five minutes passed while Agents mode is on screen: read the limits
    /// of the connected profiles again.
    LimitsTick,
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
    /// A profile of the left panel was pressed: show its details.
    SelectProfile(Profile),
    /// The Hidden profiles row was pressed: list the hidden profiles or
    /// stop listing them.
    ToggleHiddenProfiles,
    /// Log in of a profile was pressed: the agent logs in in a terminal
    /// tab.
    LogIn(Profile),
    /// Hide of a profile was pressed: take it and its sessions off the
    /// screen.
    HideProfile(Profile),
    /// Show of a hidden profile was pressed.
    ShowProfile(Profile),
    /// Remove of an added profile was pressed: ask first.
    RemoveProfile(Profile),
    /// The removal was confirmed.
    RemoveConfirmed,
    /// The removal was cancelled.
    RemoveCancelled,
    /// Add profile was pressed: open the profile dialog to add one.
    OpenAddProfile,
    /// Edit of a profile was pressed: open the profile dialog with it.
    EditProfile(Profile),
    /// The profile dialog was used.
    Dialog(ProfileDialogIntent),
    /// The folders that look like a config folder of `agent` were found.
    CandidatesLoaded {
        /// The agent the folders were looked for.
        agent: AgentId,
        /// The folders, sorted by path.
        folders: Vec<PathBuf>,
    },
    /// `agents.json` was written; the error is ready to show.
    ProfilesSaved(Result<(), String>),
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
    /// Read the limits of this profile; answer
    /// [`AgentsIntent::LimitsLoaded`].
    LoadLimits(Profile),
    /// Count the tokens this profile spent; answer
    /// [`AgentsIntent::SpendLoaded`].
    LoadSpend(Profile),
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
    /// Open a terminal tab named `title` that runs the login `command` of a
    /// profile in the home folder, then show Terminal mode.
    OpenLoginTab {
        /// The title of the tab: the profile name.
        title: String,
        /// The program, arguments and variables that log the profile in.
        command: AgentCommand,
    },
    /// Find the folders in the home folder that look like a config folder
    /// of `agent` and are not one of `profiles` yet; answer
    /// [`AgentsIntent::CandidatesLoaded`].
    LoadCandidates {
        /// The agent picked in the profile dialog.
        agent: AgentId,
        /// Every profile, hidden ones too.
        profiles: Vec<Profile>,
    },
    /// Create the folder `create_dir` when there is one, then write `saved`
    /// to `agents.json`; answer [`AgentsIntent::ProfilesSaved`]. A config
    /// folder is never deleted.
    SaveProfiles {
        /// The whole new content of `agents.json`.
        saved: SavedProfiles,
        /// The config folder of a new profile to create first.
        create_dir: Option<PathBuf>,
    },
    /// Tell the user about an error; the text is ready to show.
    Failed(String),
}
