use iced::{Point, Task, widget::pane_grid};
use otty_ui_term::settings::{SessionKind, Settings};
use otty_ui_term::{BlockCommand, TerminalView};

use crate::app::Event as AppEvent;
use crate::features::explorer;
use crate::features::tab::{TabContent, TabItem};
use crate::state::State;

use super::shell::ShellSession;
use super::term::{TerminalKind, TerminalState};

/// Events emitted by the terminal tab view and routed into the terminal reducer.
#[derive(Debug, Clone)]
pub(crate) enum TerminalEvent {
    ProxyToInternalWidget(otty_ui_term::Event),
    PaneClicked {
        pane: pane_grid::Pane,
    },
    PaneResized {
        event: pane_grid::ResizeEvent,
    },
    PaneGridCursorMoved {
        position: Point,
    },
    OpenContextMenu {
        pane: pane_grid::Pane,
        terminal_id: u64,
    },
    CloseContextMenu,
    ContextMenuInput,
    SplitPane {
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
    },
    ClosePane {
        pane: pane_grid::Pane,
    },
    CopySelection {
        terminal_id: u64,
    },
    PasteIntoPrompt {
        terminal_id: u64,
    },
    CopySelectedBlockContent {
        terminal_id: u64,
    },
    CopySelectedBlockPrompt {
        terminal_id: u64,
    },
    CopySelectedBlockCommand {
        terminal_id: u64,
    },
}

pub(crate) fn terminal_tab_reducer(
    state: &mut State,
    tab_id: u64,
    event: TerminalEvent,
) -> Task<AppEvent> {
    use TerminalEvent::*;

    match event {
        ProxyToInternalWidget(inner) => {
            internal_widget_event_reducer(state, inner)
        },
        OpenContextMenu { pane, terminal_id } => {
            let cursor = state.sidebar.cursor;
            let grid_size = state.screen_size;
            with_terminal_tab(state, tab_id, |tab| {
                tab.open_context_menu(pane, terminal_id, cursor, grid_size)
            })
        },
        CloseContextMenu => {
            with_terminal_tab(state, tab_id, TerminalState::close_context_menu)
        },
        ContextMenuInput => Task::none(),
        CopySelectedBlockContent { terminal_id } => {
            copy_selected_block(state, tab_id, terminal_id, CopyKind::Content)
        },
        CopySelectedBlockPrompt { terminal_id } => {
            copy_selected_block(state, tab_id, terminal_id, CopyKind::Prompt)
        },
        CopySelectedBlockCommand { terminal_id } => {
            copy_selected_block(state, tab_id, terminal_id, CopyKind::Command)
        },
        CopySelection { terminal_id } => {
            copy_selection(state, tab_id, terminal_id)
        },
        PasteIntoPrompt { terminal_id } => {
            paste_into_prompt(state, tab_id, terminal_id)
        },
        SplitPane { pane, axis } => {
            let task = split_pane(state, tab_id, pane, axis);
            explorer::event::sync_explorer_from_active_terminal(state);
            task
        },
        ClosePane { pane } => {
            let task = close_pane(state, tab_id, pane);
            explorer::event::sync_explorer_from_active_terminal(state);
            task
        },
        PaneClicked { pane } => {
            let task =
                with_terminal_tab(state, tab_id, |tab| tab.focus_pane(pane));
            explorer::event::sync_explorer_from_active_terminal(state);
            task
        },
        PaneResized { event } => {
            if let Some(tab) = terminal_tab_mut(state, tab_id) {
                tab.resize(event);
                sync_tab_title(state, tab_id);
            }
            Task::none()
        },
        PaneGridCursorMoved { position } => {
            state.sidebar.cursor = position;
            with_terminal_tab(state, tab_id, |tab| {
                tab.update_grid_cursor(position)
            })
        },
    }
}

pub(crate) fn internal_widget_event_reducer(
    state: &mut State,
    event: otty_ui_term::Event,
) -> Task<AppEvent> {
    let terminal_id = *event.terminal_id();
    let Some(tab_id) = tab_id_by_terminal(state, terminal_id) else {
        return Task::none();
    };

    let refresh_titles = matches!(
        &event,
        otty_ui_term::Event::TitleChanged { .. }
            | otty_ui_term::Event::ResetTitle { .. }
    );

    let is_content_sync =
        matches!(event, otty_ui_term::Event::ContentSync { .. });
    let is_shutdown = matches!(&event, otty_ui_term::Event::Shutdown { .. });
    let selection_task = update_block_selection(state, tab_id, &event);
    let event_task = with_terminal_tab(state, tab_id, |tab| {
        tab.handle_terminal_event(event)
    });
    let update = Task::batch(vec![selection_task, event_task]);

    if is_shutdown {
        state.reindex_terminal_tabs();
    }

    if refresh_titles {
        sync_tab_title(state, tab_id);
    }

    if is_content_sync {
        explorer::event::sync_explorer_from_terminal_event(
            state,
            tab_id,
            terminal_id,
        );
    }

    update
}

