use std::path::PathBuf;

use otty_agents::{
    Account, AgentCommand, AgentId, GivenKey, Profile, SavedProfiles, SessionId,
};
use otty_app_domain::mode::Mode;
use otty_app_domain::plugins::{OffPart, PluginNotes, PluginsPlace};
use otty_ui_kit::components::tag_search::TagSearchAction;

use super::key_dialog::KeyDialogIntent;
use super::need::NeedAction;
use super::new_session_dialog::NewSessionDialogIntent;
use super::profile_dialog::ProfileDialogIntent;
use super::project_filter::ProjectFilterChange;
use super::rename_dialog::RenameIntent;
use super::types::{
    AgentReaders, FolderQuery, LoadedAccountNumbers, LoadedBill, LoadedFolder,
    LoadedLimits, LoadedPrices, LoadedProfiles, LoadedProjectStats,
    LoadedSessionStats, LoadedSessions, LoadedSpend, LoadedVersion,
};
use super::usage::{UsageMeasure, UsagePeriod, UsageTab};

/// What the agents feature is asked to do: a press in its views, or the
/// answer of a read the binary ran for it.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentsIntent {
    /// The active mode changed: Agents mode enters, any other mode leaves
    /// it.
    ModeChanged(Mode),
    /// OTTY started: read what Agents mode shows while the mode is not on
    /// screen, the profiles first, then the other kinds together (spec
    /// Decision 10).
    LoadInBackground,
    /// Agents mode became active: read again what is older than the period
    /// of its timer, or the profiles and everything else when they are.
    Enter,
    /// Another mode became active.
    Leave,
    /// Read the profiles and their sessions again (the Refresh button).
    Refresh,
    /// The installed plugins changed: read with these agents and this
    /// registry from now on, and read the profiles again.
    ReadersChanged(AgentReaders),
    /// What the mode shows of the installed plugins changed: the parts
    /// that are off and the updates that wait.
    PluginNotesChanged(PluginNotes),
    /// "Read by" of a page was pressed: open Plugins mode at this
    /// plugin.
    OpenPlugin(String),
    /// Manage plugins… of the card with no agent part was pressed: open
    /// Plugins mode with nothing selected.
    OpenPluginsPage,
    /// The profiles were read.
    ProfilesLoaded(LoadedProfiles),
    /// The read of the profiles stopped before it answered: the profiles
    /// shown stay and the user is told why.
    ProfilesReadStopped,
    /// The sessions of one profile were read.
    SessionsLoaded(LoadedSessions),
    /// The program of an agent told its version.
    VersionLoaded(LoadedVersion),
    /// Allow of the keyring dialog was pressed: the plugin parts may read
    /// the entries of the system keyring they declare from now on.
    AllowKeyring,
    /// Not now of the keyring dialog, or Esc, was pressed: nothing is read.
    KeyringNotNow,
    /// The limits of one profile were read.
    LimitsLoaded(LoadedLimits),
    /// The tokens one profile spent were counted.
    SpendLoaded(LoadedSpend),
    /// The bill of one API key profile was read.
    BillLoaded(LoadedBill),
    /// The list prices of the money at API prices were read.
    PricesLoaded(LoadedPrices),
    /// The numbers of one account were read with its own key: its money
    /// left, the limit of its key, the bill of its organization or the
    /// limits of its login.
    AccountNumbersLoaded(LoadedAccountNumbers),
    /// Five minutes passed while Agents mode is on screen: read the limits
    /// of the connected profiles and the bills of the API key profiles
    /// again.
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
    /// The profile link of the session at this position of the shown
    /// sessions table was pressed: open the page of its profile.
    OpenSessionProfile(usize),
    /// The button of the title row that opens the project folder in a
    /// terminal tab was pressed.
    OpenProjectFolder,
    /// The button of the title row that opens the statistics of the
    /// selected project was pressed.
    ShowProjectStats,
    /// The statistics of one session were read.
    SessionStatsLoaded(LoadedSessionStats),
    /// The statistics of one project were read.
    ProjectStatsLoaded(LoadedProjectStats),
    /// Close of the statistics dialog, its cross or Esc was pressed.
    CloseStats,
    /// The plus of New session was pressed: start a session in the
    /// selected project with the only profile that can, or open the list
    /// of the profiles at the pointer when several can.
    NewSession,
    /// A profile was chosen for the new session.
    NewSessionWith(Profile),
    /// The "+" of the Projects section, New agent session in this folder
    /// of the command palette or New agent session here of Files was
    /// pressed: open the New session dialog in this folder; with none, in
    /// the folder of the focused terminal tab on this computer
    /// ([`AgentsCtx::local_folder`](crate::AgentsCtx)), else the home
    /// folder.
    OpenNewSessionDialog(Option<PathBuf>),
    /// The New session dialog was used.
    NewSessionDialog(NewSessionDialogIntent),
    /// The folder typed in a folder field with suggestions was looked up:
    /// the New session dialog or the profile dialog, as its query names.
    FolderLookedUp(LoadedFolder),
    /// A profile of the left panel or of the All profiles table was
    /// pressed: show its page.
    SelectProfile(Profile),
    /// All accounts of the left panel was pressed: show every account and
    /// what the accounts need from the user.
    SelectAllAccounts,
    /// All profiles of the left panel was pressed: show every profile and
    /// what the profiles need from the user.
    SelectAllProfiles,
    /// An account of the left panel or of the All accounts table was
    /// pressed: show its page.
    SelectAccount(Account),
    /// Log in of a profile was pressed, in its menu or in its need: the
    /// agent logs in in a terminal tab.
    LogIn(Profile),
    /// The "⋯" of a profile's page was pressed, or its row in the left
    /// panel or in the All profiles table right-pressed: open its menu at
    /// the pointer (spec 2026-11-19, Decision 7). What is shown stays.
    OpenProfileMenu(Profile),
    /// An item of the menu of a profile was pressed: it acts on the
    /// profile of the open menu.
    ProfileMenu(ProfileAction),
    /// The "⋯" of an account's page was pressed, or its row in the left
    /// panel or in the All accounts table right-pressed: open its menu at
    /// the pointer. What is shown stays.
    OpenAccountMenu(Account),
    /// An item of the menu of an account was pressed: it acts on the
    /// account of the open menu.
    AccountMenu(AccountAction),
    /// The "⋯" of the All accounts page was pressed: open its menu, Show
    /// hidden accounts and Show Found by OTTY, at the pointer.
    OpenAllAccountsMenu,
    /// The "⋯" of the All profiles page was pressed: open its menu, Show
    /// hidden profiles, at the pointer.
    OpenAllProfilesMenu,
    /// Show hidden accounts (`true`) or Hide hidden accounts (`false`):
    /// the hidden accounts join the lists, marked hidden, or leave them.
    ShowHiddenAccounts(bool),
    /// Show hidden profiles (`true`) or Hide hidden profiles (`false`).
    ShowHiddenProfiles(bool),
    /// Show Found by OTTY of the All accounts menu: open the report again.
    ShowFoundReport,
    /// The Rename dialog of an account or a profile was used.
    Rename(RenameIntent),
    /// The browser was asked to open a page; the error is why it could
    /// not be started.
    PageOpened(Result<(), String>),
    /// Remove of an added profile was pressed: ask first.
    RemoveProfile(Profile),
    /// The removal was confirmed.
    RemoveConfirmed,
    /// The removal was cancelled.
    RemoveCancelled,
    /// The `+` of the Profiles section was pressed: open the profile
    /// dialog to add one.
    OpenAddProfile,
    /// The dialog of a key a plugin part asks for was used.
    KeyDialog(KeyDialogIntent),
    /// Edit of a profile's menu was pressed: open the profile dialog with
    /// it.
    EditProfile(Profile),
    /// The profile dialog was used.
    Dialog(ProfileDialogIntent),
    /// `agents.json` was written; the error is ready to show.
    ProfilesSaved(Result<(), String>),
    /// Done, the cross or Esc of the Found by OTTY report: close it and
    /// remember in `agents.json` that it was shown.
    CloseFoundReport,
    /// The button of a need in the Found by OTTY report was pressed: close
    /// the report and do what the need asks.
    FoundReportNeed(NeedAction),
    /// The button of a need on the All accounts or All profiles page was
    /// pressed: do what the need asks.
    FixNeed(NeedAction),
    /// The tab of [`AgentsOutcome::OpenFixTab`] opened as the tab with this
    /// id: once it closes, the profiles are read again.
    FixTabOpened(u64),
    /// The tab with this id closed.
    TabClosed(u64),
    /// The title of the Accounts section of the left panel was pressed:
    /// open or close the section.
    ToggleAccountsSection,
    /// The title of the Profiles section of the left panel was pressed:
    /// open or close the section.
    ToggleAgentsSection,
    /// The title of the Projects section of the left panel was pressed:
    /// open or close the section.
    ToggleProjectsSection,
    /// The period drop-down of the usage block was pressed: open its menu
    /// at the pointer, or in a statistics popup its list under it. The
    /// usage intents act on the block of the popup while one is open.
    OpenPeriodMenu,
    /// The measure drop-down of the usage block was pressed: open its menu
    /// at the pointer, or in a statistics popup its list under it.
    OpenMeasureMenu,
    /// A period of its menu was picked: the usage block looks back over it.
    PickPeriod(UsagePeriod),
    /// A measure of its menu was picked: the usage block counts in it.
    PickMeasure(UsageMeasure),
    /// A tab of the usage block was pressed: its table lists the projects,
    /// the profiles, the models or the accounts.
    ShowUsageTab(UsageTab),
    /// A row of the table of the usage block was pressed, by its name:
    /// light its part of every bar, or stop when it was lit.
    PickUsageRow(String),
    /// The pointer moved over a row of the table of the usage block, by
    /// its name, or off the rows: light that row's part of every bar.
    HoverUsageRow(Option<String>),
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
    /// [`AgentsIntent::ProfilesLoaded`], or
    /// [`AgentsIntent::ProfilesReadStopped`] when the read stops before it
    /// answers.
    LoadProfiles,
    /// Read the sessions of this profile; answer
    /// [`AgentsIntent::SessionsLoaded`].
    LoadSessions(Profile),
    /// Ask the program of this agent for its version (`--version`), for
    /// its profiles whose session files name none. Only while Agents mode
    /// is on screen, once per agent in a run. Answer
    /// [`AgentsIntent::VersionLoaded`].
    LoadVersion(AgentId),
    /// Let the plugin parts read the entries of the system keyring they
    /// declare, which the user allowed, until OTTY quits; the system may
    /// ask the user to unlock it. Asked once Agents mode is on screen. No
    /// answer: the next profiles read finds the keys there.
    AllowPluginKeyring,
    /// Read the limits of this profile while Agents mode is on screen:
    /// the provider part of the account whose limits it shows reads them,
    /// and may start the agent's program, as is or in a hidden terminal.
    /// The read stands for every profile of the same account, which is
    /// read once (spec Decision 5); [`Agents::limits_to_read`] tells the
    /// account. Answer [`AgentsIntent::LimitsLoaded`].
    LoadLimits(Profile),
    /// Read the limits of this profile while Agents mode is away: no
    /// part starts the agent's program. Answer
    /// [`AgentsIntent::LimitsLoaded`].
    LoadLimitsInBackground(Profile),
    /// Count the tokens this profile spent; answer
    /// [`AgentsIntent::SpendLoaded`].
    LoadSpend(Profile),
    /// Read the bill of `organization`, which `profile` pays through, once
    /// for every profile of it, with the keys the user gave for it; answer
    /// [`AgentsIntent::BillLoaded`].
    LoadBill {
        /// The profile the organization was found in.
        profile: Profile,
        /// The organization, whose provider part reads the bill.
        organization: Account,
        /// The keys the user gave for it, each under its slot.
        given: Vec<GivenKey>,
    },
    /// Read the numbers of `account` through the provider part that
    /// found it, in the folders it was found in: the money of a key or an
    /// organization, or the limits of a subscription no profile reads.
    /// Answer [`AgentsIntent::AccountNumbersLoaded`].
    LoadAccountNumbers {
        /// The account to read.
        account: Account,
        /// The keys the user gave for it, each under its slot.
        given: Vec<GivenKey>,
        /// Whether its provider part may start the agent's program, while
        /// Agents mode is on screen.
        may_start_programs: bool,
    },
    /// Read the list prices of the money at API prices: the LiteLLM price
    /// table, fetched at most once a day, or its kept copy. With
    /// `fetch_again` the table is fetched even within the day, because the
    /// latest fetch failed. Answer [`AgentsIntent::PricesLoaded`].
    LoadPrices {
        /// Fetch the table even when the kept copy is less than a day
        /// old.
        fetch_again: bool,
    },
    /// Read the statistics of the session `session` of `profile` from the
    /// agent's files; answer [`AgentsIntent::SessionStatsLoaded`].
    LoadSessionStats {
        /// The profile the session belongs to.
        profile: Profile,
        /// The session.
        session: SessionId,
    },
    /// Read the statistics of the project `folder` from the files of each
    /// of `profiles`, over the whole history; answer
    /// [`AgentsIntent::ProjectStatsLoaded`].
    LoadProjectStats {
        /// The project folder.
        folder: PathBuf,
        /// The profiles with sessions in the folder whose agent reads
        /// statistics, in their order.
        profiles: Vec<Profile>,
    },
    /// Open a terminal tab named `title` that runs `command` in `folder`,
    /// then show Terminal mode. New session of a project screen and Start
    /// of the New session dialog report it alike.
    OpenTab {
        /// The title of the tab: the session title or the profile name.
        title: String,
        /// The project folder the agent works in.
        folder: PathBuf,
        /// The program, arguments and variables that start the agent or
        /// log the profile in.
        command: AgentCommand,
    },
    /// Open a terminal tab named `title` that runs `command` in `folder`,
    /// then show Terminal mode, as [`AgentsOutcome::OpenTab`] does, for
    /// what a profile needs: the command of a need, such as a login or the
    /// install of the agent's program (issue 32), and Log in of a profile.
    /// The app tells the id of the tab with [`AgentsIntent::FixTabOpened`],
    /// and the profiles are read again once it closes.
    OpenFixTab {
        /// The title of the tab: the profile or the account of the need.
        title: String,
        /// The folder the command runs in: the home folder.
        folder: PathBuf,
        /// The program, arguments and variables the need runs.
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
    /// terminal of Files does, then show Terminal mode: the title row of the
    /// project screen asks for it.
    OpenFolder(PathBuf),
    /// Put the line that runs `command` in a shell on the clipboard: its
    /// variables, then the program and its arguments, each quoted.
    CopyCommand(AgentCommand),
    /// Put this text on the clipboard.
    CopyText(String),
    /// Open this address in the user's browser, such as the usage page of
    /// an account at its provider; answer [`AgentsIntent::PageOpened`].
    OpenPage(String),
    /// Show Terminal mode with the Files panel open at this folder, such
    /// as the config folder of a profile. Nothing is started in it.
    ShowInFiles(PathBuf),
    /// Create the folder `create_dir` when there is one, then write `saved`
    /// to `agents.json`; answer [`AgentsIntent::ProfilesSaved`]. A config
    /// folder is never deleted.
    SaveProfiles {
        /// The whole new content of `agents.json`.
        saved: SavedProfiles,
        /// The config folder of a new profile to create first.
        create_dir: Option<PathBuf>,
    },
    /// Check whether the folder typed in the New session dialog or the
    /// profile dialog is a folder and list the folders it may stand for;
    /// answer [`AgentsIntent::FolderLookedUp`].
    LookUpFolder(FolderQuery),
    /// Move the keyboard focus to the folder field of the New session
    /// dialog, whose id is
    /// [`NEW_SESSION_FOLDER`](crate::NEW_SESSION_FOLDER).
    FocusNewSessionFolder,
    /// Tell the user about an error; the text is ready to show.
    Failed(String),
    /// Move the keyboard focus to the search field of the projects, whose
    /// id is [`PROJECT_SEARCH`](crate::PROJECT_SEARCH).
    FocusProjectSearch,
    /// Move the keyboard focus to the name field of the Rename dialog,
    /// whose id is [`RENAME_FIELD`](crate::RENAME_FIELD).
    FocusRenameField,
    /// A menu opened. It takes the keyboard, so typed keys do not reach the
    /// search field under it and Esc reaches the frame, which closes it.
    MenuOpened,
    /// Open Plugins mode at this place: "Read by", Add plugin… or
    /// Review of a need.
    OpenPlugins(PluginsPlace),
    /// Turn on this part of a plugin: Turn on of the need "<Part> of the
    /// <Plugin> plugin is off".
    TurnOnPluginPart(OffPart),
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
    /// Open the statistics of the session in a dialog.
    ShowStatistics,
}

/// An item of the menu of an account (spec 2026-11-19, Decision 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAction {
    /// Give the account a name of the user's own.
    Rename,
    /// Open the usage page of the account at its provider in the browser.
    OpenUsagePage,
    /// Stop reading the account and take it out of the lists.
    Hide,
    /// Read the hidden account again and bring it back to the lists.
    ShowAgain,
}

/// An item of the menu of a profile (spec 2026-11-19, Decision 7). Edit,
/// Log in and Remove send their own intents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAction {
    /// Give the profile another name.
    Rename,
    /// Show the config folder of the profile in the Files panel.
    OpenFolder,
    /// Stop reading the profile and take it out of the lists.
    Hide,
    /// Read the hidden profile again and bring it back to the lists.
    ShowAgain,
}
