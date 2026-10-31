/// Pane slots of the root split view: `left | main | right`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarPane {
    Left,
    Main,
    Right,
}
