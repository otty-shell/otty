use crate::widgets::quick_launch::types::NodePath;

/// What running a palette entry does. Every entry carries one as its key;
/// the app router turns it into the event the owning widget already
/// handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PaletteTarget {
    /// Switch to this open tab.
    Session { tab_id: u64 },
    /// Run the launch at this path of the launch tree.
    Launch { path: NodePath },
    /// Open a terminal tab with the default shell.
    NewTerminal,
    /// Open the editor for a new launch.
    NewLaunch,
    /// Start naming a new launch folder.
    NewFolder,
    /// Open the Settings tab.
    Settings,
    /// Split the focused pane of the active terminal tab.
    SplitPane,
    /// Close the active tab.
    CloseSession,
    /// Show or hide the left panel.
    ToggleLeftPanel,
    /// Show or hide the files (right) panel.
    ToggleFilesPanel,
    /// Enter or leave focus mode.
    ToggleFocusMode,
}
