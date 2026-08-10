/// Terminal context determining whether shell metadata is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalKind {
    /// Interactive shell session.
    Shell,
    /// One-shot command execution.
    Command,
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

#[cfg(test)]
mod tests {
    use super::{BlockSelection, TerminalKind};

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
