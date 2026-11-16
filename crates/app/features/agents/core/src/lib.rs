//! Agents mode of the OTTY app: the profiles of the coding agents and the
//! projects their sessions ran in, in the left panel; the sessions of the
//! selected project with Continue and New session, in the main area; and
//! the counts in the left side of the status bar (spec Decisions 2, 3 and
//! 6).
//!
//! The feature reads no file and starts no program itself: it has the data
//! types of `otty-agents` and the list of known agents, whose commands it
//! builds. It reports the reads it needs as an [`AgentsOutcome`];
//! `otty-app-agents-io` runs them and gives the answer back as an
//! [`AgentsIntent`]. Continue and New session report
//! [`AgentsOutcome::OpenTab`], and the binary opens the terminal tab.
//!
//! # Example
//!
//! Entering Agents mode reads the profiles; each profile has its sessions
//! read.
//!
//! ```
//! use std::path::PathBuf;
//! use std::sync::Arc;
//!
//! use otty_agents::{AgentId, Profile, ProfileStatus};
//! use otty_app_agents::{
//!     Agents, AgentsIntent, AgentsOutcome, LoadedProfile, LoadedProfiles,
//! };
//! use otty_app_slots::Component;
//!
//! let mut agents = Agents::new(Arc::from(otty_agents::known_agents()));
//!
//! let entered = agents.reduce(AgentsIntent::Enter, ());
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
//!         problem: None,
//!     }),
//!     (),
//! );
//! assert_eq!(loaded, vec![AgentsOutcome::LoadSessions(codex)]);
//! ```

mod age;
mod event;
mod model;
mod reducer;
mod state;
#[cfg(test)]
mod testing;
mod types;
mod view;

use std::path::PathBuf;
use std::time::SystemTime;

pub use event::{AgentsIntent, AgentsOutcome};
use iced::Element;
use model::{LeftPanelViewModel, MainAreaViewModel, StatusBarViewModel};
use otty_app_slots::{Component, MainArea, MainContent, StatusBar};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::status_bar::StatusItem;
use state::AgentsState;
pub use types::{AgentList, LoadedProfile, LoadedProfiles, LoadedSessions};
use view::{left_panel, main_area, status_bar};

/// The agents feature: the profiles of the known agents, their sessions
/// grouped into projects, and what is selected (spec Decision 6).
///
/// # Example
///
/// ```
/// use std::sync::Arc;
///
/// use otty_app_agents::Agents;
///
/// let agents = Agents::new(Arc::from(otty_agents::known_agents()));
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

    /// Create the feature with the agents OTTY knows, before anything was
    /// read.
    pub fn new(agents: AgentList) -> Self {
        Self {
            state: AgentsState::new(agents),
        }
    }

    /// The left panel of Agents mode: the profiles and the projects.
    ///
    /// It is a method of its own and not [`otty_app_slots::LeftPanel`]: that
    /// trait draws one of the stacked sections of Terminal mode, with a
    /// header the frame opens and closes. This panel fills the whole left
    /// panel, like the engine tree of Containers mode.
    pub fn left_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, AgentsIntent> {
        left_panel::view(left_panel::LeftPanelProps {
            vm: LeftPanelViewModel::from_state(&self.state),
            theme,
        })
    }
}

impl Component for Agents {
    type Intent = AgentsIntent;
    type Outcome = AgentsOutcome;
    type Ctx<'a> = ();

    /// Reduce an intent into state changes and outcomes.
    fn reduce(&mut self, intent: AgentsIntent, _ctx: ()) -> Vec<AgentsOutcome> {
        reducer::reduce(&mut self.state, intent)
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
    /// The sessions of the project in this folder.
    Project(PathBuf),
}

impl MainArea for Agents {
    type Screen = AgentsScreen;

    /// The sessions of the selected project, or a card while there is
    /// none; the ages count up to the current time.
    fn main_area<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> MainContent<'a, AgentsIntent, AgentsScreen> {
        let vm = MainAreaViewModel::from_state(&self.state, SystemTime::now());
        let id = match &vm {
            MainAreaViewModel::Loading => AgentsScreen::Loading,
            MainAreaViewModel::NoAgent => AgentsScreen::NoAgent,
            MainAreaViewModel::Start => AgentsScreen::Start,
            MainAreaViewModel::Project(project) => {
                AgentsScreen::Project(project.folder.clone())
            },
        };

        MainContent::Screen {
            id,
            content: main_area::view(vm, theme.theme.ui()),
        }
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
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::AppTheme;

    use super::*;
    use crate::testing::{
        agents, claude, connected, profiles_loaded, session, sessions_loaded,
    };

    fn agents_with_a_project() -> Agents {
        let mut feature = Agents::new(agents());
        for intent in [
            AgentsIntent::Enter,
            profiles_loaded(&[(claude(), connected())]),
            sessions_loaded(&claude(), vec![session("c1", "/work/otty", 1)]),
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        ] {
            let _ = feature.reduce(intent, ());
        }

        feature
    }

    #[test]
    fn the_feature_is_active_only_in_agents_mode() {
        let mut feature = Agents::new(agents());

        let _ = feature.reduce(AgentsIntent::Enter, ());
        assert!(feature.is_active());
        let _ = feature.reduce(AgentsIntent::Leave, ());
        assert!(!feature.is_active());
    }

    #[test]
    fn the_main_area_is_a_screen_of_the_selected_project() {
        let feature = agents_with_a_project();
        let theme = AppTheme::default();

        let content = feature.main_area(ThemeProps::new(&theme));

        let MainContent::Screen { id, content } = content else {
            panic!("the main area of Agents mode is a screen");
        };
        assert_eq!(id, AgentsScreen::Project(PathBuf::from("/work/otty")));
        let mut ui = simulator(content);
        ui.find("Title of c1").expect("the session should show");
    }

    #[test]
    fn before_the_first_read_the_main_area_is_the_loading_card() {
        let feature = Agents::new(agents());
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
}
