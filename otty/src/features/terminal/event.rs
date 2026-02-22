use std::fmt;
use std::path::PathBuf;

use iced::{Point, Task, widget::pane_grid};
use otty_ui_term::{BlockCommand, TerminalView};

use crate::app::Event as AppEvent;
use crate::features::tab::TabContent;
use crate::state::State;

#[cfg(test)]
use super::model::TerminalKind;
use super::state::TerminalState;

/// Events emitted by terminal UI and terminal-related flows.
#[derive(Clone)]
pub(crate) enum TerminalEvent {
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
        Widget(event) => reduce_widget_event(state, event),
        PaneClicked { tab_id, pane } => {
            with_terminal_tab(state, tab_id, |tab| tab.focus_pane(pane))
        },
        PaneResized { tab_id, event } => {
            if let Some(tab) = terminal_tab_mut(state, tab_id) {
                tab.resize(event);
                sync_tab_title(state, tab_id);
            }
            Task::none()
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
            with_terminal_tab(state, tab_id, TerminalState::close_context_menu)
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

    if refresh_titles {
        sync_tab_title(state, tab_id);
    }

    update
}

fn focus_active_terminal(state: &State) -> Task<AppEvent> {
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
        _ => Task::none(),
    }
}

fn apply_terminal_theme(
    state: &mut State,
    palette: otty_ui_term::ColorPalette,
) -> Task<AppEvent> {
    for tab in state.tab_items_mut().values_mut() {
        if let TabContent::Terminal(terminal) = &mut tab.content {
            terminal.apply_theme(palette.clone());
        }
    }

    Task::none()
}

fn close_all_context_menus(state: &mut State) -> Task<AppEvent> {
    let mut tasks = Vec::new();
    for tab in state.tab_items_mut().values_mut() {
        if let TabContent::Terminal(terminal) = &mut tab.content
            && terminal.context_menu().is_some()
        {
            tasks.push(terminal.close_context_menu());
        }
    }

    Task::batch(tasks)
}

fn sync_tab_block_selection(state: &State, tab_id: u64) -> Task<AppEvent> {
    let Some(tab) = state.tab_items().get(&tab_id) else {
        return Task::none();
    };

    let terminal = match &tab.content {
        TabContent::Terminal(terminal) => terminal,
        _ => {
            return Task::none();
        },
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
                    .map(|sel| sel.terminal_id() == terminal_id)
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
            Some(selection) if selection.terminal_id() == terminal_id => {
                selection
            },
            _ => return Task::none(),
        };

        let block_id = selection.block_id().to_string();
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
    state.tab_items_mut().get_mut(&tab_id).and_then(|tab| {
        match &mut tab.content {
            TabContent::Terminal(terminal) => Some(terminal.as_mut()),
            _ => None,
        }
    })
}

fn sync_tab_title(state: &mut State, tab_id: u64) {
    let Some(tab) = state.tab_items_mut().get_mut(&tab_id) else {
        return;
    };

    if let TabContent::Terminal(terminal) = &tab.content {
        tab.title = terminal.title().to_string();
    }
}

fn terminal_tab(state: &State, tab_id: u64) -> Option<&TerminalState> {
    state
        .tab_items()
        .get(&tab_id)
        .and_then(|tab| match &tab.content {
            TabContent::Terminal(terminal) => Some(terminal.as_ref()),
            _ => None,
        })
}

fn tab_id_by_terminal(state: &State, terminal_id: u64) -> Option<u64> {
    state.terminal_tab_id(terminal_id)
}

/// Resolve current working directory from active shell terminal tab.
pub(crate) fn shell_cwd_for_active_tab(state: &State) -> Option<PathBuf> {
    let tab_id = state.active_tab_id()?;
    let terminal = shell_terminal_tab(state, tab_id)?;
    terminal
        .focused_terminal_entry()
        .and_then(|entry| terminal_cwd(&entry.terminal.blocks()))
}

