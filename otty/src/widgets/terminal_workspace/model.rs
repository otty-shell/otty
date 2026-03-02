use std::collections::HashMap;

use iced::widget::pane_grid;
use otty_ui_term::settings::SessionKind;

use super::state::PaneContextMenuState;

// ---------------------------------------------------------------------------
// Domain models
// ---------------------------------------------------------------------------

/// Terminal context determining whether shell metadata is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalKind {
    /// Interactive shell session.
    Shell,
    /// One-shot command execution.
    Command,
}

/// Shell session information needed to start a terminal backend.
#[derive(Debug, Clone)]
pub(crate) struct ShellSession {
    name: String,
    session: SessionKind,
}

impl ShellSession {
    /// Create shell session metadata for terminal startup.
    pub(crate) fn new(name: String, session: SessionKind) -> Self {
        Self { name, session }
    }

    /// Return shell label shown in tab titles.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Return backend session descriptor used by terminal settings.
    pub(crate) fn session(&self) -> &SessionKind {
        &self.session
    }
}

/// A terminal entry stored per pane.
pub(crate) struct TerminalEntry {
    pub(super) terminal: otty_ui_term::Terminal,
    pub(super) title: String,
}

impl TerminalEntry {
    /// Return read-only reference to the underlying terminal.
    pub(crate) fn terminal(&self) -> &otty_ui_term::Terminal {
        &self.terminal
    }

    /// Return the current title of this terminal.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }
}

/// Selected block metadata tracked across split panes.
#[derive(Clone, Debug)]
pub(crate) struct BlockSelection {
    terminal_id: u64,
    block_id: String,
}

impl BlockSelection {
    /// Create a new block selection.
    pub(crate) fn new(terminal_id: u64, block_id: String) -> Self {
        Self {
            terminal_id,
            block_id,
        }
    }

    /// Return the terminal that owns the selected block.
    pub(crate) fn terminal_id(&self) -> u64 {
        self.terminal_id
    }

    /// Return the block identifier within the terminal.
    pub(crate) fn block_id(&self) -> &str {
        &self.block_id
    }
}

// ---------------------------------------------------------------------------
// View models
// ---------------------------------------------------------------------------

/// View model for a single terminal tab, used by the pane grid view.
#[derive(Clone, Copy)]
pub(crate) struct TerminalTabViewModel<'a> {
    pub(crate) tab_id: u64,
    pub(crate) panes: &'a pane_grid::State<u64>,
    pub(crate) terminals: &'a HashMap<u64, TerminalEntry>,
    pub(crate) focus: Option<pane_grid::Pane>,
    pub(crate) context_menu: Option<&'a PaneContextMenuState>,
    pub(crate) has_block_selection: bool,
}

/// View model for the entire terminal workspace, keyed by active tab id.
#[derive(Clone, Copy)]
pub(crate) struct TerminalWorkspaceViewModel<'a> {
    pub(crate) tab: Option<TerminalTabViewModel<'a>>,
}

#[cfg(test)]
mod tests {
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

    use super::{BlockSelection, ShellSession, TerminalKind};

    #[test]
    fn given_shell_session_when_constructed_then_getters_return_values() {
        let session_kind = SessionKind::from_local_options(
            LocalSessionOptions::default().with_program("/bin/sh"),
        );
        let session = ShellSession::new(String::from("shell"), session_kind);

        assert_eq!(session.name(), "shell");
        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.program(), "/bin/sh");
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }

    #[test]
    fn given_block_selection_when_constructed_then_getters_return_values() {
        let selection = BlockSelection::new(42, String::from("block-1"));

        assert_eq!(selection.terminal_id(), 42);
        assert_eq!(selection.block_id(), "block-1");
    }

    #[test]
    fn given_terminal_kind_values_when_compared_then_equality_matches() {
        assert_eq!(TerminalKind::Shell, TerminalKind::Shell);
        assert_ne!(TerminalKind::Shell, TerminalKind::Command);
    }
}
