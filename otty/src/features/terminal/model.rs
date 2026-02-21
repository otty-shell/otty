use iced::widget::pane_grid;
use otty_ui_term::settings::SessionKind;

/// Shell session information needed to start a terminal backend.
#[derive(Debug, Clone)]
pub(crate) struct ShellSession {
    pub name: String,
    pub session: SessionKind,
}

/// Terminal entry used by the tab view.
pub(crate) struct TerminalEntry {
    pub(crate) pane: pane_grid::Pane,
    pub(crate) terminal: otty_ui_term::Terminal,
    pub(crate) title: String,
}

/// Terminal context determining whether shell metadata is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalKind {
    Shell,
    Command,
}

/// Selected block metadata tracked across split panes.
#[derive(Clone, Debug)]
pub(crate) struct BlockSelection {
    pub terminal_id: u64,
    pub block_id: String,
}
