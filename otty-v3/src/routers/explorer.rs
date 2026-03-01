use std::path::PathBuf;

use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::explorer::model::FileNode;
use crate::widgets::explorer::services::read_dir_nodes;
use crate::widgets::explorer::{
    ExplorerCommand, ExplorerCtx, ExplorerEffect, ExplorerEvent,
};

/// Route an explorer UI event through the widget reducer.
pub(crate) fn route_event(
    app: &mut App,
    event: ExplorerEvent,
) -> Task<AppEvent> {
    let command = map_event_to_command(event);
    route_command(app, command)
}

/// Route an explorer command directly (used by flow routers).
pub(crate) fn route_command(
    app: &mut App,
    command: ExplorerCommand,
) -> Task<AppEvent> {
    let active_tab_id = app.widgets.tabs.active_tab_id();
    let ctx = ExplorerCtx {
        active_shell_cwd: app
            .widgets
            .terminal_workspace
            .shell_cwd_for_active_tab(active_tab_id),
    };
    app.widgets
        .explorer
        .reduce(command, &ctx)
        .map(AppEvent::ExplorerEffect)
}

/// Route an explorer effect event to app-level tasks.
pub(crate) fn route_effect(effect: ExplorerEffect) -> Task<AppEvent> {
    match effect {
        ExplorerEffect::LoadRootRequested { root } => {
            load_directory_async(root.clone(), move |nodes| {
                AppEvent::ExplorerUi(ExplorerEvent::RootLoaded {
                    root: root.clone(),
                    nodes,
                })
            })
        },
        ExplorerEffect::LoadFolderRequested { path, directory } => {
            load_directory_async(directory, move |nodes| {
                AppEvent::ExplorerUi(ExplorerEvent::FolderLoaded {
                    path: path.clone(),
                    nodes,
                })
            })
        },
        ExplorerEffect::OpenFileTerminalTab { file_path } => {
            Task::done(AppEvent::OpenFileTerminalTab { file_path })
        },
    }
}

/// Asynchronously read a directory and produce a completion event.
fn load_directory_async<F>(dir: PathBuf, on_complete: F) -> Task<AppEvent>
where
    F: Fn(Vec<FileNode>) -> AppEvent + Send + 'static,
{
    Task::perform(async move { read_dir_nodes(&dir) }, move |result| {
        match result {
            Ok(nodes) => on_complete(nodes),
            Err(err) => AppEvent::ExplorerUi(ExplorerEvent::LoadFailed {
                message: format!("{err}"),
            }),
        }
    })
}

fn map_event_to_command(event: ExplorerEvent) -> ExplorerCommand {
    use {ExplorerCommand as C, ExplorerEvent as E};

    match event {
        E::NodePressed { path } => C::NodePressed { path },
        E::NodeHovered { path } => C::NodeHovered { path },
        E::SyncRoot { cwd } => C::SyncRoot { cwd },
        E::RootLoaded { root, nodes } => C::RootLoaded { root, nodes },
        E::FolderLoaded { path, nodes } => C::FolderLoaded { path, nodes },
        E::LoadFailed { message } => C::LoadFailed { message },
    }
}
