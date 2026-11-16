use std::path::PathBuf;

use otty_agents::{AgentCommand, AgentId, Profile, SavedProfiles, SessionId};
use otty_app_domain::mode::Mode;

use super::profile_dialog::ProfileDialogIntent;
use super::project_filter::ProjectFilterChange;
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
    /// The title of the Agents section of the left panel was pressed: open
    /// or close the section.
    ToggleAgentsSection,
    /// The title of the Projects section of the left panel was pressed:
    /// open or close the section.
    ToggleProjectsSection,
    /// The search button of the Projects section was pressed: open the
    /// search field under the header, or close it.
    ToggleProjectSearch,
    /// The text of the search field changed.
    ProjectSearchChanged(String),
    /// Esc in the search field or its cross: clear the text, or close the
    /// field when it has none.
    ProjectSearchCleared,
    /// The funnel next to the search field was pressed: open its menu at
    /// the pointer.
    OpenFilterMenu,
    /// A choice of the funnel menu, or the cross of a chip.
    ProjectFilter(ProjectFilterChange),
    /// The chevron of New session was pressed: open the list of the
    /// profiles a session can start with, at the pointer.
    OpenNewSessionMenu,
    /// A press away from the open menu, or Esc: close it.
    CloseMenu,
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
    /// Move the keyboard focus to the search field of the projects, whose
    /// id is [`PROJECT_SEARCH`](crate::PROJECT_SEARCH).
    FocusProjectSearch,
    /// A menu opened. It takes the keyboard, so typed keys do not reach the
    /// search field under it and Esc reaches the frame, which closes it.
    MenuOpened,
}
