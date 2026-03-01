mod errors;
pub(crate) mod event;
pub(crate) mod model;
pub(crate) mod reducer;
pub(crate) mod services;
pub(crate) mod state;
pub(crate) mod view;

use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) use event::{
    TerminalWorkspaceEffect, TerminalWorkspaceEvent, TerminalWorkspaceUiEvent,
};
use iced::{Size, Task};
pub(crate) use reducer::TerminalWorkspaceCtx;
use state::{TerminalTabState, TerminalWorkspaceState};

use self::model::{TerminalTabViewModel, TerminalWorkspaceViewModel};

/// Terminal workspace widget: manages terminal sessions across tabs.
///
/// Each terminal tab contains a pane grid with one or more terminal
/// panes. The widget handles tab and pane lifecycle, context menus,
/// theme application, and terminal widget events.
pub(crate) struct TerminalWorkspaceWidget {
    state: TerminalWorkspaceState,
    terminal_to_tab: HashMap<u64, u64>,
    next_terminal_id: u64,
}

impl TerminalWorkspaceWidget {
    /// Create a new widget with default empty state.
    pub(crate) fn new() -> Self {
        Self {
            state: TerminalWorkspaceState::default(),
            terminal_to_tab: HashMap::new(),
            next_terminal_id: 0,
        }
    }

    /// Reduce a UI event into state updates and effect events.
    pub(crate) fn reduce(
        &mut self,
        event: TerminalWorkspaceUiEvent,
        ctx: &TerminalWorkspaceCtx,
    ) -> Task<TerminalWorkspaceEvent> {
        reducer::reduce(
            &mut self.state,
            &mut self.terminal_to_tab,
            &mut self.next_terminal_id,
            event,
            ctx,
        )
    }

    /// Return a view model for the given active tab.
    pub(crate) fn vm(
        &self,
        active_tab_id: Option<u64>,
    ) -> TerminalWorkspaceViewModel<'_> {
        let tab = active_tab_id.and_then(|tab_id| self.state.tab(tab_id)).map(
            |tab| TerminalTabViewModel {
                tab_id: active_tab_id.unwrap_or(0),
                panes: tab.panes(),
                terminals: tab.terminals(),
                focus: tab.focus(),
                context_menu: tab.context_menu(),
                has_block_selection: tab.selected_block().is_some(),
            },
        );

        TerminalWorkspaceViewModel { tab }
    }

    /// Return read-only access to a tab state.
    pub(crate) fn tab(&self, tab_id: u64) -> Option<&TerminalTabState> {
        self.state.tab(tab_id)
    }

    /// Iterate all terminal tabs.
    pub(crate) fn tabs(
        &self,
    ) -> impl Iterator<Item = (&u64, &TerminalTabState)> {
        self.state.tabs()
    }

    /// Return the active shell tab if it is a shell session.
    pub(crate) fn active_shell_tab(
        &self,
        active_tab_id: Option<u64>,
    ) -> Option<&TerminalTabState> {
        let tab_id = active_tab_id?;
        self.state.tab(tab_id).filter(|tab| tab.is_shell())
    }

    /// Return whether any terminal tab has an open context menu.
    pub(crate) fn has_any_context_menu(&self) -> bool {
        self.state
            .tabs()
            .any(|(_, tab)| tab.context_menu().is_some())
    }

    /// Allocate a new unique terminal identifier.
    pub(crate) fn allocate_terminal_id(&mut self) -> u64 {
        let id = self.next_terminal_id;
        self.next_terminal_id += 1;
        id
    }

    /// Apply a new pane grid size to every terminal tab.
    pub(crate) fn set_grid_size(&mut self, size: Size) {
        for (_, tab) in self.state.tabs_mut() {
            tab.set_grid_size(size);
        }
    }

    /// Resolve the active terminal working directory from block metadata.
    pub(crate) fn shell_cwd_for_active_tab(
        &self,
        active_tab_id: Option<u64>,
    ) -> Option<PathBuf> {
        let tab = self.active_shell_tab(active_tab_id)?;
        tab.focused_terminal_entry().and_then(|entry| {
            services::terminal_cwd_from_blocks(&entry.terminal().blocks())
        })
    }

    /// Return read-only access to state for tests.
    #[cfg(test)]
    pub(crate) fn state(&self) -> &TerminalWorkspaceState {
        &self.state
    }

    /// Return the terminal-to-tab index for tests.
    #[cfg(test)]
    pub(crate) fn terminal_tab_id(&self, terminal_id: u64) -> Option<u64> {
        self.terminal_to_tab.get(&terminal_id).copied()
    }
}
