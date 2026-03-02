use std::path::PathBuf;

use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::services::editor_terminal_settings;
use crate::widgets::explorer::model::FileNode;
use crate::widgets::explorer::services::read_dir_nodes;
use crate::widgets::explorer::{
    ExplorerCtx, ExplorerEffect, ExplorerEvent, ExplorerIntent,
};
use crate::widgets::tabs::{TabsEvent, TabsIntent};

pub(crate) fn handle(app: &mut App, event: ExplorerEvent) -> Task<AppEvent> {
    match event {
        ExplorerEvent::Intent(event) => handle_intent(app, event),
        ExplorerEvent::Effect(effect) => handle_effect(app, effect),
    }
}

fn handle_intent(app: &mut App, event: ExplorerIntent) -> Task<AppEvent> {
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

fn handle_effect(app: &mut App, effect: ExplorerEffect) -> Task<AppEvent> {
    match effect {
        ExplorerEffect::LoadRootRequested { root } => {
            load_directory_async(root.clone(), move |nodes| {
                ExplorerIntent::RootLoaded {
                    root: root.clone(),
                    nodes,
                }
            })
        },
        ExplorerEffect::LoadFolderRequested { path, directory } => {
            load_directory_async(directory, move |nodes| {
                ExplorerIntent::FolderLoaded {
                    path: path.clone(),
                    nodes,
                }
            })
        },
        ExplorerEffect::OpenFileTerminalTab { file_path } => {
            open_file_terminal_tab(app, file_path)
        },
    }
}

/// Asynchronously read a directory and produce a completion event.
fn load_directory_async<F>(dir: PathBuf, on_complete: F) -> Task<AppEvent>
where
    F: Fn(Vec<FileNode>) -> ExplorerIntent + Send + 'static,
{
    Task::perform(async move { read_dir_nodes(&dir) }, move |result| {
        match result {
            Ok(nodes) => {
                AppEvent::Explorer(ExplorerEvent::Intent(on_complete(nodes)))
            },
            Err(err) => AppEvent::Explorer(ExplorerEvent::Intent(
                ExplorerIntent::LoadFailed {
                    message: format!("{err}"),
                },
            )),
        }
    })
}

fn open_file_terminal_tab(app: &mut App, file_path: PathBuf) -> Task<AppEvent> {
    let Some(settings) = editor_terminal_settings(
        app.shell_session.name().trim(),
        &app.terminal_settings,
        &file_path,
    ) else {
        return Task::none();
    };

    let file_display = file_path.display();
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{file_display}"));

    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenCommandTab {
            title,
            settings: Box::new(settings),
        },
    )))
}
