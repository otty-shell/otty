//! Agents mode of the OTTY app: the profiles of the coding agents and the
//! projects their sessions ran in, in the left panel; the sessions table of
//! the selected project with Continue, its row menu and New session, in the
//! main area; and the counts in the left side of the status bar (spec
//! Decisions 2, 3 and 6). Each connected profile shows its limits, and each
//! profile the tokens it spent.
//!
//! What the mode shows is read when OTTY starts, with the mode away: the
//! profiles, then the sessions, the limits, the tokens, the list prices of
//! the money estimate (while a profile pays with an API key) and the money,
//! one kind after another, and never with the agent's program running
//! (`codex app-server`, or `claude` in a hidden terminal). Until the first
//! read of the profiles ends the main area shows a spinner; then the first
//! profile is selected, unless the user selected something. On screen,
//! Refresh reads everything again and the limits are read every five
//! minutes; opening the mode reads again only what is older than that
//! (spec Decision 10). A real failure, such as an `agents.json` that can
//! not be read, is reported once as [`AgentsOutcome::Failed`] until it goes
//! away.
//!
//! The feature reads no file and starts no program itself: it has the data
//! types of `otty-agents` and the list of known agents, whose commands it
//! builds. It reports the reads it needs as an [`AgentsOutcome`];
//! `otty-app-agents-io` runs them and gives the answer back as an
//! [`AgentsIntent`]. Continue and New session report
//! [`AgentsOutcome::OpenTab`], and the binary opens the terminal tab;
//! Continue in a split reports [`AgentsOutcome::OpenSplit`] while Terminal
//! mode has a pane on this computer to split. Show statistics of a session
//! and the statistics button of a project open a dialog and report
//! [`AgentsOutcome::LoadSessionStats`] or
//! [`AgentsOutcome::LoadProjectStats`]; the dialog shows a spinner until
//! the answer comes. The New session dialog starts a session in any folder
//! (spec Decision 15): it opens on [`AgentsIntent::OpenNewSessionDialog`],
//! asks for the typed folder with [`AgentsOutcome::LookUpFolder`] and
//! reports [`AgentsOutcome::OpenTab`] on Start. The config folder field of
//! the profile dialog asks for its folders the same way (spec Decision
//! 17).
//!
//! # Example
//!
//! Entering Agents mode reads the profiles; each profile has its sessions
//! read and its tokens counted.
//!
//! ```
//! use std::path::PathBuf;
//! use std::sync::Arc;
//!
//! use otty_agents::{
//!     AgentEnv, AgentId, Profile, ProfileStatus, SavedProfiles,
//! };
//! use otty_app_agents::{
//!     Agents, AgentsCtx, AgentsIntent, AgentsOutcome, LoadedProfile,
//!     LoadedProfiles,
//! };
//! use otty_app_slots::Component;
//!
//! let mut agents = Agents::new(
//!     Arc::from(otty_agents::known_agents()),
//!     AgentEnv::new(PathBuf::from("/home/user")),
//! );
//!
//! let entered = agents.reduce(AgentsIntent::Enter, AgentsCtx::default());
//! assert_eq!(entered, vec![AgentsOutcome::LoadProfiles]);
//!
//! // The binary read the profiles and gives the answer back.
//! let codex = Profile::default_for(
//!     AgentId::new("codex"),
//!     "Codex",
//!     PathBuf::from("/home/user/.codex"),
//! );
//! let loaded = agents.reduce(
//!     AgentsIntent::ProfilesLoaded(LoadedProfiles {
//!         profiles: vec![LoadedProfile {
//!             profile: codex.clone(),
//!             status: ProfileStatus::NotLoggedIn,
//!         }],
//!         saved: Some(SavedProfiles::default()),
//!         problem: None,
//!         time_zone: None,
//!         read_at: std::time::SystemTime::UNIX_EPOCH,
//!     }),
//!     AgentsCtx::default(),
//! );
//! assert_eq!(
//!     loaded,
//!     vec![
//!         AgentsOutcome::LoadSessions(codex.clone()),
//!         AgentsOutcome::LoadSpend(codex),
//!     ]
//! );
//! ```

