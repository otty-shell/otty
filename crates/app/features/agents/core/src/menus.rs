//! The menus of Agents mode and where they open: at the pointer, kept
//! inside the main area.

use otty_agents::Profile;
use otty_ui_kit::components::context_menu::MenuPlace;

use super::sessions_table::SessionKey;

/// A menu of Agents mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentsMenu {
    /// The funnel menu of the projects: the agent, what to show, the sort.
    ProjectFilter,
    /// The profiles a new session can start with, from New session.
    NewSession,
    /// The actions of one session of the sessions table.
    Session(SessionKey),
    /// The "⋯" of a profile's dashboard, also opened by a right press on
    /// its row in the left panel: Edit, Log in while the profile is not
    /// logged in, and Remove for an added profile.
    Profile(Profile),
}

/// A menu that is open, with the place it opened at.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OpenMenu {
    menu: AgentsMenu,
    place: MenuPlace,
}

impl OpenMenu {
    /// Which menu is open.
    pub(crate) fn menu(&self) -> &AgentsMenu {
        &self.menu
    }

    /// Where it opened.
    pub(crate) fn place(&self) -> MenuPlace {
        self.place
    }

    /// `menu`, open at `place`.
    pub(crate) fn new(menu: AgentsMenu, place: MenuPlace) -> Self {
        Self { menu, place }
    }
}
