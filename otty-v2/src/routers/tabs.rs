use iced::Task;
use otty_ui_term::settings::Settings;

use crate::app::{App, Event as AppEvent};
use crate::widgets::{quick_launch, tab};

/// Open initial terminal tab on app-ready event.
pub(crate) fn route_iced_ready(app: &mut App) -> Task<AppEvent> {
    open_terminal_tab(app)
}

/// Route activate-tab event.
pub(crate) fn route_activate_tab(app: &mut App, tab_id: u64) -> Task<AppEvent> {
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets
        .tab_mut()
        .reduce(tab::TabEvent::Activate { tab_id }, &ctx)
}

/// Route close-tab request.
pub(crate) fn route_close_tab(app: &mut App, tab_id: u64) -> Task<AppEvent> {
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets
        .tab_mut()
        .reduce(tab::TabEvent::CloseRequested { tab_id }, &ctx)
}

/// Route set-tab-title request.
pub(crate) fn route_set_tab_title(
    app: &mut App,
    tab_id: u64,
    title: String,
) -> Task<AppEvent> {
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets
        .tab_mut()
        .reduce(tab::TabEvent::SetTitle { tab_id, title }, &ctx)
}

/// Route app request to open a blank terminal tab.
pub(crate) fn open_terminal_tab(app: &mut App) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_mut().allocate_terminal_id();
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets
        .tab_mut()
        .reduce(tab::TabEvent::OpenTerminalTab { terminal_id }, &ctx)
}

/// Route app request to open settings tab.
pub(crate) fn open_settings_tab(app: &mut App) -> Task<AppEvent> {
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets
        .tab_mut()
        .reduce(tab::TabEvent::OpenSettingsTab, &ctx)
}

/// Route app request to open command terminal tab.
pub(crate) fn open_command_terminal_tab(
    app: &mut App,
    title: String,
    tab_settings: Settings,
) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_mut().allocate_terminal_id();
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets.tab_mut().reduce(
        tab::TabEvent::OpenCommandTerminalTab {
            title,
            terminal_id,
            settings: Box::new(tab_settings),
        },
        &ctx,
    )
}

/// Route app request to open quick-launch terminal tab.
pub(crate) fn open_quick_launch_command_terminal_tab(
    app: &mut App,
    title: String,
    tab_settings: Settings,
    command: Box<quick_launch::QuickLaunch>,
) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_mut().allocate_terminal_id();
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets.tab_mut().reduce(
        tab::TabEvent::OpenQuickLaunchCommandTerminalTab {
            title,
            terminal_id,
            settings: Box::new(tab_settings),
            command,
        },
        &ctx,
    )
}

/// Route app request to open quick-launch create wizard tab.
pub(crate) fn open_quick_launch_wizard_create_tab(
    app: &mut App,
    parent_path: quick_launch::NodePath,
) -> Task<AppEvent> {
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets.tab_mut().reduce(
        tab::TabEvent::OpenQuickLaunchWizardCreateTab { parent_path },
        &ctx,
    )
}

/// Route app request to open quick-launch edit wizard tab.
pub(crate) fn open_quick_launch_wizard_edit_tab(
    app: &mut App,
    path: quick_launch::NodePath,
    command: quick_launch::QuickLaunch,
) -> Task<AppEvent> {
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets.tab_mut().reduce(
        tab::TabEvent::OpenQuickLaunchWizardEditTab {
            path,
            command: Box::new(command),
        },
        &ctx,
    )
}

/// Route app request to open quick-launch error tab.
pub(crate) fn open_quick_launch_error_tab(
    app: &mut App,
    title: String,
    message: String,
) -> Task<AppEvent> {
    let ctx = tab::TabCtx {
        shell_session: &app.shell_session,
        terminal_settings: &app.terminal_settings,
    };
    app.widgets.tab_mut().reduce(
        tab::TabEvent::OpenQuickLaunchErrorTab { title, message },
        &ctx,
    )
}
