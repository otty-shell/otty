use std::collections::HashMap;

use iced::widget::pane_grid;

use super::types::TerminalEntry;
use super::state::PaneContextMenuState;

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
