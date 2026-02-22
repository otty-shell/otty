use std::path::PathBuf;

use iced::Task;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};
use otty_ui_tree::TreePath;

use crate::app::Event as AppEvent;
use crate::features::tab::{TabContent, TabEvent, TabOpenRequest};
use crate::features::terminal::event::settings_for_session;
use crate::features::terminal::state::TerminalState;
use crate::state::State;

use super::errors::EditorCommandParseError;

/// Events emitted by the explorer tree UI.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerEvent {
    NodePressed { path: TreePath },
    NodeHovered { path: Option<TreePath> },
}

/// Handle explorer UI events and trigger side effects.
pub(crate) fn explorer_reducer(
    state: &mut State,
    terminal_settings: &Settings,
    event: ExplorerEvent,
) -> Task<AppEvent> {
    match event {
        ExplorerEvent::NodePressed { path } => {
            state.explorer.selected = Some(path.clone());

            if state.explorer.node_is_folder(&path).unwrap_or(false) {
                state.explorer.toggle_folder(&path);
                return Task::none();
            }

            let Some(file_path) = state.explorer.node_path(&path) else {
                return Task::none();
            };

            open_file_in_editor(state, terminal_settings, file_path)
        },
        ExplorerEvent::NodeHovered { path } => {
            state.explorer.hovered = path;
            Task::none()
        },
    }
}

/// Sync explorer state from the currently active shell tab, if any.
pub(crate) fn sync_explorer_from_active_terminal(state: &mut State) {
    let Some(tab_id) = state.active_tab_id else {
        return;
    };

    if let Some(cwd) =
        terminal_cwd_for_sync(state, tab_id, ExplorerSyncTarget::FocusedPane)
    {
        state.explorer.set_root(cwd);
    }
}

/// Sync explorer state from a terminal event when it targets the focused pane.
pub(crate) fn sync_explorer_from_terminal_event(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
) {
    if state.active_tab_id != Some(tab_id) {
        return;
    }

    if let Some(cwd) = terminal_cwd_for_sync(
        state,
        tab_id,
        ExplorerSyncTarget::FocusedTerminal(terminal_id),
    ) {
        state.explorer.set_root(cwd);
    }
}

fn open_file_in_editor(
    state: &mut State,
    terminal_settings: &Settings,
    file_path: PathBuf,
) -> Task<AppEvent> {
    let editor_raw = state.settings.draft().terminal_editor().trim();
    let (program, mut args) = match parse_command_line(editor_raw) {
        Ok(parsed) => parsed,
        Err(err) => {
            log::warn!("default editor parse failed: {err}");
            return Task::none();
        },
    };

    let file_arg = file_path.to_string_lossy().into_owned();
    args.push(file_arg);

    let mut options = LocalSessionOptions::default()
        .with_program(&program)
        .with_args(args);

    if let Some(parent) = file_path.parent() {
        options = options.with_working_directory(parent.into());
    }

    let session = SessionKind::from_local_options(options);
    let settings = settings_for_session(terminal_settings, session);

    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;
    let terminal_id = state.next_terminal_id;
    state.next_terminal_id += 1;

    let file_display = file_path.display();
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{file_display}"));

    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::CommandTerminal {
            tab_id,
            terminal_id,
            title,
            settings: Box::new(settings),
        },
    }))
}

#[derive(Debug, Clone, Copy)]
enum ExplorerSyncTarget {
    FocusedPane,
    FocusedTerminal(u64),
}

fn parse_command_line(
    input: &str,
) -> Result<(String, Vec<String>), EditorCommandParseError> {
    let parts = shell_words::split(input)?;
    let Some((program, args)) = parts.split_first() else {
        return Err(EditorCommandParseError::Empty);
    };

    Ok((program.clone(), args.to_vec()))
}

fn terminal_cwd_for_sync(
    state: &State,
    tab_id: u64,
    target: ExplorerSyncTarget,
) -> Option<PathBuf> {
    let terminal = shell_terminal_tab(state, tab_id)?;

    match target {
        ExplorerSyncTarget::FocusedPane => terminal
            .focused_terminal_entry()
            .and_then(|entry| terminal_cwd(&entry.terminal.blocks())),
        ExplorerSyncTarget::FocusedTerminal(terminal_id) => {
            if terminal.focused_terminal_id() != Some(terminal_id) {
                return None;
            }

            terminal
                .terminals()
                .get(&terminal_id)
                .and_then(|entry| terminal_cwd(&entry.terminal.blocks()))
        },
    }
}

fn shell_terminal_tab(state: &State, tab_id: u64) -> Option<&TerminalState> {
    let terminal = terminal_tab(state, tab_id)?;
    terminal.is_shell().then_some(terminal)
}

fn terminal_tab(state: &State, tab_id: u64) -> Option<&TerminalState> {
    state
        .tab_items
        .get(&tab_id)
        .and_then(|tab| match &tab.content {
            TabContent::Terminal(terminal) => Some(terminal.as_ref()),
            _ => None,
        })
}

fn terminal_cwd(blocks: &[otty_ui_term::BlockSnapshot]) -> Option<PathBuf> {
    blocks
        .iter()
        .rev()
        .find_map(|block| block.meta.cwd.as_deref())
        .map(PathBuf::from)
}