pub(crate) fn create_terminal_tab(
    state: &mut State,
    terminal_settings: &Settings,
    shell_session: &ShellSession,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    let terminal_id = state.next_terminal_id;
    state.next_terminal_id += 1;

    let settings =
        settings_for_session(terminal_settings, shell_session.session.clone());
    let (tab, focus_task) = match TerminalState::new(
        tab_id,
        shell_session.name.clone(),
        terminal_id,
        settings,
        TerminalKind::Shell,
    ) {
        Ok(result) => result,
        Err(err) => {
            log::warn!("failed to create terminal tab: {err}");
            return Task::none();
        },
    };

    insert_terminal_tab(state, tab_id, tab, focus_task, true)
}

pub(crate) fn insert_terminal_tab(
    state: &mut State,
    tab_id: u64,
    mut tab: TerminalState,
    focus_task: Task<AppEvent>,
    sync_explorer: bool,
) -> Task<AppEvent> {
    tab.set_grid_size(state.pane_grid_size());
    for terminal_id in tab.terminals().keys().copied() {
        state.register_terminal_for_tab(terminal_id, tab_id);
    }

    let title = tab.title().to_string();
    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title,
            content: TabContent::Terminal(Box::new(tab)),
        },
    );
    state.active_tab_id = Some(tab_id);

    let sync_task = sync_tab_block_selection(state, tab_id);
    if sync_explorer {
        explorer::event::sync_explorer_from_active_terminal(state);
    }

    Task::batch(vec![focus_task, sync_task])
}

pub(crate) fn focus_active_terminal(state: &State) -> Task<AppEvent> {
    let Some(tab) = state.active_tab() else {
        return Task::none();
    };

    match &tab.content {
        TabContent::Terminal(terminal) => {
            match terminal.focused_terminal_entry() {
                Some(entry) => {
                    TerminalView::focus(entry.terminal.widget_id().clone())
                },
                None => Task::none(),
            }
        },
        TabContent::Settings
        | TabContent::QuickCommandEditor(_)
        | TabContent::QuickCommandError(_) => Task::none(),
    }
}

