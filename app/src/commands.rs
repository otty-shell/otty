//! App commands: what the user asks the app to do, whichever way they asked.
//!
//! A shortcut, a command palette entry and a header button can ask for the
//! same thing. Each picks an [`AppCommand`], and `router/command.rs` turns
//! it into the intents of the widgets that do the work, so every way runs
//! the same code. [`AppCommand::is_available`] says when a command can run;
//! the palette lists only those, and the router runs only those.

use crate::domain::mode::Mode;

/// An action the user can ask for by a shortcut, a palette entry or a
/// header button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppCommand {
    /// Open the command palette.
    OpenPalette,
    /// Show or hide the left panel.
    ToggleLeftPanel,
    /// Show or hide the files (right) panel.
    ToggleFilesPanel,
    /// Enter or leave focus mode.
    ToggleFocusMode,
    /// Open the Settings tab.
    OpenSettings,
    /// Open a terminal tab with the default shell.
    NewTerminal,
    /// Close the active tab.
    CloseSession,
    /// Split the focused pane of the active tab to the right.
    SplitPane,
    /// Switch to the mode at this switcher position: Terminal at 0, then
    /// the pinned modes.
    SelectModeAt { position: usize },
    /// Switch to this mode, pinned or not.
    SwitchMode(Mode),
    /// Open or close the All modes menu.
    ToggleModesMenu,
    /// Open the editor for a new launch.
    NewLaunch,
    /// Start naming a new launch folder.
    NewFolder,
}

impl AppCommand {
    /// Return whether the command can run now. Close session and Split pane
    /// act on the active tab, and New folder on the Launches panel, which
    /// only show in Terminal mode (spec Decision 5). Close session also
    /// needs an active tab, and Split pane a focused terminal pane.
    pub(crate) fn is_available(self, ctx: &CommandCtx) -> bool {
        match self {
            Self::CloseSession => ctx.is_terminal_mode && ctx.has_active_tab,
            Self::SplitPane => ctx.is_terminal_mode && ctx.has_focused_pane,
            Self::NewFolder => ctx.is_terminal_mode,
            _ => true,
        }
    }
}

/// What decides whether a command can run, gathered from the widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandCtx {
    /// Terminal mode is on screen.
    pub(crate) is_terminal_mode: bool,
    /// A tab is active.
    pub(crate) has_active_tab: bool,
    /// The active tab is a terminal tab with a focused pane.
    pub(crate) has_focused_pane: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Context where nothing a command may need is there.
    const NOTHING: CommandCtx = CommandCtx {
        is_terminal_mode: false,
        has_active_tab: false,
        has_focused_pane: false,
    };

    /// Context in Terminal mode with an active terminal tab.
    const TERMINAL_TAB: CommandCtx = CommandCtx {
        is_terminal_mode: true,
        has_active_tab: true,
        has_focused_pane: true,
    };

    #[test]
    fn close_session_needs_terminal_mode_and_an_active_tab() {
        let command = AppCommand::CloseSession;

        assert!(command.is_available(&TERMINAL_TAB));
        assert!(command.is_available(&CommandCtx {
            has_focused_pane: false,
            ..TERMINAL_TAB
        }));
        assert!(!command.is_available(&CommandCtx {
            is_terminal_mode: false,
            ..TERMINAL_TAB
        }));
        assert!(!command.is_available(&CommandCtx {
            has_active_tab: false,
            has_focused_pane: false,
            ..TERMINAL_TAB
        }));
    }

    #[test]
    fn split_pane_needs_terminal_mode_and_a_focused_pane() {
        let command = AppCommand::SplitPane;

        assert!(command.is_available(&TERMINAL_TAB));
        assert!(!command.is_available(&CommandCtx {
            is_terminal_mode: false,
            ..TERMINAL_TAB
        }));
        assert!(!command.is_available(&CommandCtx {
            has_focused_pane: false,
            ..TERMINAL_TAB
        }));
    }

    #[test]
    fn new_folder_needs_terminal_mode() {
        let command = AppCommand::NewFolder;

        assert!(command.is_available(&CommandCtx {
            is_terminal_mode: true,
            ..NOTHING
        }));
        assert!(!command.is_available(&NOTHING));
    }

    #[test]
    fn every_other_command_is_always_available() {
        for command in [
            AppCommand::OpenPalette,
            AppCommand::ToggleLeftPanel,
            AppCommand::ToggleFilesPanel,
            AppCommand::ToggleFocusMode,
            AppCommand::OpenSettings,
            AppCommand::NewTerminal,
            AppCommand::SelectModeAt { position: 1 },
            AppCommand::SwitchMode(Mode::Containers),
            AppCommand::ToggleModesMenu,
            AppCommand::NewLaunch,
        ] {
            assert!(command.is_available(&NOTHING), "{command:?}");
        }
    }
}
