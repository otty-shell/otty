use std::collections::HashMap;

use iced::widget::pane_grid;
use iced::{Point, Size, Task};
use otty_ui_term::{BlockCommand, TerminalView};

use super::event::{
    TerminalWorkspaceEffect, TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};
use super::model::TerminalKind;
use super::state::{StateCommand, TerminalTabState, TerminalWorkspaceState};

/// Runtime context injected into each reduce call.
pub(crate) struct TerminalWorkspaceCtx {
    /// Active tab identifier at the time of dispatch.
    pub(crate) active_tab_id: Option<u64>,
    /// Available pane grid area for the terminal viewport.
    pub(crate) pane_grid_size: Size,
    /// Full screen area used for context menu placement.
    pub(crate) screen_size: Size,
    /// Current cursor position in sidebar-relative coordinates.
    pub(crate) sidebar_cursor: Point,
}

/// Reduce a terminal workspace intent event into state updates and effects.
pub(crate) fn reduce(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    next_terminal_id: &mut u64,
    event: TerminalWorkspaceIntent,
    ctx: &TerminalWorkspaceCtx,
) -> Task<TerminalWorkspaceEvent> {
    use TerminalWorkspaceIntent::*;

    match event {
        OpenTab {
            tab_id,
            terminal_id,
            default_title,
            settings,
            kind,
            sync_explorer,
        } => reduce_open_tab(
            state,
            terminal_to_tab,
            ctx,
            tab_id,
            terminal_id,
            default_title,
            *settings,
            kind,
            sync_explorer,
        ),
        TabClosed { tab_id } => {
            let _ = state.remove_tab(tab_id);
            terminal_to_tab.retain(|_, mapped_tab| *mapped_tab != tab_id);
            Task::none()
        },
        Widget(event) => {
            reduce_widget_event(state, terminal_to_tab, next_terminal_id, event)
        },
        PaneClicked { tab_id, pane } => {
            with_terminal_tab(state, tab_id, |tab| tab.focus_pane(pane))
        },
        PaneResized { tab_id, event } => {
            with_terminal_tab(state, tab_id, |tab| {
                tab.resize(event);
                StateCommand::None
            })
        },
        PaneGridCursorMoved { tab_id, position } => {
            with_terminal_tab(state, tab_id, |tab| {
                tab.update_grid_cursor(position)
            })
        },
        OpenContextMenu {
            tab_id,
            pane,
            terminal_id,
        } => {
            let cursor = ctx.sidebar_cursor;
            let grid_size = ctx.screen_size;
            with_terminal_tab(state, tab_id, |tab| {
                tab.open_context_menu(pane, terminal_id, cursor, grid_size)
            })
        },
        CloseContextMenu { tab_id } => {
            with_terminal_tab(state, tab_id, |tab| tab.close_context_menu())
        },
        ContextMenuInput { .. } => Task::none(),
        SplitPane { tab_id, pane, axis } => reduce_split_pane(
            state,
            terminal_to_tab,
            next_terminal_id,
            tab_id,
            pane,
            axis,
        ),
        ClosePane { tab_id, pane } => {
            let task =
                with_terminal_tab(state, tab_id, |tab| tab.close_pane(pane));
            reindex_terminal_tabs(state, terminal_to_tab);
            task
        },
        CopySelection {
            tab_id,
            terminal_id,
        } => reduce_copy_selection(state, tab_id, terminal_id),
        PasteIntoPrompt {
            tab_id,
            terminal_id,
        } => reduce_paste_into_prompt(state, tab_id, terminal_id),
        CopySelectedBlockContent {
            tab_id,
            terminal_id,
        } => reduce_copy_selected_block(
            state,
            tab_id,
            terminal_id,
            CopyKind::Content,
        ),
        CopySelectedBlockPrompt {
            tab_id,
            terminal_id,
        } => reduce_copy_selected_block(
            state,
            tab_id,
            terminal_id,
            CopyKind::Prompt,
        ),
        CopySelectedBlockCommand {
            tab_id,
            terminal_id,
        } => reduce_copy_selected_block(
            state,
            tab_id,
            terminal_id,
            CopyKind::Command,
        ),
        ApplyTheme { palette } => {
            for (_, tab) in state.tabs_mut() {
                tab.apply_theme(*palette.clone());
            }
            Task::none()
        },
        CloseAllContextMenus => {
            let mut commands = Vec::new();
            for (_, tab) in state.tabs_mut() {
                if tab.context_menu().is_some() {
                    commands.push(tab.close_context_menu());
                }
            }
            Task::batch(commands.into_iter().map(execute_command))
        },
        FocusActive => reduce_focus_active(state, ctx.active_tab_id),
        SyncSelection { tab_id } => reduce_sync_selection(state, tab_id),
        SyncPaneGridSize => {
            for (_, tab) in state.tabs_mut() {
                tab.set_grid_size(ctx.pane_grid_size);
            }
            Task::none()
        },
    }
}

