use std::process::ExitStatus;

use super::model::PaneLabel;
use crate::domain::exit_behavior::ExitBehavior;
use crate::shortcuts::AppShortcut;

/// The terminal widget this application embeds, bound to its own
/// action type.
pub(crate) type Terminal = otty_ui_term::Terminal<AppShortcut>;

/// An event reported by one of this application's terminal widgets.
pub(crate) type TerminalEvent = otty_ui_term::Event<AppShortcut>;

/// Terminal context determining whether shell metadata is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalKind {
    /// Interactive shell session.
    Shell,
    /// One-shot command execution.
    Command {
        /// What the pane does once the command exits.
        exit_behavior: ExitBehavior,
    },
}

/// A terminal entry stored per pane.
pub(crate) struct TerminalEntry {
    pub(super) terminal: Terminal,
    /// Name the pane opened with: the shell or launch name.
    pub(super) title: String,
    /// Set once the process exited and the pane was kept open.
    pub(super) exit_status: Option<ExitStatus>,
    /// What the pane runs and where, kept up to date on terminal events so
    /// views read it instead of rebuilding it every frame.
    label: PaneLabel,
}

impl TerminalEntry {
    /// Return read-only reference to the underlying terminal.
    pub(crate) fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    /// Return the name the pane opened with: the shell or launch name.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Return how the process ended, if it exited and the pane stayed
    /// open.
    pub(crate) fn exit_status(&self) -> Option<ExitStatus> {
        self.exit_status
    }

    /// Return what the pane runs and where.
    pub(crate) fn label(&self) -> &PaneLabel {
        &self.label
    }

    /// Create the entry of a new pane opened as `title`.
    pub(super) fn new(terminal: Terminal, title: String) -> Self {
        let mut entry = Self {
            terminal,
            label: PaneLabel::new(&title, None, None),
            title,
            exit_status: None,
        };
        entry.refresh_label();

        entry
    }

    /// Read the program and folder again from the terminal. Returns whether
    /// they changed.
    pub(super) fn refresh_label(&mut self) -> bool {
        let label = PaneLabel::for_entry(self);
        if label == self.label {
            return false;
        }

        self.label = label;
        true
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

#[cfg(test)]
mod tests {
    use super::{BlockSelection, TerminalKind};
    use crate::domain::exit_behavior::ExitBehavior;

    #[test]
    fn given_block_selection_when_constructed_then_getters_return_values() {
        let selection = BlockSelection::new(42, String::from("block-1"));

        assert_eq!(selection.terminal_id(), 42);
        assert_eq!(selection.block_id(), "block-1");
    }

    #[test]
    fn given_terminal_kind_values_when_compared_then_equality_matches() {
        assert_eq!(TerminalKind::Shell, TerminalKind::Shell);
        assert_ne!(
            TerminalKind::Shell,
            TerminalKind::Command {
                exit_behavior: ExitBehavior::Close,
            }
        );
    }
}
