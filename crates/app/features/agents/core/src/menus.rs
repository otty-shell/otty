//! The menus of Agents mode and where they open: at the pointer, kept
//! inside the main area.

use iced::{Point, Size};
use otty_agents::Profile;

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
    /// The "⋯" of a profile's dashboard: Edit, Log in while the profile is
    /// not logged in, and Remove for an added profile.
    Profile(Profile),
}

/// Where a menu opens: the pointer, measured from the top-left corner of
/// the main area, and the size of the main area the menu stays inside.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct MenuPlace {
    cursor: Point,
    area: Size,
}

impl MenuPlace {
    /// The pointer, measured from the top-left corner of the main area.
    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }

    /// The size of the main area.
    pub(crate) fn area(&self) -> Size {
        self.area
    }

    /// A place at `cursor` in a main area of `area`.
    pub(crate) fn new(cursor: Point, area: Size) -> Self {
        Self { cursor, area }
    }
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
