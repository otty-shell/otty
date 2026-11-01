mod errors;
pub(crate) mod event;
pub(crate) mod model;
mod pane_balance;
pub(crate) mod reducer;
pub(crate) mod services;
pub(crate) mod state;
pub(crate) mod types;
pub(crate) mod view;

use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) use event::{
    TerminalWorkspaceEffect, TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};
use iced::Task;
use iced::widget::pane_grid;
use otty_ui_kit::components::target_dot::TargetKind;
pub(crate) use reducer::TerminalWorkspaceCtx;
use state::{TerminalTabState, TerminalWorkspaceState};

use self::model::{
    PaneLabel, TerminalTabViewModel, TerminalWorkspaceViewModel,
};

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

    /// Reduce an intent event into state updates and effect events.
    pub(crate) fn reduce(
        &mut self,
        event: TerminalWorkspaceIntent,
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

    /// Return a view model for the given active tab. `machine` and
    /// `machine_target` name the machine the tab runs on, for the chip in
    /// every pane header.
    pub(crate) fn vm<'a>(
        &'a self,
        active_tab_id: Option<u64>,
        machine: &'a str,
        machine_target: TargetKind,
    ) -> TerminalWorkspaceViewModel<'a> {
        let tab = active_tab_id.and_then(|tab_id| self.state.tab(tab_id)).map(
            |tab| TerminalTabViewModel {
                tab_id: active_tab_id.unwrap_or(0),
                panes: tab.panes(),
                terminals: tab.terminals(),
                focus: tab.focus(),
                machine,
                machine_target,
            },
        );

        TerminalWorkspaceViewModel { tab }
    }

    /// Iterate all terminal tabs.
    pub(crate) fn tabs(
        &self,
    ) -> impl Iterator<Item = (&u64, &TerminalTabState)> {
        self.state.tabs()
    }

    /// Return the active terminal tab if it runs on this computer: a shell
    /// or a local launch.
    pub(crate) fn active_local_tab(
        &self,
        active_tab_id: Option<u64>,
    ) -> Option<&TerminalTabState> {
        let tab_id = active_tab_id?;
        self.state.tab(tab_id).filter(|tab| tab.runs_locally())
    }

    /// Return whether any terminal tab has an open context menu.
    pub(crate) fn has_any_context_menu(&self) -> bool {
        self.state
            .tabs()
            .any(|(_, tab)| tab.context_menu().is_some())
    }

    /// Resolve the working directory of the active local terminal tab from
    /// block metadata. `None` until its shell reports one.
    pub(crate) fn local_cwd_for_active_tab(
        &self,
        active_tab_id: Option<u64>,
    ) -> Option<PathBuf> {
        let tab = self.active_local_tab(active_tab_id)?;
        let entry = tab.focused_terminal_entry()?;
        let snapshot = entry.terminal().snapshot_arc();

        services::terminal_cwd_from_blocks(snapshot.view().blocks())
    }

    /// Return a terminal tab with its focused pane: the pane a split of
    /// that tab acts on. `None` when `tab_id` is not a terminal tab.
    pub(crate) fn focused_pane(
        &self,
        tab_id: Option<u64>,
    ) -> Option<(u64, pane_grid::Pane)> {
        let tab_id = tab_id?;
        let pane = self.state.tab(tab_id)?.focus()?;

        Some((tab_id, pane))
    }

    /// Return the program and folder of the focused pane of a terminal tab.
    pub(crate) fn focused_pane_label(
        &self,
        tab_id: Option<u64>,
    ) -> Option<&PaneLabel> {
        let entry = self.state.tab(tab_id?)?.focused_terminal_entry()?;

        Some(entry.label())
    }

    /// Return the number of panes in a terminal tab.
    pub(crate) fn pane_count(&self, tab_id: Option<u64>) -> Option<usize> {
        self.state.tab(tab_id?).map(|tab| tab.panes().len())
    }
}

#[cfg(test)]
impl TerminalWorkspaceWidget {
    /// Return read-only access to a tab state.
    pub(crate) fn tab(&self, tab_id: u64) -> Option<&TerminalTabState> {
        self.state.tab(tab_id)
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::*;
    use crate::widgets::terminal_workspace::types::TerminalKind;

    #[cfg(unix)]
    const TEST_SHELL_PATH: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const TEST_SHELL_PATH: &str = "cmd.exe";

    /// Open a terminal tab with one real shell pane.
    fn open_shell_tab(widget: &mut TerminalWorkspaceWidget, tab_id: u64) {
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(TEST_SHELL_PATH),
            ),
        );
        let ctx = TerminalWorkspaceCtx {
            active_tab_id: Some(tab_id),
            pane_grid_size: Size::new(800.0, 600.0),
            screen_size: Size::new(800.0, 600.0),
            sidebar_cursor: Point::ORIGIN,
            equalize_panes: true,
        };

        let _ = widget.reduce(
            TerminalWorkspaceIntent::OpenTab {
                tab_id,
                default_title: String::from("sh"),
                settings: Box::new(settings),
                kind: TerminalKind::Shell,
                sync_explorer: false,
            },
            &ctx,
        );
    }

    #[test]
    fn given_terminal_tab_when_focused_pane_asked_then_tab_and_pane_return() {
        let mut widget = TerminalWorkspaceWidget::new();
        open_shell_tab(&mut widget, 3);
        let focus = widget
            .tab(3)
            .and_then(|tab| tab.focus())
            .expect("shell tab should have a focused pane");

        let focused = widget.focused_pane(Some(3));

        assert_eq!(focused, Some((3, focus)));
    }

    #[test]
    fn given_no_terminal_tab_when_focused_pane_asked_then_none_returns() {
        let mut widget = TerminalWorkspaceWidget::new();
        open_shell_tab(&mut widget, 3);

        assert_eq!(widget.focused_pane(None), None);
        assert_eq!(widget.focused_pane(Some(9)), None);
    }
}
