use std::fmt;

use iced::widget::pane_grid;
use iced::{Point, Task};
use otty_ui_term::settings::Settings;
use otty_ui_term::{BlockCommand, TerminalView};

use super::model::TerminalKind;
use super::state::{TerminalCommand, TerminalTabState};
use crate::app::Event as AppEvent;
use crate::features::explorer::ExplorerEvent;
use crate::features::tab::{TabEvent, TabOpenRequest};
use crate::state::State;

/// Events emitted by terminal UI and terminal-related flows.
#[derive(Clone)]
pub(crate) enum TerminalEvent {
    OpenTab {
        tab_id: u64,
        terminal_id: u64,
        default_title: String,
        settings: Box<Settings>,
        kind: TerminalKind,
        sync_explorer: bool,
        error_tab: Option<(String, String)>,
    },
    TabClosed {
        tab_id: u64,
    },
    Widget(otty_ui_term::Event),
    PaneClicked {
        tab_id: u64,
        pane: pane_grid::Pane,
    },
    PaneResized {
        tab_id: u64,
        event: pane_grid::ResizeEvent,
    },
    PaneGridCursorMoved {
        tab_id: u64,
        position: Point,
    },
    OpenContextMenu {
        tab_id: u64,
        pane: pane_grid::Pane,
        terminal_id: u64,
    },
    CloseContextMenu {
        tab_id: u64,
    },
    ContextMenuInput {
        tab_id: u64,
    },
    SplitPane {
        tab_id: u64,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
    },
    ClosePane {
        tab_id: u64,
        pane: pane_grid::Pane,
    },
    CopySelection {
        tab_id: u64,
        terminal_id: u64,
    },
    PasteIntoPrompt {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockContent {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockPrompt {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockCommand {
        tab_id: u64,
        terminal_id: u64,
    },
    ApplyTheme {
        palette: Box<otty_ui_term::ColorPalette>,
    },
    CloseAllContextMenus,
    FocusActive,
    SyncSelection {
        tab_id: u64,
    },
}

impl fmt::Debug for TerminalEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TerminalEvent::OpenTab {
                tab_id,
                terminal_id,
                default_title,
                kind,
                sync_explorer,
                ..
            } => f
                .debug_struct("OpenTab")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .field("default_title", default_title)
                .field("kind", kind)
                .field("sync_explorer", sync_explorer)
                .finish(),
            TerminalEvent::TabClosed { tab_id } => {
                f.debug_struct("TabClosed").field("tab_id", tab_id).finish()
            },
            TerminalEvent::Widget(event) => {
                f.debug_tuple("Widget").field(event).finish()
            },
            TerminalEvent::PaneClicked { tab_id, pane } => f
                .debug_struct("PaneClicked")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .finish(),
            TerminalEvent::PaneResized { tab_id, event } => f
                .debug_struct("PaneResized")
                .field("tab_id", tab_id)
                .field("event", event)
                .finish(),
            TerminalEvent::PaneGridCursorMoved { tab_id, position } => f
                .debug_struct("PaneGridCursorMoved")
                .field("tab_id", tab_id)
                .field("position", position)
                .finish(),
            TerminalEvent::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            } => f
                .debug_struct("OpenContextMenu")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::CloseContextMenu { tab_id } => f
                .debug_struct("CloseContextMenu")
                .field("tab_id", tab_id)
                .finish(),
            TerminalEvent::ContextMenuInput { tab_id } => f
                .debug_struct("ContextMenuInput")
                .field("tab_id", tab_id)
                .finish(),
            TerminalEvent::SplitPane { tab_id, pane, axis } => f
                .debug_struct("SplitPane")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .field("axis", axis)
                .finish(),
            TerminalEvent::ClosePane { tab_id, pane } => f
                .debug_struct("ClosePane")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .finish(),
            TerminalEvent::CopySelection {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelection")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::PasteIntoPrompt {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("PasteIntoPrompt")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::CopySelectedBlockContent {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockContent")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::CopySelectedBlockPrompt {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockPrompt")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::CopySelectedBlockCommand {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockCommand")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::ApplyTheme { .. } => f.write_str("ApplyTheme"),
            TerminalEvent::CloseAllContextMenus => {
                f.write_str("CloseAllContextMenus")
            },
            TerminalEvent::FocusActive => f.write_str("FocusActive"),
            TerminalEvent::SyncSelection { tab_id } => f
                .debug_struct("SyncSelection")
                .field("tab_id", tab_id)
                .finish(),
        }
    }
}

