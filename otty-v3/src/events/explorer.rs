use std::path::PathBuf;

use iced::Task;

use crate::app::App;
use crate::widgets::explorer::model::FileNode;
use crate::widgets::explorer::services::read_dir_nodes;
use crate::widgets::explorer::{
    ExplorerCtx, ExplorerEffect, ExplorerEvent, ExplorerUiEvent,
};
use super::AppEvent;

pub(crate) fn handle(app: &mut App, event: ExplorerEvent) -> Task<AppEvent> {
    match event {
        ExplorerEvent::Ui(event) => handle_ui_event(app, event),
        ExplorerEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_ui_event(app: &mut App, event: ExplorerUiEvent) -> Task<AppEvent> {
    let active_tab_id = app.widgets.tabs.active_tab_id();
    let ctx = ExplorerCtx {
        active_shell_cwd: app
            .widgets
            .terminal_workspace
            .shell_cwd_for_active_tab(active_tab_id),
    };
    app.widgets
        .explorer
        .reduce(event, &ctx)
        .map(AppEvent::Explorer)
}

fn handle_effect(effect: ExplorerEffect) -> Task<AppEvent> {
    match effect {
        ExplorerEffect::LoadRootRequested { root } => {
            load_directory_async(root.clone(), move |nodes| {
                ExplorerUiEvent::RootLoaded {
                    root: root.clone(),
                    nodes,
                }
            })
        },
        ExplorerEffect::LoadFolderRequested { path, directory } => {
            load_directory_async(directory, move |nodes| {
                ExplorerUiEvent::FolderLoaded {
                    path: path.clone(),
                    nodes,
                }
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
    F: Fn(Vec<FileNode>) -> ExplorerUiEvent + Send + 'static,
{
    Task::perform(async move { read_dir_nodes(&dir) }, move |result| {
        match result {
            Ok(nodes) => {
                AppEvent::Explorer(ExplorerEvent::Ui(on_complete(nodes)))
            },
            Err(err) => AppEvent::Explorer(ExplorerEvent::Ui(
                ExplorerUiEvent::LoadFailed {
                    message: format!("{err}"),
                },
            )),
        }
    })
}
