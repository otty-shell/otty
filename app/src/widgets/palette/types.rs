use crate::commands::AppCommand;

/// What running a palette entry does. Every entry carries one as its key;
/// the app router turns it into the event the owning widget already
/// handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PaletteTarget {
    /// Switch to this open tab.
    Session { tab_id: u64 },
    /// Run the launch at this path of the launch tree: the folder titles,
    /// then the launch title.
    Launch { path: Vec<String> },
    /// Run this app command, as its shortcut or header button would.
    Command(AppCommand),
}
