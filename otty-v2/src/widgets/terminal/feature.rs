use std::collections::HashMap;
use std::path::PathBuf;

use iced::widget::pane_grid;
use iced::{Point, Size, Task};
use otty_ui_term::settings::Settings;
use otty_ui_term::{BlockCommand, TerminalView};

use super::event::TerminalEvent;
use super::model::TerminalKind;
use super::state::{TerminalCommand, TerminalState, TerminalTabState};
use crate::app::Event as AppEvent;
use crate::widgets::explorer::ExplorerUiEvent;

/// Runtime context injected by `App` into each terminal reduce call.
pub(crate) struct TerminalCtx {
    /// Active tab identifier at the time of dispatch.
    pub(crate) active_tab_id: Option<u64>,
    /// Available pane grid area for the terminal viewport.
    pub(crate) pane_grid_size: Size,
    /// Full screen area used for context menu placement.
    pub(crate) screen_size: Size,
    /// Current cursor position in sidebar-relative coordinates.
    pub(crate) sidebar_cursor: Point,
}

/// Terminal feature root that owns all terminal state and reduction logic.
pub(crate) struct TerminalFeature {
    state: TerminalState,
    terminal_to_tab: HashMap<u64, u64>,
    next_terminal_id: u64,
}

impl TerminalFeature {
    /// Construct terminal feature with default empty state.
    pub(crate) fn new() -> Self {
        Self {
            state: TerminalState::default(),
            terminal_to_tab: HashMap::new(),
            next_terminal_id: 0,
        }
    }

    /// Return read-only access to the underlying terminal state.
    pub(crate) fn state(&self) -> &TerminalState {
        &self.state
    }

    /// Iterate all terminal tabs.
    pub(crate) fn tabs(
        &self,
    ) -> impl Iterator<Item = (&u64, &TerminalTabState)> {
        self.state.tabs()
    }

    /// Return terminal tab state by tab id.
    #[cfg(test)]
    pub(crate) fn tab(&self, tab_id: u64) -> Option<&TerminalTabState> {
        self.state.tab(tab_id)
    }

    /// Return the active terminal tab if it is a shell tab.
    ///
    /// Used by explorer sync to locate the current working directory.
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

    /// Look up the tab identifier that owns a given terminal.
    #[cfg(test)]
    pub(crate) fn terminal_tab_id(&self, terminal_id: u64) -> Option<u64> {
        self.terminal_to_tab.get(&terminal_id).copied()
    }

    /// Remove all terminal → tab entries that belong to a closed tab.
    pub(crate) fn remove_tab_terminals(&mut self, tab_id: u64) {
        self.terminal_to_tab
            .retain(|_, mapped_tab| *mapped_tab != tab_id);
    }

    /// Rebuild the terminal → tab index from the current tab states.
    pub(crate) fn reindex_terminal_tabs(&mut self) {
        self.terminal_to_tab.clear();
        for (&tab_id, tab) in self.state.tabs() {
            for terminal_id in tab.terminals().keys().copied() {
                self.terminal_to_tab.insert(terminal_id, tab_id);
            }
        }
    }