mod bill;
mod ctx;
mod event;
mod failure_text;
mod folder_field;
mod freshness;
mod limits;
mod menus;
mod model;
mod new_session_dialog;
mod prices;
mod profile_actions;
mod profile_changes;
mod profile_dialog;
mod profile_state;
mod profiles;
mod project_filter;
mod project_search;
mod reads;
mod real_failures;
mod reducer;
mod session_actions;
mod session_sort;
mod sessions_table;
mod spend;
mod state;
mod stats_dialog;
#[cfg(test)]
mod testing;
mod top_tab;
mod types;
mod view;

use std::time::SystemTime;

pub use ctx::AgentsCtx;
pub use event::{AgentsIntent, AgentsOutcome, SessionAction};
pub use folder_field::FolderFieldIntent;
use iced::widget::Id;
use iced::{Element, Subscription};
use model::{
    LeftPanelViewModel, MainAreaViewModel, MenuViewModel,
    RemoveDialogViewModel, StatsDialogViewModel, StatusBarViewModel,
};
pub use new_session_dialog::NewSessionDialogIntent;
use new_session_dialog::NewSessionDialogViewModel;
use otty_agents::AgentEnv;
use otty_app_slots::{
    Component, MainArea, MainContent, Overlay, Overlays, StatusBar,
};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::MenuPlace;
use otty_ui_kit::components::status_bar::StatusItem;
use profile_dialog::ProfileDialogViewModel;
pub use profile_dialog::{AdminKeyChoice, FolderChoice, ProfileDialogIntent};
pub use project_filter::ProjectFilterChange;
use state::AgentsState;
pub use top_tab::TopTab;
pub use types::{
    AgentList, FolderAsker, FolderQuery, LimitsRead, LoadedBill, LoadedFolder,
    LoadedLimits, LoadedPrices, LoadedProfile, LoadedProfiles,
    LoadedProjectStats, LoadedSessionStats, LoadedSessions, LoadedSpend,
    ProjectStatsFailure, SessionsRead, SpendCount,
};
use view::{
    left_panel, main_area, menus as menu_view, remove_dialog,
    stats_dialog as stats_view, status_bar,
};

/// The id of the search field of the projects in the left panel; the
/// binary moves the keyboard focus to it on
/// [`AgentsOutcome::FocusProjectSearch`].
pub const PROJECT_SEARCH: Id = Id::new("agents_project_search");

/// The id of the folder field of the New session dialog; the binary moves
/// the keyboard focus to it on [`AgentsOutcome::FocusNewSessionFolder`].
pub const NEW_SESSION_FOLDER: Id = Id::new("agents_new_session_folder");

/// The id of the spinner shown while the profiles are read for the first
/// time; the tests of the binary look for it. Only with the `test-ids`
/// feature, which the test build of the binary turns on.
#[cfg(feature = "test-ids")]
pub const LOADING_SPINNER_ID: &str = view::main_area::LOADING_SPINNER_ID;

/// The id of the box around the button that opens the statistics of the
/// project; the tests of the binary press it by it. Only with the
/// `test-ids` feature.
#[cfg(feature = "test-ids")]
pub const PROJECT_STATS_BUTTON_ID: &str =
    view::project::PROJECT_STATS_BUTTON_ID;

/// The id of the config folder field of the profile dialog; the tests of
/// the binary type in it by it. Only with the `test-ids` feature.
#[cfg(feature = "test-ids")]
pub const PROFILE_CONFIG_FOLDER: Id = profile_dialog::CONFIG_FOLDER_ID;

/// The id of the box around the plus of the Projects section that opens
/// the New session dialog; the tests of the binary press it by it. Only
/// with the `test-ids` feature.
#[cfg(feature = "test-ids")]
pub const NEW_SESSION_BUTTON_ID: &str = view::NEW_SESSION_BUTTON_ID;

