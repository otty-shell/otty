use iced::Task;
use iced::widget::operation::snap_to_end;

use crate::app::{App, AppEvent};
use crate::widgets::tabs::view::tab_bar::TAB_BAR_SCROLL_ID;
use crate::widgets::tabs::{TabsCommand, TabsEffect, TabsEvent};
use crate::widgets::terminal_workspace::services::terminal_settings_for_session;

/// Route a tabs UI event through the widget reducer and map effects.
pub(crate) fn route_event(app: &mut App, event: TabsEvent) -> Task<AppEvent> {
    let command = map_tabs_event_to_command(event);
    route_command(app, command)
}

/// Route a tabs command through the widget reducer and map effects.
pub(crate) fn route_command(
    app: &mut App,
    command: TabsCommand,
) -> Task<AppEvent> {
    app.widgets.tabs.reduce(command).map(AppEvent::TabsEffect)
}

/// Route a tabs effect event to an app-level task.
pub(crate) fn route_effect(
    app: &mut App,
    effect: TabsEffect,
) -> Task<AppEvent> {
    match effect {
        TabsEffect::Activated { tab_id } => {
            let mut tasks: Vec<Task<AppEvent>> = Vec::new();

            // Focus the terminal in the activated tab.
            tasks.push(
                crate::routers::terminal_workspace::route_command(
                    app,
                    crate::widgets::terminal_workspace::TerminalWorkspaceCommand::FocusActive,
                ),
            );

            // Sync explorer from terminal CWD.
            let active_tab_id = Some(tab_id);
            if let Some(cwd) = app
                .widgets
                .terminal_workspace
                .shell_cwd_for_active_tab(active_tab_id)
            {
                tasks.push(crate::routers::explorer::route_command(
                    app,
                    crate::widgets::explorer::ExplorerCommand::SyncRoot { cwd },
                ));
            }

            Task::batch(tasks)
        },
        TabsEffect::Closed {
            tab_id,
            new_active_id,
            ..
        } => {
            let mut tasks: Vec<Task<AppEvent>> = Vec::new();

            // Notify terminal workspace of tab closure.
            tasks.push(
                crate::routers::terminal_workspace::route_command(
                    app,
                    crate::widgets::terminal_workspace::TerminalWorkspaceCommand::TabClosed { tab_id },
                ),
            );

            // Notify quick launch of tab closure.
            tasks.push(crate::routers::quick_launch::route_command(
                app,
                crate::widgets::quick_launch::QuickLaunchCommand::TabClosed {
                    tab_id,
                },
            ));

            // Sync explorer for the new active tab.
            if let Some(active_id) = new_active_id {
                if let Some(cwd) = app
                    .widgets
                    .terminal_workspace
                    .shell_cwd_for_active_tab(Some(active_id))
                {
                    tasks.push(crate::routers::explorer::route_command(
                        app,
                        crate::widgets::explorer::ExplorerCommand::SyncRoot {
                            cwd,
                        },
                    ));
                }
            }

            Task::batch(tasks)
        },
        TabsEffect::TerminalTabOpened {
            tab_id,
            terminal_id,
            title,
        } => {
            let settings = Box::new(terminal_settings_for_session(
                &app.terminal_settings,
                app.shell_session.session().clone(),
            ));
            let kind =
                crate::widgets::terminal_workspace::model::TerminalKind::Shell;

            crate::routers::terminal_workspace::route_command(
                app,
                crate::widgets::terminal_workspace::TerminalWorkspaceCommand::OpenTab {
                    tab_id,
                    terminal_id,
                    default_title: title,
                    settings,
                    kind,
                    sync_explorer: true,
                },
            )
        },
        TabsEffect::SettingsTabOpened => {
            crate::routers::settings::route_command(
                app,
                crate::widgets::settings::SettingsCommand::Reload,
            )
        },
        TabsEffect::WizardTabOpened { .. } => Task::none(),
        TabsEffect::ErrorTabOpened { .. } => Task::none(),
        TabsEffect::ScrollBarToEnd => snap_to_end(TAB_BAR_SCROLL_ID),
    }
}

fn map_tabs_event_to_command(event: TabsEvent) -> TabsCommand {
    match event {
        TabsEvent::ActivateTab { tab_id } => TabsCommand::Activate { tab_id },
        TabsEvent::CloseTab { tab_id } => TabsCommand::Close { tab_id },
    }
}
