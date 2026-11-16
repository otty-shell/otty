use std::path::PathBuf;

use otty_agents::{AgentCommand, AgentId, Profile, SavedProfiles};
use otty_app_domain::mode::Mode;
use otty_ui_kit::components::tag_search::TagSearchAction;

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
    /// The user did something in the search over the sessions table of the
    /// selected project.
    SessionSearch(TagSearchAction),
    /// The title of the column at this position of the sessions table was
    /// pressed.
    SortSessions(usize),
    /// The session at this position of the shown (searched and sorted)
    /// sessions table was pressed: select it.
    SelectSession(usize),
    /// The session at this position of the shown sessions table was
    /// double-pressed: select it and continue it in a terminal tab.
    OpenSession(usize),
    /// Enter was pressed and no widget took it: continue the selected
    /// session of the project on screen in a terminal tab.
    ContinueSelected,
    /// The session at this position of the shown sessions table was
    /// right-pressed, or its "⋯" was pressed: select it and open its menu
    /// at the pointer.
    OpenSessionMenu(usize),
    /// An item of the menu of a session was pressed.
    SessionMenu(SessionAction),
    /// The button of the title row that opens the project folder in a
    /// terminal tab was pressed.
    OpenProjectFolder,
    /// The plus of New session was pressed: start a session in the
    /// selected project with the only profile that can, or open the list
    /// of the profiles at the pointer when several can.
    NewSession,
    /// A profile was chosen for the new session.
    NewSessionWith(Profile),
    /// A profile of the left panel was pressed: show its dashboard.
    SelectProfile(Profile),
    /// The Hidden profiles row was pressed: list the hidden profiles or
    /// stop listing them.
    ToggleHiddenProfiles,
    /// Log in of a profile was pressed: the agent logs in in a terminal
    /// tab.
    LogIn(Profile),
    /// The "⋯" of a profile's dashboard was pressed: open its menu, Hide
    /// and Remove, at the pointer.
    OpenProfileMenu(Profile),
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
    /// Enter in the search field: select the first project the filtered
    /// list shows, whose folder this is; `None` when the list shows none.
    ProjectSearchSubmitted(Option<PathBuf>),
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
    /// Split the focused pane of the terminal tab in front, run `command`
    /// in `folder` in the new pane, then show Terminal mode. The feature
    /// reports it only while that tab is a terminal tab on this computer
    /// ([`AgentsCtx::has_local_pane`](crate::AgentsCtx)).
    OpenSplit {
        /// The project folder the agent works in.
        folder: PathBuf,
        /// The program, arguments and variables that continue the session.
        command: AgentCommand,
    },
    /// Open a terminal tab with the user's shell in this folder, as Open in
    /// terminal of Files does, then show Terminal mode.
    OpenFolder(PathBuf),
    /// Put the line that runs `command` in a shell on the clipboard: its
    /// variables, then the program and its arguments, each quoted.
    CopyCommand(AgentCommand),
    /// Put this text on the clipboard.
    CopyText(String),
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

/// An item of the menu of a session in the sessions table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAction {
    /// Continue the session in a new terminal tab.
    Continue,
    /// Continue the session in a new pane next to the focused pane of the
    /// terminal tab in front, or in a new tab when there is none.
    ContinueInSplit,
    /// Copy the command that continues the session.
    CopyResumeCommand,
    /// Copy the id of the session.
    CopySessionId,
    /// Open the project folder in a terminal tab with the user's shell.
    OpenFolder,
}