/// The agents feature: the profiles of the known agents, their sessions
/// grouped into projects, and what is selected (spec Decision 6).
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::AgentEnv;
/// use otty_app_agents::Agents;
///
/// let agents = Agents::new(
///     Arc::from(otty_agents::known_agents()),
///     AgentEnv::new(PathBuf::from("/home/user")),
/// );
///
/// assert!(!agents.is_active());
/// assert_eq!(agents.known_agents().len(), 2);
/// ```
pub struct Agents {
    state: AgentsState,
}

impl Agents {
    /// Return whether Agents mode is the active mode.
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Return the agents OTTY knows. The binary gives them to the reads of
    /// `otty-app-agents-io`, so every read uses the same agents.
    pub fn known_agents(&self) -> &AgentList {
        self.state.agents()
    }

    /// Return what the agents look at in the environment: the home folder
    /// and the config folder variable of each agent. The binary gives it
    /// to the read of the profiles and opens a login tab in its home
    /// folder, so the feature is its one owner.
    pub fn env(&self) -> &AgentEnv {
        self.state.env()
    }

    /// Return whether the profile dialog is open.
    pub fn has_profile_dialog(&self) -> bool {
        self.state.dialog().is_some()
    }

    /// Return whether the dialog that asks before a removal is open.
    pub fn has_remove_dialog(&self) -> bool {
        self.state.removing().is_some()
    }

    /// Return whether the statistics dialog of a session or a project is
    /// open.
    pub fn has_stats_dialog(&self) -> bool {
        self.state.stats().is_some()
    }

    /// Return whether the New session dialog is open.
    pub fn has_new_session_dialog(&self) -> bool {
        self.state.new_session().is_some()
    }

    /// Return whether a menu of Agents mode is open: the funnel menu of the
    /// projects, the profiles of New session or the menu of a session.
    pub fn has_menu(&self) -> bool {
        self.state.menu().is_some()
    }

    /// Create the feature with the agents OTTY knows and the environment
    /// they look at, before anything was read. The home folder of `env`
    /// gives the folder proposed to a new profile.
    pub fn new(agents: AgentList, env: AgentEnv) -> Self {
        Self {
            state: AgentsState::new(agents, env),
        }
    }

    /// The left panel of Agents mode: the Profiles section over the
    /// Projects section, sharing the panel height (spec Decisions 6 and 10).
    ///
    /// It is a method of its own and not [`otty_app_slots::LeftPanel`]: that
    /// trait draws one of the stacked sections of Terminal mode, whose open
    /// state and header menus the frame keeps. Agents mode keeps both
    /// sections, their open state and their menus itself, and fills the
    /// whole left panel, like the engine tree of Containers mode.
    pub fn left_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, AgentsIntent> {
        left_panel::view(left_panel::LeftPanelProps {
            vm: LeftPanelViewModel::from_state(&self.state, SystemTime::now()),
            theme,
        })
    }
}

impl Component for Agents {
    type Intent = AgentsIntent;
    type Outcome = AgentsOutcome;
    type Ctx<'a> = AgentsCtx;

    /// Reduce an intent into state changes and outcomes. The pointer of
    /// `ctx` is kept as the place the next menu opens at, whether Terminal
    /// mode has a pane to continue a session in a split, and the folder of
    /// that pane, where the New session dialog opens.
    fn reduce(
        &mut self,
        intent: AgentsIntent,
        ctx: AgentsCtx,
    ) -> Vec<AgentsOutcome> {
        self.state
            .set_pointer(MenuPlace::new(ctx.cursor, ctx.screen_size));
        self.state.set_has_local_pane(ctx.has_local_pane);
        self.state.set_local_folder(ctx.local_folder);

        reducer::reduce(&mut self.state, intent)
    }

