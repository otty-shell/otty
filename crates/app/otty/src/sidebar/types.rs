/// Pane slots of the root split view: `left | main | right`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarPane {
    Left,
    Main,
    Right,
}

/// The two left panel headers that have a split button with a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the section headers of tabs (issue 10) and quick launch (issue 12) open these menus"
    )
)]
pub(crate) enum HeaderMenu {
    /// The New session menu under the Sessions header.
    Sessions,
    /// The New launch / New folder menu under the Launches header.
    Launches,
}

/// A header menu that is open, with the point where it was opened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OpenHeaderMenu {
    menu: HeaderMenu,
    cursor: iced::Point,
}

impl OpenHeaderMenu {
    /// Return which header menu is open.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the header menu layer (issues 10 and 12) draws the open menu"
        )
    )]
    pub(crate) fn menu(&self) -> HeaderMenu {
        self.menu
    }

    /// Return the point where the menu was opened.
    #[expect(
        dead_code,
        reason = "the header menu layer (issues 10 and 12) anchors the menu here"
    )]
    pub(crate) fn cursor(&self) -> iced::Point {
        self.cursor
    }

    /// Open `menu` at `cursor`.
    pub(crate) fn new(menu: HeaderMenu, cursor: iced::Point) -> Self {
        Self { menu, cursor }
    }
}
