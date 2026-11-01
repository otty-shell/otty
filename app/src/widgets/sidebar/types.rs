/// Pane slots of the root split view: `left | main | right`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarPane {
    Left,
    Main,
    Right,
}

/// The two left panel headers that have a split button with a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeaderMenu {
    /// The New session menu under the Sessions header.
    Sessions,
    /// The New launch / New folder menu under the Launches header.
    Launches,
}

/// A header menu that is open, with the point where it was opened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OpenHeaderMenu {
    pub(crate) menu: HeaderMenu,
    pub(crate) cursor: iced::Point,
}