    /// The timer of the limits while Agents mode is active, none while it
    /// is away; the reads themselves are I/O, so `otty-app-agents-io` runs
    /// them.
    fn subscription(&self) -> Subscription<AgentsIntent> {
        if !self.state.is_active() {
            return Subscription::none();
        }

        iced::time::every(freshness::READ_PERIOD)
            .map(|_| AgentsIntent::LimitsTick)
    }
}

/// Which screen fills the main area of Agents mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentsScreen {
    /// The card while the profiles are read for the first time.
    Loading,
    /// The card that says no agent was found.
    NoAgent,
    /// The card that says what the mode is for.
    Start,
    /// The dashboard of the profile with this name.
    Profile(String),
}

impl MainArea for Agents {
    type Screen = AgentsScreen;

    /// The sessions table of the selected project, which scrolls on its
    /// own, or a screen: the dashboard of a profile or a card; the ages
    /// count up to the current time.
    fn main_area<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> MainContent<'a, AgentsIntent, AgentsScreen> {
        let vm = MainAreaViewModel::from_state(&self.state, SystemTime::now());
        let id = match &vm {
            MainAreaViewModel::Loading => Some(AgentsScreen::Loading),
            MainAreaViewModel::NoAgent => Some(AgentsScreen::NoAgent),
            MainAreaViewModel::Start => Some(AgentsScreen::Start),
            MainAreaViewModel::Project(_) => None,
            MainAreaViewModel::Profile(dashboard) => {
                Some(AgentsScreen::Profile(dashboard.name.clone()))
            },
        };

        let content = main_area::view(vm, theme.theme.ui());
        match id {
            Some(id) => MainContent::Screen { id, content },
            None => MainContent::ScrollsOnItsOwn(content),
        }
    }
}

impl Overlays for Agents {
    /// The overlay on top: the profile dialog, then the dialog that asks
    /// before a removal, then the statistics dialog, then the New session
    /// dialog, all over the whole window; then the open menu, at the
    /// pointer in the main area.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, AgentsIntent>> {
        if let Some(vm) = ProfileDialogViewModel::from_state(&self.state) {
            let dialog =
                profile_dialog::view(profile_dialog::ProfileDialogProps {
                    vm,
                    theme,
                });

            return Some(Overlay::dialog(dialog));
        }

        if let Some(vm) = RemoveDialogViewModel::from_state(&self.state) {
            let dialog =
                remove_dialog::view(remove_dialog::RemoveDialogProps {
                    vm,
                    theme,
                });

            return Some(Overlay::dialog(dialog));
        }

        if let Some(vm) = StatsDialogViewModel::from_state(&self.state) {
            let dialog =
                stats_view::view(stats_view::StatsDialogProps { vm, theme });

            return Some(Overlay::dialog(dialog));
        }

        if let Some(vm) = NewSessionDialogViewModel::from_state(&self.state) {
            let dialog = new_session_dialog::view(
                new_session_dialog::NewSessionDialogProps { vm, theme },
            );

            return Some(Overlay::dialog(dialog));
        }

        let vm = MenuViewModel::from_state(&self.state)?;
        let menu = menu_view::view(menu_view::MenuProps { vm, theme });

        Some(Overlay::menu(menu))
    }
}

