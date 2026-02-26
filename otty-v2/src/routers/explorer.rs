use iced::Task;

use crate::app::{App, Event as AppEvent};
use crate::widgets::explorer::{
    ExplorerCommand, ExplorerCtx, ExplorerEffectEvent, ExplorerUiEvent,
};
use crate::widgets::terminal::shell_cwd_for_active_tab;

/// Route explorer UI event into reducer command path.
pub(crate) fn route_event(
    app: &mut App,
    event: ExplorerUiEvent,
) -> Task<AppEvent> {
    let command = map_ui_event_to_command(event);
    let active_tab_id = app.widgets.tab().active_tab_id();
    let editor_command = app.widgets.settings().terminal_editor().to_string();
    let active_shell_cwd =
        shell_cwd_for_active_tab(active_tab_id, app.widgets.terminal());

    app.widgets
        .explorer_mut()
        .reduce(
            command,
            &ExplorerCtx {
                active_shell_cwd,
                terminal_settings: &app.terminal_settings,
                editor_command: &editor_command,
            },
        )
        .map(AppEvent::ExplorerEffect)
}

/// Route explorer side-effect events into app-level tasks.
pub(crate) fn route_effect(event: ExplorerEffectEvent) -> Task<AppEvent> {
    use ExplorerEffectEvent as E;

    use crate::widgets::explorer::{ExplorerLoadTarget, read_dir_nodes};

    match event {
        E::LoadRootRequested { root } => Task::perform(
            async move {
                let loaded = read_dir_nodes(&root.clone());
                (root, loaded)
            },
            |(root, result)| match result {
                Ok(nodes) => {
                    AppEvent::ExplorerUi(ExplorerUiEvent::RootLoaded {
                        root,
                        nodes,
                    })
                },
                Err(err) => AppEvent::ExplorerUi(ExplorerUiEvent::LoadFailed {
                    target: ExplorerLoadTarget::Root { root },
                    message: format!("{err}"),
                }),
            },
        ),
        E::LoadFolderRequested { path, directory } => Task::perform(
            async move {
                let loaded = read_dir_nodes(&directory.clone());
                (path, directory, loaded)
            },
            |(path, directory, result)| match result {
                Ok(nodes) => {
                    AppEvent::ExplorerUi(ExplorerUiEvent::FolderLoaded {
                        path,
                        nodes,
                    })
                },
                Err(err) => AppEvent::ExplorerUi(ExplorerUiEvent::LoadFailed {
                    target: ExplorerLoadTarget::Folder { path, directory },
                    message: format!("{err}"),
                }),
            },
        ),
        E::OpenCommandTerminalTab { title, settings } => {
            Task::done(AppEvent::OpenCommandTerminalTab { title, settings })
        },
    }
}

fn map_ui_event_to_command(event: ExplorerUiEvent) -> ExplorerCommand {
    use {ExplorerCommand as C, ExplorerUiEvent as E};

    match event {
        E::NodePressed { path } => C::NodePressed { path },
        E::NodeHovered { path } => C::NodeHovered { path },
        E::SyncFromActiveTerminal => C::SyncFromActiveTerminal,
        E::RootLoaded { root, nodes } => C::RootLoaded { root, nodes },
        E::FolderLoaded { path, nodes } => C::FolderLoaded { path, nodes },
        E::LoadFailed { target, message } => C::LoadFailed { target, message },
    }
}