// ---------------------------------------------------------------------------
// Private reducer helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum CopyKind {
    Content,
    Prompt,
    Command,
}

#[allow(clippy::too_many_arguments)]
fn reduce_open_tab(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    ctx: &TerminalWorkspaceCtx,
    tab_id: u64,
    terminal_id: u64,
    default_title: String,
    settings: otty_ui_term::settings::Settings,
    kind: TerminalKind,
    sync_explorer: bool,
) -> Task<TerminalWorkspaceEvent> {
    let (mut terminal_tab, widget_id) = match TerminalTabState::new(
        tab_id,
        default_title,
        terminal_id,
        settings,
        kind,
    ) {
        Ok(result) => result,
        Err(err) => {
            log::warn!("failed to create terminal tab: {err}");
            return Task::done(TerminalWorkspaceEvent::Effect(
                TerminalWorkspaceEffect::TabClosed { tab_id },
            ));
        },
    };

    terminal_tab.set_grid_size(ctx.pane_grid_size);
    for t_id in terminal_tab.terminals().keys().copied() {
        terminal_to_tab.insert(t_id, tab_id);
    }

    let title = terminal_tab.title().to_string();
    state.insert_tab(tab_id, terminal_tab);

    let mut tasks: Vec<Task<TerminalWorkspaceEvent>> = vec![
        TerminalView::focus(widget_id),
        Task::done(TerminalWorkspaceEvent::Effect(
            TerminalWorkspaceEffect::TitleChanged { tab_id, title },
        )),
    ];
    if sync_explorer {
        tasks.push(Task::done(TerminalWorkspaceEvent::Effect(
            TerminalWorkspaceEffect::SyncExplorer,
        )));
    }

    Task::batch(tasks)
}

fn reduce_widget_event(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    next_terminal_id: &mut u64,
    event: otty_ui_term::Event,
) -> Task<TerminalWorkspaceEvent> {
    let terminal_id = *event.terminal_id();
    let Some(tab_id) = terminal_to_tab.get(&terminal_id).copied() else {
        return Task::none();
    };

    let refresh_titles = matches!(
        &event,
        otty_ui_term::Event::TitleChanged { .. }
            | otty_ui_term::Event::ResetTitle { .. }
    );

    let is_shutdown = matches!(&event, otty_ui_term::Event::Shutdown { .. });
    let selection_task = update_block_selection(state, tab_id, &event);
    let event_task = with_terminal_tab(state, tab_id, |tab| {
        tab.handle_terminal_event(event)
    });
    let _ = next_terminal_id;
    let update = Task::batch(vec![selection_task, event_task]);

    if is_shutdown {
        reindex_terminal_tabs(state, terminal_to_tab);
    }

    if !refresh_titles {
        return update;
    }

    let title_task = state
        .tab(tab_id)
        .map(|tab| {
            Task::done(TerminalWorkspaceEvent::Effect(
                TerminalWorkspaceEffect::TitleChanged {
                    tab_id,
                    title: tab.title().to_string(),
                },
            ))
        })
        .unwrap_or_else(Task::none);

    Task::batch(vec![update, title_task])
}