fn shell_terminal_tab(state: &State, tab_id: u64) -> Option<&TerminalState> {
    let terminal = terminal_tab(state, tab_id)?;
    terminal.is_shell().then_some(terminal)
}

fn terminal_cwd(blocks: &[otty_ui_term::BlockSnapshot]) -> Option<PathBuf> {
    blocks
        .iter()
        .rev()
        .find_map(|block| block.meta.cwd.as_deref())
        .map(PathBuf::from)
}

#[derive(Clone, Copy, Debug)]
enum CopyKind {
    Content,
    Prompt,
    Command,
}

#[cfg(test)]
mod tests {
    use std::process::ExitStatus;
    use std::sync::Arc;

    use iced::widget::pane_grid;
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{TerminalEvent, terminal_reducer};
    use crate::features::tab::{TabContent, TabItem};
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

    fn insert_terminal_tab(
        state: &mut State,
        tab_id: u64,
        terminal_id: u64,
        settings: Settings,
    ) {
        let (mut terminal, _task) = super::TerminalState::new(
            tab_id,
            String::from("Shell"),
            terminal_id,
            settings,
            super::TerminalKind::Shell,
        )
        .expect("terminal should initialize");
        terminal.set_grid_size(state.pane_grid_size());
        state.register_terminal_for_tab(terminal_id, tab_id);
        state.tab.insert(
            tab_id,
            TabItem {
                id: tab_id,
                title: terminal.title().to_string(),
                content: TabContent::Terminal(Box::new(terminal)),
            },
        );
        state.tab.activate(Some(tab_id));
    }

    fn try_insert_terminal_tab(
        state: &mut State,
        tab_id: u64,
        terminal_id: u64,
        settings: Settings,
    ) -> bool {
        let Ok((mut terminal, _task)) = super::TerminalState::new(
            tab_id,
            String::from("Shell"),
            terminal_id,
            settings,
            super::TerminalKind::Shell,
        ) else {
            return false;
        };

        terminal.set_grid_size(state.pane_grid_size());
        state.register_terminal_for_tab(terminal_id, tab_id);
        state.tab.insert(
            tab_id,
            TabItem {
                id: tab_id,
                title: terminal.title().to_string(),
                content: TabContent::Terminal(Box::new(terminal)),
            },
        );
        state.tab.activate(Some(tab_id));
        true
    }

    fn terminal_tab(state: &State, tab_id: u64) -> &super::TerminalState {
        let item = state.tab_items().get(&tab_id).expect("tab item");
        match &item.content {
            TabContent::Terminal(terminal) => terminal.as_ref(),
            _ => panic!("expected terminal tab"),
        }
    }