pub(crate) fn sync_tab_block_selection(
    state: &State,
    tab_id: u64,
) -> Task<AppEvent> {
    let Some(tab) = state.tab_items.get(&tab_id) else {
        return Task::none();
    };

    let terminal = match &tab.content {
        TabContent::Terminal(terminal) => terminal,
        TabContent::Settings
        | TabContent::QuickCommandEditor(_)
        | TabContent::QuickCommandError(_) => {
            return Task::none();
        },
    };
    let selection = terminal.selected_block().cloned();
    let mut tasks = Vec::new();

    for entry in terminal.terminals().values() {
        let cmd = if let Some(sel) = &selection
            && sel.terminal_id == entry.terminal.id
        {
            BlockCommand::Select(sel.block_id.clone())
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
    state: &mut State,
    tab_id: u64,
    pane: pane_grid::Pane,
    axis: pane_grid::Axis,
) -> Task<AppEvent> {
    let terminal_id = state.next_terminal_id;
    state.next_terminal_id += 1;

    let task = with_terminal_tab(state, tab_id, move |tab| {
        tab.split_pane(pane, axis, terminal_id)
    });

    if terminal_tab(state, tab_id)
        .map(|tab| tab.contains_terminal(terminal_id))
        .unwrap_or(false)
    {
        state.register_terminal_for_tab(terminal_id, tab_id);
    }

    task
}

fn close_pane(
    state: &mut State,
    tab_id: u64,
    pane: pane_grid::Pane,
) -> Task<AppEvent> {
    let task = with_terminal_tab(state, tab_id, |tab| tab.close_pane(pane));
    state.reindex_terminal_tabs();
    task
}

fn update_block_selection(
    state: &mut State,
    tab_id: u64,
    event: &otty_ui_term::Event,
) -> Task<AppEvent> {
    use otty_ui_term::Event::*;

    match event {
        BlockSelected { block_id, .. } => {
            let terminal_id = *event.terminal_id();
            with_terminal_tab(state, tab_id, |tab| {
                tab.set_selected_block(terminal_id, block_id.clone());

                let mut tasks = Vec::new();
                for entry in tab.terminals().values() {
                    if entry.terminal.id != terminal_id {
                        tasks.push(TerminalView::command(
                            entry.terminal.widget_id().clone(),
                            BlockCommand::ClearSelection,
                        ));
                    }
                }

                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            })
        },
        BlockSelectionCleared { .. } => {
            let terminal_id = *event.terminal_id();
            with_terminal_tab(state, tab_id, |tab| {
                if tab
                    .selected_block()
                    .map(|sel| sel.terminal_id == terminal_id)
                    .unwrap_or(false)
                {
                    tab.clear_selected_block();
                }
                Task::none()
            })
        },
        BlockCopied { .. } => Task::none(),
        _ => Task::none(),
    }
}

fn copy_selected_block(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
    kind: CopyKind,
) -> Task<AppEvent> {
    with_terminal_tab(state, tab_id, |tab| {
        let selection = match tab.selected_block() {
            Some(selection) if selection.terminal_id == terminal_id => {
                selection
            },
            _ => return Task::none(),
        };

        let block_id = selection.block_id.clone();
        let widget_id = match tab.terminals().get(&terminal_id) {
            Some(entry) => entry.terminal.widget_id().clone(),
            None => return Task::none(),
        };

        let command = match kind {
            CopyKind::Content => BlockCommand::CopyContent(block_id),
            CopyKind::Prompt => BlockCommand::CopyPrompt(block_id),
            CopyKind::Command => BlockCommand::CopyCommand(block_id),
        };

        let close_menu_task = tab.close_context_menu();
        let copy_task = TerminalView::command(widget_id, command);

        Task::batch(vec![close_menu_task, copy_task])
    })
}

fn copy_selection(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
) -> Task<AppEvent> {
    with_terminal_tab(state, tab_id, |tab| {
        let widget_id = match tab.terminals().get(&terminal_id) {
            Some(entry) => entry.terminal.widget_id().clone(),
            None => return Task::none(),
        };

        let close_menu_task = tab.close_context_menu();
        let copy_task =
            TerminalView::command(widget_id, BlockCommand::CopySelection);

        Task::batch(vec![close_menu_task, copy_task])
    })
}

fn paste_into_prompt(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
) -> Task<AppEvent> {
    with_terminal_tab(state, tab_id, |tab| {
        let widget_id = match tab.terminals().get(&terminal_id) {
            Some(entry) => entry.terminal.widget_id().clone(),
            None => return Task::none(),
        };

        let close_menu_task = tab.close_context_menu();
        let paste_task =
            TerminalView::command(widget_id, BlockCommand::PasteClipboard);

        Task::batch(vec![close_menu_task, paste_task])
    })
}

fn with_terminal_tab<F>(state: &mut State, tab_id: u64, f: F) -> Task<AppEvent>
where
    F: FnOnce(&mut TerminalState) -> Task<AppEvent>,
{
    let Some(tab) = terminal_tab_mut(state, tab_id) else {
        return Task::none();
    };

    let task = f(tab);
    sync_tab_title(state, tab_id);
    task
}

fn terminal_tab_mut(
    state: &mut State,
    tab_id: u64,
) -> Option<&mut TerminalState> {
    state
        .tab_items
        .get_mut(&tab_id)
        .and_then(|tab| match &mut tab.content {
            TabContent::Terminal(terminal) => Some(terminal.as_mut()),
            TabContent::Settings
            | TabContent::QuickCommandEditor(_)
            | TabContent::QuickCommandError(_) => None,
        })
}

fn sync_tab_title(state: &mut State, tab_id: u64) {
    let Some(tab) = state.tab_items.get_mut(&tab_id) else {
        return;
    };

    if let TabContent::Terminal(terminal) = &tab.content {
        tab.title = terminal.title().to_string();
    }
}

fn terminal_tab(state: &State, tab_id: u64) -> Option<&TerminalState> {
    state
        .tab_items
        .get(&tab_id)
        .and_then(|tab| match &tab.content {
            TabContent::Terminal(terminal) => Some(terminal.as_ref()),
            TabContent::Settings
            | TabContent::QuickCommandEditor(_)
            | TabContent::QuickCommandError(_) => None,
        })
}

fn tab_id_by_terminal(state: &State, terminal_id: u64) -> Option<u64> {
    state.terminal_tab_id(terminal_id)
}

#[derive(Clone, Copy, Debug)]
enum CopyKind {
    Content,
    Prompt,
    Command,
}

pub(crate) fn settings_for_session(
    base_settings: &Settings,
    session: SessionKind,
) -> Settings {
    let mut settings = base_settings.clone();
    settings.backend = settings.backend.clone().with_session(session);
    settings
}