impl StatusBar for Agents {
    /// The counts of the projects and their sessions, or that the profiles
    /// are read.
    fn status_bar(&self) -> Vec<StatusItem<AgentsIntent>> {
        status_bar::items(&StatusBarViewModel::from_state(&self.state))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::AppTheme;

    use super::*;
    use crate::testing::{
        agents, claude, connected, env, profiles_loaded, session,
        sessions_loaded,
    };

    fn agents_with_a_project() -> Agents {
        let mut feature = Agents::new(agents(), env());
        for intent in [
            AgentsIntent::Enter,
            profiles_loaded(&[(claude(), connected())]),
            sessions_loaded(&claude(), vec![session("c1", "/work/otty", 1)]),
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        ] {
            let _ = feature.reduce(intent, AgentsCtx::default());
        }

        feature
    }

    #[test]
    fn the_feature_is_active_only_in_agents_mode() {
        let mut feature = Agents::new(agents(), env());

        let _ = feature.reduce(AgentsIntent::Enter, AgentsCtx::default());
        assert!(feature.is_active());
        let _ = feature.reduce(AgentsIntent::Leave, AgentsCtx::default());
        assert!(!feature.is_active());
    }

    #[test]
    fn the_limits_timer_runs_only_in_agents_mode() {
        let mut feature = Agents::new(agents(), env());
        let before = feature.subscription().units();

        let _ = feature.reduce(AgentsIntent::Enter, AgentsCtx::default());
        let active = feature.subscription().units();
        let _ = feature.reduce(AgentsIntent::Leave, AgentsCtx::default());

        assert_eq!(before, 0);
        assert_eq!(active, 1);
        assert_eq!(feature.subscription().units(), 0);
    }

    /// The sessions table has a fixed header over rows that scroll, which
    /// an outer scroller would break.
    #[test]
    fn the_sessions_table_of_the_selected_project_scrolls_on_its_own() {
        let feature = agents_with_a_project();
        let theme = AppTheme::default();

        let content = feature.main_area(ThemeProps::new(&theme));

        let MainContent::ScrollsOnItsOwn(content) = content else {
            panic!("the sessions table scrolls on its own");
        };
        let mut ui = simulator(content);
        ui.find("Title of c1").expect("the session should show");
    }

    #[test]
    fn continue_in_a_split_splits_the_pane_the_ctx_tells_of() {
        let mut feature = agents_with_a_project();
        let with_pane = AgentsCtx {
            has_local_pane: true,
            ..AgentsCtx::default()
        };
        let _ =
            feature.reduce(AgentsIntent::OpenSessionMenu(0), with_pane.clone());

        let outcomes = feature.reduce(
            AgentsIntent::SessionMenu(SessionAction::ContinueInSplit),
            with_pane,
        );

        assert!(matches!(
            outcomes.as_slice(),
            [AgentsOutcome::OpenSplit { folder, .. }]
                if folder == &PathBuf::from("/work/otty")
        ));
    }

    #[test]
    fn the_profile_dialog_and_the_remove_dialog_are_dialogs_over_the_window() {
        let mut feature = agents_with_a_project();
        let theme = AppTheme::default();
        let closed = feature.overlay(ThemeProps::new(&theme)).is_none();

        let _ =
            feature.reduce(AgentsIntent::OpenAddProfile, AgentsCtx::default());
        let add = feature
            .overlay(ThemeProps::new(&theme))
            .map(|open| open.kind());
        let has_dialog = feature.has_profile_dialog();
        let _ = feature.reduce(
            AgentsIntent::Dialog(ProfileDialogIntent::Cancelled),
            AgentsCtx::default(),
        );
        let _ = feature.reduce(
            AgentsIntent::RemoveProfile(claude()),
            AgentsCtx::default(),
        );

        assert!(closed);
        assert_eq!(add, Some(otty_app_slots::OverlayKind::Dialog));
        assert!(has_dialog);
        assert!(!feature.has_profile_dialog());
        assert!(!feature.has_remove_dialog(), "a default profile stays");
    }

    /// Owner, issue 36: with no folder given, the New session dialog opens
    /// on the folder of the focused terminal tab, which the ctx tells of.
    #[test]
    fn the_new_session_dialog_opens_on_the_folder_the_ctx_tells_of() {
        let mut feature = agents_with_a_project();
        let theme = AppTheme::default();
        let ctx = AgentsCtx {
            local_folder: Some(PathBuf::from("/work/otty")),
            ..AgentsCtx::default()
        };

        let outcomes =
            feature.reduce(AgentsIntent::OpenNewSessionDialog(None), ctx);

        assert!(feature.has_new_session_dialog());
        assert_eq!(
            feature.state.new_session().map(|dialog| dialog.text()),
            Some("/work/otty")
        );
        assert!(outcomes.contains(&AgentsOutcome::FocusNewSessionFolder));
        let overlay = feature
            .overlay(ThemeProps::new(&theme))
            .expect("the dialog is open");
        assert_eq!(overlay.kind(), otty_app_slots::OverlayKind::Dialog);
        let mut ui = simulator(overlay.into_content());
        ui.find(i18n::t(Key::AgentsFieldFolder))
            .expect("the folder field should show");
    }

    #[test]
    fn the_add_dialog_shows_its_fields() {
        let mut feature = agents_with_a_project();
        let theme = AppTheme::default();
        let _ =
            feature.reduce(AgentsIntent::OpenAddProfile, AgentsCtx::default());

        let overlay = feature
            .overlay(ThemeProps::new(&theme))
            .expect("the dialog is open");
        let mut ui = simulator(overlay.into_content());

        ui.find(i18n::t(Key::AgentsAddProfile))
            .expect("the title should show");
        ui.find(i18n::t(Key::AgentsFolderCreate))
            .expect("the folder choices should show");
        ui.find(i18n::t(Key::AgentsCreateFolderHint))
            .expect("the hint of a new folder should show");
    }

    #[test]
    fn a_selected_profile_fills_the_main_area() {
        let mut feature = agents_with_a_project();
        let _ = feature.reduce(
            AgentsIntent::SelectProfile(claude()),
            AgentsCtx::default(),
        );
        let theme = AppTheme::default();

        let content = feature.main_area(ThemeProps::new(&theme));

        assert!(matches!(
            content,
            MainContent::Screen { id: AgentsScreen::Profile(name), .. }
                if name == "Claude Code"
        ));
    }

    #[test]
    fn before_the_first_read_the_main_area_is_the_loading_card() {
        let feature = Agents::new(agents(), env());
        let theme = AppTheme::default();

        let content = feature.main_area(ThemeProps::new(&theme));

        assert!(matches!(
            content,
            MainContent::Screen {
                id: AgentsScreen::Loading,
                ..
            }
        ));
    }

    #[test]
    fn the_left_panel_and_the_status_bar_show_the_project() {
        let feature = agents_with_a_project();
        let theme = AppTheme::default();

        let mut ui = simulator(feature.left_panel(ThemeProps::new(&theme)));
        ui.find("otty").expect("the project should show");
        ui.find("Claude Code").expect("the profile should show");

        let items = feature.status_bar();
        assert_eq!(items.len(), 2);
        let mut bar =
            otty_ui_kit::components::status_bar::StatusBar::new(theme.ui());
        for item in items {
            bar = bar.left(item);
        }
        let mut ui = simulator(bar);
        ui.find(i18n::t(Key::AgentsStatusOneProject))
            .expect("the project count should show");
    }

    #[test]
    fn an_open_menu_is_a_menu_overlay_at_the_pointer() {
        let mut feature = agents_with_a_project();
        let theme = AppTheme::default();
        let ctx = AgentsCtx {
            cursor: iced::Point::new(100.0, 200.0),
            screen_size: iced::Size::new(1000.0, 700.0),
            has_local_pane: false,
            local_folder: None,
        };
        let _ = feature.reduce(AgentsIntent::ToggleProjectSearch, ctx.clone());

        let opened = feature.reduce(AgentsIntent::OpenFilterMenu, ctx);

        assert_eq!(opened, vec![AgentsOutcome::MenuOpened]);
        assert!(feature.has_menu());
        let overlay = feature
            .overlay(ThemeProps::new(&theme))
            .expect("the menu is open");
        assert_eq!(overlay.kind(), otty_app_slots::OverlayKind::Menu);
        let mut ui = simulator(overlay.into_content());
        ui.find(i18n::t(Key::AgentsFilterAllAgents))
            .expect("the menu should list the agents");
    }
}