    /// Apply a new pane grid size to every terminal tab.
    pub(crate) fn set_grid_size(&mut self, size: Size) {
        for (_, tab) in self.state.tabs_mut() {
            tab.set_grid_size(size);
        }
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn open_tab(
        &mut self,
        ctx: &TerminalCtx,
        tab_id: u64,
        terminal_id: u64,
        default_title: String,
        settings: Settings,
        kind: TerminalKind,
        sync_explorer: bool,
        error_tab: Option<(String, String)>,
    ) -> Task<AppEvent> {
        let (mut terminal, widget_id) = match TerminalTabState::new(
            tab_id,
            default_title,
            terminal_id,
            settings,
            kind,
        ) {
            Ok(result) => result,
            Err(err) => {
                log::warn!("failed to create terminal tab: {err}");
                if let Some((title, message)) = error_tab {
                    return Task::done(AppEvent::OpenQuickLaunchErrorTab {
                        title,
                        message: format!("{message}\nError: {err}"),
                    });
                }
                return Task::done(AppEvent::CloseTabRequested { tab_id });
            },
        };

        terminal.set_grid_size(ctx.pane_grid_size);
        for t_id in terminal.terminals().keys().copied() {
            self.terminal_to_tab.insert(t_id, tab_id);
        }

        let title = terminal.title().to_string();
        self.state.insert_tab(tab_id, terminal);

        let mut tasks = vec![
            TerminalView::focus(widget_id),
            request_terminal_event(TerminalEvent::SyncSelection { tab_id }),
            request_tab_title(tab_id, title),
        ];
        if sync_explorer {
            tasks.push(request_sync_explorer());
        }

        Task::batch(tasks)
    }

    fn reduce_widget_event(
        &mut self,
        ctx: &TerminalCtx,
        event: otty_ui_term::Event,
    ) -> Task<AppEvent> {
        let terminal_id = *event.terminal_id();
        let Some(tab_id) = self.terminal_to_tab.get(&terminal_id).copied()
        else {
            return Task::none();
        };

        let refresh_titles = matches!(
            &event,
            otty_ui_term::Event::TitleChanged { .. }
                | otty_ui_term::Event::ResetTitle { .. }
        );

        let is_shutdown =
            matches!(&event, otty_ui_term::Event::Shutdown { .. });
        let selection_task = self.update_block_selection(tab_id, &event);
        let event_task = self
            .with_terminal_tab(tab_id, |tab| tab.handle_terminal_event(event));
        let _ = ctx;
        let update = Task::batch(vec![selection_task, event_task]);

        if is_shutdown {
            self.reindex_terminal_tabs();
        }

        if !refresh_titles {
            return update;
        }

        let title_task = self
            .state
            .tab(tab_id)
            .map(|tab| request_tab_title(tab_id, tab.title().to_string()))
            .unwrap_or_else(Task::none);
        Task::batch(vec![update, title_task])
    }

    fn focus_active(&self, active_tab_id: Option<u64>) -> Task<AppEvent> {
        let Some(terminal) =
            active_tab_id.and_then(|tab_id| self.state.tab(tab_id))
        else {
            return Task::none();
        };

        match terminal.focused_terminal_entry() {
            Some(entry) => {
                TerminalView::focus(entry.terminal.widget_id().clone())
            },
            None => Task::none(),
        }
    }

    fn apply_theme(
        &mut self,
        palette: otty_ui_term::ColorPalette,
    ) -> Task<AppEvent> {
        for (_, terminal) in self.state.tabs_mut() {
            terminal.apply_theme(palette.clone());
        }
        Task::none()
    }

    fn close_all_context_menus(&mut self) -> Task<AppEvent> {
        let mut commands = Vec::new();
        for (_, terminal) in self.state.tabs_mut() {
            if terminal.context_menu().is_some() {
                commands.push(terminal.close_context_menu());
            }
        }
        Task::batch(commands.into_iter().map(execute_command))
    }

    fn sync_tab_block_selection(&self, tab_id: u64) -> Task<AppEvent> {
        let Some(terminal) = self.state.tab(tab_id) else {
            return Task::none();
        };
        let selection = terminal.selected_block().cloned();
        let mut tasks = Vec::new();

        for entry in terminal.terminals().values() {
            let cmd = if let Some(sel) = &selection
                && sel.terminal_id() == entry.terminal.id
            {
                BlockCommand::Select(sel.block_id().to_string())
            } else {
                BlockCommand::ClearSelection
            };
            tasks.push(TerminalView::command(
                entry.terminal.widget_id().clone(),
                cmd,
            ));
        }

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    fn split_pane(
        &mut self,
        tab_id: u64,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
    ) -> Task<AppEvent> {
        let terminal_id = self.allocate_terminal_id();
        let task = self.with_terminal_tab(tab_id, move |tab| {
            tab.split_pane(pane, axis, terminal_id)
        });

        if self
            .state
            .tab(tab_id)
            .map(|tab| tab.contains_terminal(terminal_id))
            .unwrap_or(false)
        {
            self.terminal_to_tab.insert(terminal_id, tab_id);
        }

        task
    }

    fn close_pane(
        &mut self,
        tab_id: u64,
        pane: pane_grid::Pane,
    ) -> Task<AppEvent> {
        let task = self.with_terminal_tab(tab_id, |tab| tab.close_pane(pane));
        self.reindex_terminal_tabs();
        task
    }

    fn update_block_selection(
        &mut self,
        tab_id: u64,
        event: &otty_ui_term::Event,
    ) -> Task<AppEvent> {
        use otty_ui_term::Event::*;

        match event {
            BlockSelected { block_id, .. } => {
                let terminal_id = *event.terminal_id();
                let other_widget_ids: Vec<_> = self
                    .state
                    .tab(tab_id)
                    .map(|tab| {
                        tab.terminals()
                            .values()
                            .filter(|e| e.terminal.id != terminal_id)
                            .map(|e| e.terminal.widget_id().clone())
                            .collect()
                    })
                    .unwrap_or_default();

                let _ = self.with_terminal_tab(tab_id, |tab| {
                    tab.set_selected_block(terminal_id, block_id.clone());
                    TerminalCommand::None
                });

                if other_widget_ids.is_empty() {
                    Task::none()
                } else {
                    Task::batch(other_widget_ids.into_iter().map(|id| {
                        TerminalView::command(id, BlockCommand::ClearSelection)
                    }))
                }
            },
            BlockSelectionCleared { .. } => {
                let terminal_id = *event.terminal_id();
                self.with_terminal_tab(tab_id, |tab| {
                    if tab
                        .selected_block()
                        .map(|sel| sel.terminal_id() == terminal_id)
                        .unwrap_or(false)
                    {
                        tab.clear_selected_block();
                    }
                    TerminalCommand::None
                })
            },
            BlockCopied { .. } => Task::none(),
            _ => Task::none(),
        }
    }

    fn copy_selected_block(
        &mut self,
        tab_id: u64,
        terminal_id: u64,
        kind: CopyKind,
    ) -> Task<AppEvent> {
        let Some((block_id, widget_id)) =
            self.state.tab(tab_id).and_then(|tab| {
                let selection = tab.selected_block()?;
                if selection.terminal_id() != terminal_id {
                    return None;
                }
                let block_id = selection.block_id().to_string();
                let widget_id = tab
                    .terminals()
                    .get(&terminal_id)?
                    .terminal
                    .widget_id()
                    .clone();
                Some((block_id, widget_id))
            })
        else {
            return Task::none();
        };

        let command = match kind {
            CopyKind::Content => BlockCommand::CopyContent(block_id),
            CopyKind::Prompt => BlockCommand::CopyPrompt(block_id),
            CopyKind::Command => BlockCommand::CopyCommand(block_id),
        };

        let close_cmd =
            self.with_terminal_tab(tab_id, |tab| tab.close_context_menu());
        let copy_task = TerminalView::command(widget_id, command);
        Task::batch(vec![close_cmd, copy_task])
    }

    fn copy_selection(
        &mut self,
        tab_id: u64,
        terminal_id: u64,
    ) -> Task<AppEvent> {
        let Some(widget_id) = self
            .state
            .tab(tab_id)
            .and_then(|tab| tab.terminals().get(&terminal_id))
            .map(|entry| entry.terminal.widget_id().clone())
        else {
            return Task::none();
        };

        let close_cmd =
            self.with_terminal_tab(tab_id, |tab| tab.close_context_menu());
        let copy_task =
            TerminalView::command(widget_id, BlockCommand::CopySelection);
        Task::batch(vec![close_cmd, copy_task])
    }

    fn paste_into_prompt(
        &mut self,
        tab_id: u64,
        terminal_id: u64,
    ) -> Task<AppEvent> {
        let Some(widget_id) = self
            .state
            .tab(tab_id)
            .and_then(|tab| tab.terminals().get(&terminal_id))
            .map(|entry| entry.terminal.widget_id().clone())
        else {
            return Task::none();
        };

        let close_cmd =
            self.with_terminal_tab(tab_id, |tab| tab.close_context_menu());
        let paste_task =
            TerminalView::command(widget_id, BlockCommand::PasteClipboard);
        Task::batch(vec![close_cmd, paste_task])
    }

    fn with_terminal_tab<F>(&mut self, tab_id: u64, f: F) -> Task<AppEvent>
    where
        F: FnOnce(&mut TerminalTabState) -> TerminalCommand,
    {
        let (cmd, title) = {
            let Some(tab) = self.state.tab_mut(tab_id) else {
                return Task::none();
            };
            let cmd = f(tab);
            let title = tab.title().to_string();
            (cmd, title)
        };

        let command_task = execute_command(cmd);
        let title_task = request_tab_title(tab_id, title);
        Task::batch(vec![command_task, title_task])
    }
}

impl TerminalFeature {
    /// Reduce a terminal event into state updates and routed app tasks.
    pub(crate) fn reduce(
        &mut self,
        event: TerminalEvent,
        ctx: &TerminalCtx,
    ) -> Task<AppEvent> {
        use TerminalEvent::*;

        match event {
            OpenTab {
                tab_id,
                terminal_id,
                default_title,
                settings,
                kind,
                sync_explorer,
                error_tab,
            } => self.open_tab(
                ctx,
                tab_id,
                terminal_id,
                default_title,
                *settings,
                kind,
                sync_explorer,
                error_tab,
            ),
            TabClosed { tab_id } => {
                let _ = self.state.remove_tab(tab_id);
                self.remove_tab_terminals(tab_id);
                Task::none()
            },
            Widget(event) => self.reduce_widget_event(ctx, event),
            PaneClicked { tab_id, pane } => {
                self.with_terminal_tab(tab_id, |tab| tab.focus_pane(pane))
            },
            PaneResized { tab_id, event } => {
                self.with_terminal_tab(tab_id, |tab| {
                    tab.resize(event);
                    TerminalCommand::None
                })
            },
            PaneGridCursorMoved { tab_id, position } => self
                .with_terminal_tab(tab_id, |tab| {
                    tab.update_grid_cursor(position)
                }),
            OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            } => {
                let cursor = ctx.sidebar_cursor;
                let grid_size = ctx.screen_size;
                self.with_terminal_tab(tab_id, |tab| {
                    tab.open_context_menu(pane, terminal_id, cursor, grid_size)
                })
            },
            CloseContextMenu { tab_id } => {
                self.with_terminal_tab(tab_id, |tab| tab.close_context_menu())
            },
            ContextMenuInput { tab_id: _ } => Task::none(),
            SplitPane { tab_id, pane, axis } => {
                self.split_pane(tab_id, pane, axis)
            },
            ClosePane { tab_id, pane } => self.close_pane(tab_id, pane),
            CopySelection {
                tab_id,
                terminal_id,
            } => self.copy_selection(tab_id, terminal_id),
            PasteIntoPrompt {
                tab_id,
                terminal_id,
            } => self.paste_into_prompt(tab_id, terminal_id),
            CopySelectedBlockContent {
                tab_id,
                terminal_id,
            } => {
                self.copy_selected_block(tab_id, terminal_id, CopyKind::Content)
            },
            CopySelectedBlockPrompt {
                tab_id,
                terminal_id,
            } => {
                self.copy_selected_block(tab_id, terminal_id, CopyKind::Prompt)
            },
            CopySelectedBlockCommand {
                tab_id,
                terminal_id,
            } => {
                self.copy_selected_block(tab_id, terminal_id, CopyKind::Command)
            },
            ApplyTheme { palette } => self.apply_theme(*palette),
            CloseAllContextMenus => self.close_all_context_menus(),
            FocusActive => self.focus_active(ctx.active_tab_id),
            SyncSelection { tab_id } => self.sync_tab_block_selection(tab_id),
        }
    }
}

// ── Module-private helper types ───────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
enum CopyKind {
    Content,
    Prompt,
    Command,
}

// ── Free helpers used by the feature ─────────────────────────────────────────

fn execute_command(command: TerminalCommand) -> Task<AppEvent> {
    match command {
        TerminalCommand::None => Task::none(),
        TerminalCommand::FocusTerminal(id) => TerminalView::focus(id),
        TerminalCommand::SelectHovered(id) => {
            TerminalView::command(id, BlockCommand::SelectHovered)
        },
        TerminalCommand::FocusElement(id) => iced::widget::operation::focus(id),
        TerminalCommand::CloseTab { tab_id } => {
            Task::done(AppEvent::CloseTabRequested { tab_id })
        },
        TerminalCommand::Batch(cmds) => {
            Task::batch(cmds.into_iter().map(execute_command))
        },
    }
}

fn request_sync_explorer() -> Task<AppEvent> {
    Task::done(AppEvent::ExplorerUi(
        ExplorerUiEvent::SyncFromActiveTerminal,
    ))
}

fn request_terminal_event(event: TerminalEvent) -> Task<AppEvent> {
    Task::done(AppEvent::Terminal(event))
}

fn request_tab_title(tab_id: u64, title: String) -> Task<AppEvent> {
    Task::done(AppEvent::SetTabTitle { tab_id, title })
}

/// Resolve active-terminal working directory from a terminal feature.
///
/// Used by the explorer sync service after the `State` decoupling.
pub(crate) fn shell_cwd_for_active_tab(
    active_tab_id: Option<u64>,
    terminal: &TerminalFeature,
) -> Option<PathBuf> {
    let terminal_tab = terminal.active_shell_tab(active_tab_id)?;
    terminal_tab
        .focused_terminal_entry()
        .and_then(|entry| terminal_cwd_from_blocks(&entry.terminal.blocks()))
}

fn terminal_cwd_from_blocks(
    blocks: &[otty_ui_term::BlockSnapshot],
) -> Option<PathBuf> {
    blocks
        .iter()
        .rev()
        .find_map(|block| block.meta.cwd.as_deref())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid;
    use iced::{Point, Size};
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{TerminalCtx, TerminalFeature};
    use crate::widgets::terminal::TerminalKind;
    use crate::widgets::terminal::event::TerminalEvent;

    #[cfg(unix)]
    const VALID_SHELL_PATH: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const VALID_SHELL_PATH: &str = "cmd.exe";

    fn settings_with_program(program: &str) -> Settings {
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(program),
            ),
        );
        settings
    }

    fn default_ctx() -> TerminalCtx {
        TerminalCtx {
            active_tab_id: None,
            pane_grid_size: Size::ZERO,
            screen_size: Size::ZERO,
            sidebar_cursor: Point::ORIGIN,
        }
    }

    #[test]
    fn given_open_tab_event_when_reduced_then_terminal_feature_stores_tab() {
        let mut feature = TerminalFeature::new();
        let ctx = default_ctx();

        let _task = feature.reduce(
            TerminalEvent::OpenTab {
                tab_id: 1,
                terminal_id: 10,
                default_title: String::from("Shell"),
                settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
                kind: TerminalKind::Shell,
                sync_explorer: true,
                error_tab: None,
            },
            &ctx,
        );

        assert!(feature.tab(1).is_some());
        assert_eq!(feature.terminal_tab_id(10), Some(1));
    }

    #[test]
    fn given_missing_tab_when_pane_clicked_then_reducer_ignores_event() {
        let mut feature = TerminalFeature::new();
        let ctx = default_ctx();

        let (_grid, pane) = pane_grid::State::new(1_u64);
        let _task = feature
            .reduce(TerminalEvent::PaneClicked { tab_id: 999, pane }, &ctx);

        assert!(feature.tab(999).is_none());
    }

    #[test]
    fn given_tab_closed_event_when_reduced_then_terminal_tab_is_removed() {
        let mut feature = TerminalFeature::new();
        let ctx = default_ctx();

        let _ = feature.reduce(
            TerminalEvent::OpenTab {
                tab_id: 1,
                terminal_id: 10,
                default_title: String::from("Shell"),
                settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
                kind: TerminalKind::Shell,
                sync_explorer: false,
                error_tab: None,
            },
            &ctx,
        );

        let _task =
            feature.reduce(TerminalEvent::TabClosed { tab_id: 1 }, &ctx);

        assert!(feature.tab(1).is_none());
        assert_eq!(feature.terminal_tab_id(10), None);
    }
}