fn reduce_focus_active(
    state: &TerminalWorkspaceState,
    active_tab_id: Option<u64>,
) -> Task<TerminalWorkspaceEvent> {
    let Some(tab) = active_tab_id.and_then(|id| state.tab(id)) else {
        return Task::none();
    };

    match tab.focused_terminal_entry() {
        Some(entry) => {
            TerminalView::focus(entry.terminal().widget_id().clone())
        },
        None => Task::none(),
    }
}

fn reduce_split_pane(
    state: &mut TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
    next_terminal_id: &mut u64,
    tab_id: u64,
    pane: pane_grid::Pane,
    axis: pane_grid::Axis,
) -> Task<TerminalWorkspaceEvent> {
    let terminal_id = *next_terminal_id;
    *next_terminal_id += 1;

    let task = with_terminal_tab(state, tab_id, move |tab| {
        tab.split_pane(pane, axis, terminal_id)
    });

    if state
        .tab(tab_id)
        .map(|tab| tab.contains_terminal(terminal_id))
        .unwrap_or(false)
    {
        terminal_to_tab.insert(terminal_id, tab_id);
    }

    task
}

fn reduce_copy_selection(
    state: &mut TerminalWorkspaceState,
    tab_id: u64,
    terminal_id: u64,
) -> Task<TerminalWorkspaceEvent> {
    let Some(widget_id) = state
        .tab(tab_id)
        .and_then(|tab| tab.terminals().get(&terminal_id))
        .map(|entry| entry.terminal().widget_id().clone())
    else {
        return Task::none();
    };

    let close_cmd =
        with_terminal_tab(state, tab_id, |tab| tab.close_context_menu());
    let copy_task =
        TerminalView::command(widget_id, BlockCommand::CopySelection);
    Task::batch(vec![close_cmd, copy_task])
}

fn reduce_paste_into_prompt(
    state: &mut TerminalWorkspaceState,
    tab_id: u64,
    terminal_id: u64,
) -> Task<TerminalWorkspaceEvent> {
    let Some(widget_id) = state
        .tab(tab_id)
        .and_then(|tab| tab.terminals().get(&terminal_id))
        .map(|entry| entry.terminal().widget_id().clone())
    else {
        return Task::none();
    };

    let close_cmd =
        with_terminal_tab(state, tab_id, |tab| tab.close_context_menu());
    let paste_task =
        TerminalView::command(widget_id, BlockCommand::PasteClipboard);
    Task::batch(vec![close_cmd, paste_task])
}

fn reduce_copy_selected_block(
    state: &mut TerminalWorkspaceState,
    tab_id: u64,
    terminal_id: u64,
    kind: CopyKind,
) -> Task<TerminalWorkspaceEvent> {
    let Some((block_id, widget_id)) = state.tab(tab_id).and_then(|tab| {
        let selection = tab.selected_block()?;
        if selection.terminal_id() != terminal_id {
            return None;
        }
        let block_id = selection.block_id().to_string();
        let widget_id = tab
            .terminals()
            .get(&terminal_id)?
            .terminal()
            .widget_id()
            .clone();
        Some((block_id, widget_id))
    }) else {
        return Task::none();
    };

    let command = match kind {
        CopyKind::Content => BlockCommand::CopyContent(block_id),
        CopyKind::Prompt => BlockCommand::CopyPrompt(block_id),
        CopyKind::Command => BlockCommand::CopyCommand(block_id),
    };

    let close_cmd =
        with_terminal_tab(state, tab_id, |tab| tab.close_context_menu());
    let copy_task = TerminalView::command(widget_id, command);
    Task::batch(vec![close_cmd, copy_task])
}

fn reduce_sync_selection(
    state: &TerminalWorkspaceState,
    tab_id: u64,
) -> Task<TerminalWorkspaceEvent> {
    let Some(tab) = state.tab(tab_id) else {
        return Task::none();
    };
    let selection = tab.selected_block().cloned();
    let mut tasks = Vec::new();

    for entry in tab.terminals().values() {
        let cmd = if let Some(sel) = &selection
            && sel.terminal_id() == entry.terminal().id
        {
            BlockCommand::Select(sel.block_id().to_string())
        } else {
            BlockCommand::ClearSelection
        };
        tasks.push(TerminalView::command(
            entry.terminal().widget_id().clone(),
            cmd,
        ));
    }

    if tasks.is_empty() {
        Task::none()
    } else {
        Task::batch(tasks)
    }
}