    fn success_exit_status() -> ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            ExitStatus::from_raw(0)
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::ExitStatusExt;
            ExitStatus::from_raw(0)
        }
    }

    #[test]
    fn given_initialized_terminal_when_inserted_then_state_updates() {
        let mut state = State::default();

        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );

        assert_eq!(state.active_tab_id(), Some(1));
        assert_eq!(state.terminal_tab_id(10), Some(1));
        let tab = state.tab_items().get(&1).expect("tab");
        assert_eq!(tab.id, 1);
        assert_eq!(tab.title, "Shell");
        assert!(matches!(tab.content, TabContent::Terminal(_)));
    }

    #[test]
    fn given_terminal_init_failure_when_inserting_then_state_stays_unchanged() {
        let mut state = State::default();

        let inserted = try_insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program("/definitely/missing/otty-shell"),
        );

        assert!(!inserted);
        assert!(state.tab.is_empty());
        assert_eq!(state.terminal_to_tab_len(), 0);
        assert!(state.active_tab_id().is_none());
    }

    #[test]
    fn given_missing_tab_when_pane_clicked_then_reducer_ignores_event() {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );
        let focused_before = terminal_tab(&state, 1).focus();
        let existing_pane = focused_before.expect("focused pane");

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::PaneClicked {
                tab_id: 999,
                pane: existing_pane,
            },
        );

        assert_eq!(terminal_tab(&state, 1).focus(), focused_before);
    }

    #[test]
    fn given_title_change_widget_event_when_dispatched_then_tab_title_is_synced()
     {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::Widget(otty_ui_term::Event::TitleChanged {
                id: 10,
                title: String::from("Renamed"),
            }),
        );

        let tab = state.tab_items().get(&1).expect("tab");
        assert_eq!(tab.title, "Renamed");
        assert_eq!(terminal_tab(&state, 1).title(), "Renamed");
    }

    #[test]
    fn given_unknown_terminal_widget_event_when_dispatched_then_state_is_unchanged()
     {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );
        let tab_before = terminal_tab(&state, 1).title().to_string();

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::Widget(otty_ui_term::Event::TitleChanged {
                id: 777,
                title: String::from("Ignored"),
            }),
        );

        assert_eq!(terminal_tab(&state, 1).title(), tab_before);
    }

    #[test]
    fn given_block_selection_events_when_dispatched_then_selection_state_transitions()
     {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::Widget(otty_ui_term::Event::BlockSelected {
                id: 10,
                block_id: String::from("block-1"),
            }),
        );
        let selection =
            terminal_tab(&state, 1).selected_block().expect("selection");
        assert_eq!(selection.terminal_id(), 10);
        assert_eq!(selection.block_id(), "block-1");

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::Widget(otty_ui_term::Event::BlockSelectionCleared {
                id: 10,
            }),
        );
        assert!(terminal_tab(&state, 1).selected_block().is_none());
    }

    #[test]
    fn given_split_and_close_pane_events_when_dispatched_then_terminal_index_is_reindexed()
     {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );
        state.set_next_terminal_id_for_tests(11);
        let pane = terminal_tab(&state, 1).focus().expect("focused pane");

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::SplitPane {
                tab_id: 1,
                pane,
                axis: pane_grid::Axis::Vertical,
            },
        );

        assert_eq!(state.terminal_tab_id(11), Some(1));
        let closing_pane =
            terminal_tab(&state, 1).focus().expect("split focus");
        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::ClosePane {
                tab_id: 1,
                pane: closing_pane,
            },
        );

        assert!(state.terminal_tab_id(11).is_none());
        assert!(terminal_tab(&state, 1).contains_terminal(10));
    }

    #[test]
    fn given_empty_state_when_focus_active_event_dispatched_then_reducer_is_noop()
     {
        let mut state = State::default();
        let before_tab_count = state.tab.len();

        let _task = terminal_reducer(&mut state, TerminalEvent::FocusActive);

        assert_eq!(state.tab.len(), before_tab_count);
        assert!(state.active_tab_id().is_none());
    }

    #[test]
    fn given_non_terminal_tab_when_sync_selection_then_reducer_ignores_event() {
        let mut state = State::default();
        state.tab.insert(
            77,
            TabItem {
                id: 77,
                title: String::from("Settings"),
                content: TabContent::Settings,
            },
        );

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::SyncSelection { tab_id: 77 },
        );

        assert_eq!(state.tab.len(), 1);
        assert_eq!(
            state.tab_items().get(&77).map(|item| item.title.as_str()),
            Some("Settings"),
        );
    }

    #[test]
    fn given_context_menu_open_close_and_input_events_when_dispatched_then_menu_state_transitions()
     {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );
        let pane = terminal_tab(&state, 1).focus().expect("focused pane");

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::OpenContextMenu {
                tab_id: 1,
                pane,
                terminal_id: 10,
            },
        );
        assert!(terminal_tab(&state, 1).context_menu().is_some());

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::ContextMenuInput { tab_id: 1 },
        );
        assert!(terminal_tab(&state, 1).context_menu().is_some());

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::CloseContextMenu { tab_id: 1 },
        );
        assert!(terminal_tab(&state, 1).context_menu().is_none());
    }

    #[test]
    fn given_copy_and_paste_events_when_dispatched_then_context_menu_is_closed()
    {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );
        let pane = terminal_tab(&state, 1).focus().expect("focused pane");

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::OpenContextMenu {
                tab_id: 1,
                pane,
                terminal_id: 10,
            },
        );
        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::CopySelection {
                tab_id: 1,
                terminal_id: 10,
            },
        );
        assert!(terminal_tab(&state, 1).context_menu().is_none());

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::OpenContextMenu {
                tab_id: 1,
                pane,
                terminal_id: 10,
            },
        );
        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::PasteIntoPrompt {
                tab_id: 1,
                terminal_id: 10,
            },
        );
        assert!(terminal_tab(&state, 1).context_menu().is_none());
    }

    #[test]
    fn given_selected_block_copy_events_when_dispatched_then_menu_is_closed() {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );
        let pane = terminal_tab(&state, 1).focus().expect("focused pane");

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::Widget(otty_ui_term::Event::BlockSelected {
                id: 10,
                block_id: String::from("block-1"),
            }),
        );

        for event in [
            TerminalEvent::CopySelectedBlockContent {
                tab_id: 1,
                terminal_id: 10,
            },
            TerminalEvent::CopySelectedBlockPrompt {
                tab_id: 1,
                terminal_id: 10,
            },
            TerminalEvent::CopySelectedBlockCommand {
                tab_id: 1,
                terminal_id: 10,
            },
        ] {
            let _task = terminal_reducer(
                &mut state,
                TerminalEvent::OpenContextMenu {
                    tab_id: 1,
                    pane,
                    terminal_id: 10,
                },
            );
            let _task = terminal_reducer(&mut state, event);
            assert!(terminal_tab(&state, 1).context_menu().is_none());
        }
    }

    #[test]
    fn given_active_terminal_when_focus_active_event_dispatched_then_state_keeps_active_tab()
     {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );

        let _task = terminal_reducer(&mut state, TerminalEvent::FocusActive);

        assert_eq!(state.active_tab_id(), Some(1));
        assert_eq!(terminal_tab(&state, 1).focused_terminal_id(), Some(10));
    }

    #[test]
    fn given_terminal_tab_when_sync_selection_event_dispatched_then_selection_is_kept()
     {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );
        state.set_next_terminal_id_for_tests(11);
        let pane = terminal_tab(&state, 1).focus().expect("focused pane");
        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::SplitPane {
                tab_id: 1,
                pane,
                axis: pane_grid::Axis::Horizontal,
            },
        );

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::Widget(otty_ui_term::Event::BlockSelected {
                id: 10,
                block_id: String::from("block-2"),
            }),
        );
        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::SyncSelection { tab_id: 1 },
        );

        let selection =
            terminal_tab(&state, 1).selected_block().expect("selection");
        assert_eq!(selection.terminal_id(), 10);
        assert_eq!(selection.block_id(), "block-2");
    }

    #[test]
    fn given_shutdown_widget_event_when_dispatched_then_terminal_is_reindexed()
    {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );
        state.set_next_terminal_id_for_tests(11);
        let pane = terminal_tab(&state, 1).focus().expect("focused pane");
        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::SplitPane {
                tab_id: 1,
                pane,
                axis: pane_grid::Axis::Vertical,
            },
        );
        assert_eq!(state.terminal_tab_id(11), Some(1));

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::Widget(otty_ui_term::Event::Shutdown {
                id: 11,
                exit_status: success_exit_status(),
            }),
        );

        assert!(state.terminal_tab_id(11).is_none());
        assert_eq!(terminal_tab(&state, 1).terminals().len(), 1);
    }

    #[test]
    fn given_content_sync_widget_event_when_dispatched_then_reducer_keeps_tab_mapping()
     {
        let mut state = State::default();
        insert_terminal_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );

        let _task = terminal_reducer(
            &mut state,
            TerminalEvent::Widget(otty_ui_term::Event::ContentSync {
                id: 10,
                frame: Arc::new(otty_libterm::surface::SnapshotOwned::default()),
            }),
        );

        assert_eq!(state.terminal_tab_id(10), Some(1));
    }
}