pub(crate) fn terminal_reducer(
    state: &mut State,
    event: TerminalEvent,
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
        } => open_tab(
            state,
            tab_id,
            terminal_id,
            default_title,
            *settings,
            kind,
            sync_explorer,
            error_tab,
        ),
        TabClosed { tab_id } => {
            let _ = state.terminal.remove_tab(tab_id);
            state.remove_tab_terminals(tab_id);
            Task::none()
        },
        Widget(event) => reduce_widget_event(state, event),
        PaneClicked { tab_id, pane } => {
            with_terminal_tab(state, tab_id, |tab| tab.focus_pane(pane))
        },
        PaneResized { tab_id, event } => {
            with_terminal_tab(state, tab_id, |tab| {
                tab.resize(event);
                TerminalCommand::None
            })
        },
        PaneGridCursorMoved { tab_id, position } => {
            state.sidebar.cursor = position;
            with_terminal_tab(state, tab_id, |tab| {
                tab.update_grid_cursor(position)
            })
        },
        OpenContextMenu {
            tab_id,
            pane,
            terminal_id,
        } => {
            let cursor = state.sidebar.cursor;
            let grid_size = state.screen_size;
            with_terminal_tab(state, tab_id, |tab| {
                tab.open_context_menu(pane, terminal_id, cursor, grid_size)
            })
        },
        CloseContextMenu { tab_id } => {
            with_terminal_tab(state, tab_id, |tab| tab.close_context_menu())
        },
        ContextMenuInput { tab_id: _ } => Task::none(),
        SplitPane { tab_id, pane, axis } => {
            split_pane(state, tab_id, pane, axis)
        },
        ClosePane { tab_id, pane } => close_pane(state, tab_id, pane),
        CopySelection {
            tab_id,
            terminal_id,
        } => copy_selection(state, tab_id, terminal_id),
        PasteIntoPrompt {
            tab_id,
            terminal_id,
        } => paste_into_prompt(state, tab_id, terminal_id),
        CopySelectedBlockContent {
            tab_id,
            terminal_id,
        } => copy_selected_block(state, tab_id, terminal_id, CopyKind::Content),
        CopySelectedBlockPrompt {
            tab_id,
            terminal_id,
        } => copy_selected_block(state, tab_id, terminal_id, CopyKind::Prompt),
        CopySelectedBlockCommand {
            tab_id,
            terminal_id,
        } => copy_selected_block(state, tab_id, terminal_id, CopyKind::Command),
        ApplyTheme { palette } => apply_terminal_theme(state, *palette),
        CloseAllContextMenus => close_all_context_menus(state),
        FocusActive => focus_active_terminal(state),
        SyncSelection { tab_id } => sync_tab_block_selection(state, tab_id),
    }
}

#[allow(clippy::too_many_arguments)]
fn open_tab(
    state: &mut State,
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
                return Task::done(AppEvent::Tab(TabEvent::NewTab {
                    request: TabOpenRequest::QuickLaunchError {
                        title,
                        message: format!("{message}\nError: {err}"),
                    },
                }));
            }
            return Task::done(AppEvent::CloseTabRequested { tab_id });
        },
    };

    terminal.set_grid_size(state.pane_grid_size());
    for terminal_id in terminal.terminals().keys().copied() {
        state.register_terminal_for_tab(terminal_id, tab_id);
    }

    let title = terminal.title().to_string();
    state.terminal.insert_tab(tab_id, terminal);

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

    let is_shutdown = matches!(&event, otty_ui_term::Event::Shutdown { .. });
    let selection_task = update_block_selection(state, tab_id, &event);
    let event_task = with_terminal_tab(state, tab_id, |tab| {
        tab.handle_terminal_event(event)
    });
    let update = Task::batch(vec![selection_task, event_task]);

    if is_shutdown {
        state.reindex_terminal_tabs();
    }

    if !refresh_titles {
        return update;
    }

    let title_task = state
        .terminal
        .tab(tab_id)
        .map(|tab| request_tab_title(tab_id, tab.title().to_string()))
        .unwrap_or_else(Task::none);
    Task::batch(vec![update, title_task])
}

fn focus_active_terminal(state: &State) -> Task<AppEvent> {
    let Some(terminal) = state.active_terminal_tab() else {
        return Task::none();
    };

    match terminal.focused_terminal_entry() {
        Some(entry) => TerminalView::focus(entry.terminal.widget_id().clone()),
        None => Task::none(),
    }
}

fn apply_terminal_theme(
    state: &mut State,
    palette: otty_ui_term::ColorPalette,
) -> Task<AppEvent> {
    for (_, terminal) in state.terminal.tabs_mut() {
        terminal.apply_theme(palette.clone());
    }

    Task::none()
}

fn close_all_context_menus(state: &mut State) -> Task<AppEvent> {
    let mut commands = Vec::new();
    for (_, terminal) in state.terminal.tabs_mut() {
        if terminal.context_menu().is_some() {
            commands.push(terminal.close_context_menu());
        }
    }

    Task::batch(commands.into_iter().map(execute_command))
}