fn update_block_selection(
    state: &mut TerminalWorkspaceState,
    tab_id: u64,
    event: &otty_ui_term::Event,
) -> Task<TerminalWorkspaceEvent> {
    use otty_ui_term::Event::*;

    match event {
        BlockSelected { block_id, .. } => {
            let terminal_id = *event.terminal_id();
            let other_widget_ids: Vec<_> = state
                .tab(tab_id)
                .map(|tab| {
                    tab.terminals()
                        .values()
                        .filter(|e| e.terminal().id != terminal_id)
                        .map(|e| e.terminal().widget_id().clone())
                        .collect()
                })
                .unwrap_or_default();

            let _ = with_terminal_tab(state, tab_id, |tab| {
                tab.set_selected_block(terminal_id, block_id.clone());
                StateCommand::None
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
            with_terminal_tab(state, tab_id, |tab| {
                if tab
                    .selected_block()
                    .map(|sel| sel.terminal_id() == terminal_id)
                    .unwrap_or(false)
                {
                    tab.clear_selected_block();
                }
                StateCommand::None
            })
        },
        _ => Task::none(),
    }
}

// ---------------------------------------------------------------------------
// Command execution and state helpers
// ---------------------------------------------------------------------------

fn with_terminal_tab<F>(
    state: &mut TerminalWorkspaceState,
    tab_id: u64,
    f: F,
) -> Task<TerminalWorkspaceEvent>
where
    F: FnOnce(&mut TerminalTabState) -> StateCommand,
{
    let (cmd, title) = {
        let Some(tab) = state.tab_mut(tab_id) else {
            return Task::none();
        };
        let cmd = f(tab);
        let title = tab.title().to_string();
        (cmd, title)
    };

    let command_task = execute_command(cmd);
    let title_task = Task::done(TerminalWorkspaceEvent::Effect(
        TerminalWorkspaceEffect::TitleChanged { tab_id, title },
    ));
    Task::batch(vec![command_task, title_task])
}

fn execute_command(command: StateCommand) -> Task<TerminalWorkspaceEvent> {
    match command {
        StateCommand::None => Task::none(),
        StateCommand::FocusTerminal(id) => TerminalView::focus(id),
        StateCommand::SelectHovered(id) => {
            TerminalView::command(id, BlockCommand::SelectHovered)
        },
        StateCommand::FocusElement(id) => iced::widget::operation::focus(id),
        StateCommand::CloseTab { tab_id } => {
            Task::done(TerminalWorkspaceEvent::Effect(
                TerminalWorkspaceEffect::TabClosed { tab_id },
            ))
        },
        StateCommand::Batch(cmds) => {
            Task::batch(cmds.into_iter().map(execute_command))
        },
    }
}

fn reindex_terminal_tabs(
    state: &TerminalWorkspaceState,
    terminal_to_tab: &mut HashMap<u64, u64>,
) {
    terminal_to_tab.clear();
    for (&tab_id, tab) in state.tabs() {
        for terminal_id in tab.terminals().keys().copied() {
            terminal_to_tab.insert(terminal_id, tab_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use iced::widget::pane_grid;
    use iced::{Point, Size};
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{TerminalWorkspaceCtx, reduce};
    use crate::widgets::terminal_workspace::TerminalWorkspaceIntent;
    use crate::widgets::terminal_workspace::model::TerminalKind;
    use crate::widgets::terminal_workspace::state::TerminalWorkspaceState;

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

    fn default_ctx() -> TerminalWorkspaceCtx {
        TerminalWorkspaceCtx {
            active_tab_id: None,
            pane_grid_size: Size::ZERO,
            screen_size: Size::ZERO,
            sidebar_cursor: Point::ORIGIN,
        }
    }

    #[test]
    fn given_open_tab_command_when_reduced_then_state_stores_tab() {
        let mut state = TerminalWorkspaceState::default();
        let mut terminal_to_tab = HashMap::new();
        let mut next_id = 100_u64;
        let ctx = default_ctx();

        let _task = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::OpenTab {
                tab_id: 1,
                terminal_id: 10,
                default_title: String::from("Shell"),
                settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
                kind: TerminalKind::Shell,
                sync_explorer: true,
            },
            &ctx,
        );

        assert!(state.tab(1).is_some());
        assert_eq!(terminal_to_tab.get(&10), Some(&1));
    }

    #[test]
    fn given_sync_pane_grid_size_when_reduced_then_all_tab_grid_sizes_update() {
        let mut state = TerminalWorkspaceState::default();
        let mut terminal_to_tab = HashMap::new();
        let mut next_id = 100_u64;

        let open_ctx = TerminalWorkspaceCtx {
            active_tab_id: None,
            pane_grid_size: Size::new(120.0, 80.0),
            screen_size: Size::ZERO,
            sidebar_cursor: Point::ORIGIN,
        };

        let _ = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::OpenTab {
                tab_id: 1,
                terminal_id: 10,
                default_title: String::from("Shell"),
                settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
                kind: TerminalKind::Shell,
                sync_explorer: false,
            },
            &open_ctx,
        );

        let sync_ctx = TerminalWorkspaceCtx {
            active_tab_id: None,
            pane_grid_size: Size::new(480.0, 320.0),
            screen_size: Size::ZERO,
            sidebar_cursor: Point::ORIGIN,
        };

        let _ = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::SyncPaneGridSize,
            &sync_ctx,
        );

        let tab = state.tab(1).expect("tab must exist after opening");
        assert_eq!(tab.grid_size(), Size::new(480.0, 320.0));
    }

    #[test]
    fn given_missing_tab_when_pane_clicked_then_reducer_ignores_event() {
        let mut state = TerminalWorkspaceState::default();
        let mut terminal_to_tab = HashMap::new();
        let mut next_id = 0_u64;
        let ctx = default_ctx();

        let (_grid, pane) = pane_grid::State::new(1_u64);
        let _task = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::PaneClicked { tab_id: 999, pane },
            &ctx,
        );

        assert!(state.tab(999).is_none());
    }

    #[test]
    fn given_tab_closed_command_when_reduced_then_tab_is_removed() {
        let mut state = TerminalWorkspaceState::default();
        let mut terminal_to_tab = HashMap::new();
        let mut next_id = 100_u64;
        let ctx = default_ctx();

        let _ = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::OpenTab {
                tab_id: 1,
                terminal_id: 10,
                default_title: String::from("Shell"),
                settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
                kind: TerminalKind::Shell,
                sync_explorer: false,
            },
            &ctx,
        );

        let _task = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::TabClosed { tab_id: 1 },
            &ctx,
        );

        assert!(state.tab(1).is_none());
        assert_eq!(terminal_to_tab.get(&10), None);
    }

    #[test]
    fn given_close_all_context_menus_when_no_menus_open_then_no_error() {
        let mut state = TerminalWorkspaceState::default();
        let mut terminal_to_tab = HashMap::new();
        let mut next_id = 100_u64;
        let ctx = default_ctx();

        let _task = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::CloseAllContextMenus,
            &ctx,
        );
    }

    #[test]
    fn given_context_menu_input_when_reduced_then_no_effect() {
        let mut state = TerminalWorkspaceState::default();
        let mut terminal_to_tab = HashMap::new();
        let mut next_id = 100_u64;
        let ctx = default_ctx();

        let _task = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::ContextMenuInput { tab_id: 1 },
            &ctx,
        );
    }

    #[test]
    fn given_focus_active_with_no_tabs_when_reduced_then_no_error() {
        let mut state = TerminalWorkspaceState::default();
        let mut terminal_to_tab = HashMap::new();
        let mut next_id = 100_u64;
        let ctx = TerminalWorkspaceCtx {
            active_tab_id: Some(999),
            pane_grid_size: Size::ZERO,
            screen_size: Size::ZERO,
            sidebar_cursor: Point::ORIGIN,
        };

        let _task = reduce(
            &mut state,
            &mut terminal_to_tab,
            &mut next_id,
            TerminalWorkspaceIntent::FocusActive,
            &ctx,
        );
    }
}