fn sync_tab_block_selection(state: &State, tab_id: u64) -> Task<AppEvent> {
    let Some(terminal) = state.terminal.tab(tab_id) else {
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
    state: &mut State,
    tab_id: u64,
    pane: pane_grid::Pane,
    axis: pane_grid::Axis,
) -> Task<AppEvent> {
    let terminal_id = state.allocate_terminal_id();

    let task = with_terminal_tab(state, tab_id, move |tab| {
        tab.split_pane(pane, axis, terminal_id)
    });

    if state
        .terminal
        .tab(tab_id)
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
            let other_widget_ids: Vec<_> = state
                .terminal
                .tab(tab_id)
                .map(|tab| {
                    tab.terminals()
                        .values()
                        .filter(|entry| entry.terminal.id != terminal_id)
                        .map(|entry| entry.terminal.widget_id().clone())
                        .collect()
                })
                .unwrap_or_default();

            let _ = with_terminal_tab(state, tab_id, |tab| {
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
            with_terminal_tab(state, tab_id, |tab| {
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
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
    kind: CopyKind,
) -> Task<AppEvent> {
    let Some((block_id, widget_id)) =
        state.terminal.tab(tab_id).and_then(|tab| {
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
        with_terminal_tab(state, tab_id, |tab| tab.close_context_menu());
    let copy_task = TerminalView::command(widget_id, command);
    Task::batch(vec![close_cmd, copy_task])
}

fn copy_selection(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
) -> Task<AppEvent> {
    let Some(widget_id) = state
        .terminal
        .tab(tab_id)
        .and_then(|tab| tab.terminals().get(&terminal_id))
        .map(|entry| entry.terminal.widget_id().clone())
    else {
        return Task::none();
    };

    let close_cmd =
        with_terminal_tab(state, tab_id, |tab| tab.close_context_menu());
    let copy_task =
        TerminalView::command(widget_id, BlockCommand::CopySelection);
    Task::batch(vec![close_cmd, copy_task])
}

fn paste_into_prompt(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
) -> Task<AppEvent> {
    let Some(widget_id) = state
        .terminal
        .tab(tab_id)
        .and_then(|tab| tab.terminals().get(&terminal_id))
        .map(|entry| entry.terminal.widget_id().clone())
    else {
        return Task::none();
    };

    let close_cmd =
        with_terminal_tab(state, tab_id, |tab| tab.close_context_menu());
    let paste_task =
        TerminalView::command(widget_id, BlockCommand::PasteClipboard);
    Task::batch(vec![close_cmd, paste_task])
}

fn with_terminal_tab<F>(state: &mut State, tab_id: u64, f: F) -> Task<AppEvent>
where
    F: FnOnce(&mut TerminalTabState) -> TerminalCommand,
{
    let (cmd, title) = {
        let Some(tab) = state.terminal.tab_mut(tab_id) else {
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
    Task::done(AppEvent::Explorer(ExplorerEvent::SyncFromActiveTerminal))
}

fn request_terminal_event(event: TerminalEvent) -> Task<AppEvent> {
    Task::done(AppEvent::Terminal(event))
}

fn request_tab_title(tab_id: u64, title: String) -> Task<AppEvent> {
    Task::done(AppEvent::Tab(TabEvent::SetTitle { tab_id, title }))
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

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid;
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{TerminalEvent, terminal_reducer};
    use crate::features::tab::{TabContent, TabItem};
    use crate::features::terminal::TerminalKind;
    use crate::state::State;

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

    fn insert_tab_metadata(state: &mut State, tab_id: u64, title: &str) {
        state.tab.insert(
            tab_id,
            TabItem::new(tab_id, String::from(title), TabContent::Terminal),
        );
        state.tab.activate(Some(tab_id));
    }

    #[test]
    fn given_open_tab_event_when_reduced_then_terminal_feature_stores_tab() {
        let mut state = State::default();
        insert_tab_metadata(&mut state, 1, "Shell");

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::OpenTab {
                tab_id: 1,
                terminal_id: 10,
                default_title: String::from("Shell"),
                settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
                kind: TerminalKind::Shell,
                sync_explorer: true,
                error_tab: None,
            },
        );

        assert!(state.terminal.tab(1).is_some());
        assert_eq!(state.terminal_tab_id(10), Some(1));
    }

    #[test]
    fn given_missing_tab_when_pane_clicked_then_reducer_ignores_event() {
        let mut state = State::default();

        let (_grid, pane) = pane_grid::State::new(1_u64);
        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::PaneClicked { tab_id: 999, pane },
        );

        assert!(state.terminal.tab(999).is_none());
    }

    #[test]
    fn given_tab_closed_event_when_reduced_then_terminal_tab_is_removed() {
        let mut state = State::default();
        insert_tab_metadata(&mut state, 1, "Shell");
        let _ = terminal_reducer(
            &mut state,
            TerminalEvent::OpenTab {
                tab_id: 1,
                terminal_id: 10,
                default_title: String::from("Shell"),
                settings: Box::new(settings_with_program(VALID_SHELL_PATH)),
                kind: TerminalKind::Shell,
                sync_explorer: false,
                error_tab: None,
            },
        );

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::TabClosed { tab_id: 1 },
        );

        assert!(state.terminal.tab(1).is_none());
        assert_eq!(state.terminal_tab_id(10), None);
    }
}
