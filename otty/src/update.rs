use iced::widget::operation::snap_to_end;
use iced::{Point, Task, window};
use otty_ui_term::settings::Settings;

use super::{App, Event, quick_launch};
use crate::features::explorer::{ExplorerCtx, ExplorerEvent};
use crate::features::quick_launch_wizard::QuickLaunchWizardCtx;
use crate::features::sidebar::{self, SidebarEvent};
use crate::features::terminal::{
    ShellSession, TerminalCtx, TerminalEvent, fallback_shell_session_with_shell, setup_shell_session_with_shell, shell_cwd_for_active_tab
};
use crate::features::{settings, tab};
use crate::guards::{MenuGuard, context_menu_guard, inline_edit_guard};
use crate::state::pane_grid_size;
use crate::ui::widgets::{action_bar, tab_bar};

pub(super) fn update(app: &mut App, event: Event) -> Task<Event> {
    let mut pre_dispatch_tasks = Vec::new();
    if app.features.quick_launch().inline_edit().is_some()
        && inline_edit_guard(&event)
    {
        let ctx = quick_launch_ctx(
            &app.terminal_settings,
            app.features.sidebar().cursor(),
            app.features.sidebar().is_resizing(),
        );
        pre_dispatch_tasks.push(
            app.features
                .quick_launch_mut()
                .reduce(quick_launch::QuickLaunchEvent::CancelInlineEdit, &ctx),
        );
    }

    if any_context_menu_open(app) {
        match context_menu_guard(&event) {
            MenuGuard::Allow => {},
            MenuGuard::Ignore => return Task::none(),
            MenuGuard::Dismiss => {
                return close_all_context_menus(app);
            },
        }
    }

    let tabs_before = app.features.tab().len();
    let dispatch_task = route(app, event);
    let task = if pre_dispatch_tasks.is_empty() {
        dispatch_task
    } else {
        pre_dispatch_tasks.push(dispatch_task);
        Task::batch(pre_dispatch_tasks)
    };

    if app.features.tab().len() > tabs_before {
        Task::batch(vec![task, snap_to_end(tab_bar::TAB_BAR_SCROLL_ID)])
    } else {
        task
    }
}

fn route(app: &mut App, event: Event) -> Task<Event> {
    use Event::*;

    match event {
        IcedReady => open_terminal_tab(app),
        ActionBar(event) => handle_action_bar(app, event),
        Sidebar(event) => app
            .features
            .sidebar_mut()
            .reduce(SidebarEvent::Menu(event), &()),
        SidebarWorkspace(event) => app
            .features
            .sidebar_mut()
            .reduce(SidebarEvent::Workspace(event), &()),
        QuickLaunch(event) => {
            let ctx = quick_launch_ctx(
                &app.terminal_settings,
                app.features.sidebar().cursor(),
                app.features.sidebar().is_resizing(),
            );
            app.features.quick_launch_mut().reduce(event, &ctx)
        },
        ActivateTab { tab_id } => activate_tab(app, tab_id),
        CloseTabRequested { tab_id } => close_tab(app, tab_id),
        SetTabTitle { tab_id, title } => set_tab_title(app, tab_id, title),
        OpenCommandTerminalTab { title, settings } => {
            open_command_terminal_tab(app, title, *settings)
        },
        OpenQuickLaunchCommandTerminalTab {
            title,
            settings,
            command,
        } => open_quick_launch_command_terminal_tab(
            app, title, *settings, command,
        ),
        OpenQuickLaunchWizardCreateTab { parent_path } => {
            open_quick_launch_wizard_create_tab(app, parent_path)
        },
        OpenQuickLaunchWizardEditTab { path, command } => {
            open_quick_launch_wizard_edit_tab(app, path, *command)
        },
        OpenQuickLaunchErrorTab { title, message } => {
            open_quick_launch_error_tab(app, title, message)
        },
        OpenTerminalTab => open_terminal_tab(app),
        OpenSettingsTab => open_settings_tab(app),
        SyncTerminalGridSizes => {
            sync_terminal_grid_sizes(app);
            Task::none()
        },
        Explorer(event) => {
            let active_tab_id = app.features.tab().active_tab_id();
            let editor_command =
                app.features.settings().terminal_editor().to_string();
            let active_shell_cwd = shell_cwd_for_active_tab(
                active_tab_id,
                app.features.terminal(),
            );
            app.features.explorer_mut().reduce(
                event,
                &ExplorerCtx {
                    active_shell_cwd,
                    terminal_settings: &app.terminal_settings,
                    editor_command: &editor_command,
                },
            )
        },
        Terminal(event) => {
            let sidebar_task =
                if let TerminalEvent::PaneGridCursorMoved { position, .. } =
                    &event
                {
                    app.features.sidebar_mut().reduce(
                        SidebarEvent::PaneGridCursorMoved {
                            position: *position,
                        },
                        &(),
                    )
                } else {
                    Task::none()
                };

            let ctx = make_terminal_ctx(app);
            let sync_task = terminal_sync_followup(&event);
            let terminal_task = app.features.terminal_mut().reduce(event, &ctx);
            Task::batch(vec![sidebar_task, terminal_task, sync_task])
        },
        QuickLaunchWizard { tab_id, event } => app
            .features
            .quick_launch_wizard_mut()
            .reduce(event, &QuickLaunchWizardCtx { tab_id }),
        Settings(event) => app.features.settings_mut().reduce(event, &()),
        SettingsApplied(settings) => apply_settings(app, &settings),
        Keyboard(event) => handle_keyboard(app, event),
        Window(window::Event::Resized(size)) => {
            app.window_size = size;
            app.state.window_size = size;
            app.state
                .set_screen_size(super::view::screen_size_from_window(size));
            sync_terminal_grid_sizes(app);
            Task::none()
        },
        Window(_) => Task::none(),
        ResizeWindow(dir) => {
            window::latest().and_then(move |id| window::drag_resize(id, dir))
        },
    }
}

fn handle_keyboard(app: &mut App, event: iced::keyboard::Event) -> Task<Event> {
    if let iced::keyboard::Event::KeyPressed { key, .. } = event {
        let ctx = quick_launch_ctx(
            &app.terminal_settings,
            app.features.sidebar().cursor(),
            app.features.sidebar().is_resizing(),
        );
        if matches!(
            key,
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
        ) && app.features.quick_launch().inline_edit().is_some()
        {
            return app.features.quick_launch_mut().reduce(
                quick_launch::QuickLaunchEvent::CancelInlineEdit,
                &ctx,
            );
        }

        if matches!(
            key,
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
        ) && app.features.quick_launch().inline_edit().is_none()
        {
            return app
                .features
                .quick_launch_mut()
                .reduce(quick_launch::QuickLaunchEvent::DeleteSelected, &ctx);
        }
    }

    Task::none()
}

fn terminal_sync_followup(event: &TerminalEvent) -> Task<Event> {
    let should_sync = matches!(
        event,
        TerminalEvent::PaneClicked { .. }
            | TerminalEvent::SplitPane { .. }
            | TerminalEvent::ClosePane { .. }
            | TerminalEvent::Widget(otty_ui_term::Event::ContentSync { .. })
    );

    if should_sync {
        Task::done(Event::Explorer(ExplorerEvent::SyncFromActiveTerminal))
    } else {
        Task::none()
    }
}

fn apply_settings(
    app: &mut App,
    settings: &settings::SettingsData,
) -> Task<Event> {
    let palette = settings.to_color_palette();
    app.theme_manager.set_custom_palette(palette);
    let current_theme = app.theme_manager.current();
    app.terminal_settings = super::terminal_settings(current_theme, &app.fonts);
    let terminal_palette = current_theme.terminal_palette().clone();

    match setup_shell_session_with_shell(settings.terminal_shell()) {
        Ok(session) => app.shell_session = session,
        Err(err) => {
            log::warn!("shell integration setup failed: {err}");
            app.shell_session =
                fallback_shell_session_with_shell(settings.terminal_shell());
        },
    }

    let ctx = make_terminal_ctx(app);
    app.features.terminal_mut().reduce(
        TerminalEvent::ApplyTheme {
            palette: Box::new(terminal_palette),
        },
        &ctx,
    )
}

fn handle_action_bar(
    app: &mut App,
    event: action_bar::ActionBarEvent,
) -> Task<Event> {
    use action_bar::ActionBarEvent::*;

    match event {
        ToggleFullScreen => toggle_full_screen(app),
        MinimizeWindow => {
            window::latest().and_then(|id| window::minimize(id, true))
        },
        CloseWindow => iced::window::latest().and_then(iced::window::close),
        ToggleSidebarVisibility => app
            .features
            .sidebar_mut()
            .reduce(SidebarEvent::ToggleVisibility, &()),
        StartWindowDrag => window::latest().and_then(window::drag),
    }
}

pub(super) fn any_context_menu_open(app: &App) -> bool {
    if app.features.sidebar().has_add_menu_open()
        || app.features.quick_launch().context_menu().is_some()
    {
        return true;
    }

    app.features.terminal().has_any_context_menu()
}

pub(super) fn close_all_context_menus(app: &mut App) -> Task<Event> {
    let sidebar_task = app
        .features
        .sidebar_mut()
        .reduce(SidebarEvent::DismissAddMenu, &());
    let ctx = quick_launch_ctx(
        &app.terminal_settings,
        app.features.sidebar().cursor(),
        app.features.sidebar().is_resizing(),
    );
    let quick_launch_task = app
        .features
        .quick_launch_mut()
        .reduce(quick_launch::QuickLaunchEvent::ContextMenuDismiss, &ctx);
    let ctx = make_terminal_ctx(app);
    let terminal_task = app
        .features
        .terminal_mut()
        .reduce(TerminalEvent::CloseAllContextMenus, &ctx);
    Task::batch(vec![sidebar_task, quick_launch_task, terminal_task])
}

fn activate_tab(app: &mut App, tab_id: u64) -> Task<Event> {
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features
        .tab_mut()
        .reduce(tab::TabEvent::Activate { tab_id }, &ctx)
}

fn close_tab(app: &mut App, tab_id: u64) -> Task<Event> {
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features
        .tab_mut()
        .reduce(tab::TabEvent::CloseRequested { tab_id }, &ctx)
}

fn set_tab_title(app: &mut App, tab_id: u64, title: String) -> Task<Event> {
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features
        .tab_mut()
        .reduce(tab::TabEvent::SetTitle { tab_id, title }, &ctx)
}

fn open_terminal_tab(app: &mut App) -> Task<Event> {
    let terminal_id = app.features.terminal_mut().allocate_terminal_id();
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features
        .tab_mut()
        .reduce(tab::TabEvent::OpenTerminalTab { terminal_id }, &ctx)
}

fn open_settings_tab(app: &mut App) -> Task<Event> {
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features
        .tab_mut()
        .reduce(tab::TabEvent::OpenSettingsTab, &ctx)
}

fn open_command_terminal_tab(
    app: &mut App,
    title: String,
    tab_settings: Settings,
) -> Task<Event> {
    let terminal_id = app.features.terminal_mut().allocate_terminal_id();
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features.tab_mut().reduce(
        tab::TabEvent::OpenCommandTerminalTab {
            title,
            terminal_id,
            settings: Box::new(tab_settings),
        },
        &ctx,
    )
}

fn open_quick_launch_command_terminal_tab(
    app: &mut App,
    title: String,
    tab_settings: Settings,
    command: Box<quick_launch::QuickLaunch>,
) -> Task<Event> {
    let terminal_id = app.features.terminal_mut().allocate_terminal_id();
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features.tab_mut().reduce(
        tab::TabEvent::OpenQuickLaunchCommandTerminalTab {
            title,
            terminal_id,
            settings: Box::new(tab_settings),
            command,
        },
        &ctx,
    )
}

fn open_quick_launch_wizard_create_tab(
    app: &mut App,
    parent_path: quick_launch::NodePath,
) -> Task<Event> {
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features.tab_mut().reduce(
        tab::TabEvent::OpenQuickLaunchWizardCreateTab { parent_path },
        &ctx,
    )
}

fn open_quick_launch_wizard_edit_tab(
    app: &mut App,
    path: quick_launch::NodePath,
    command: quick_launch::QuickLaunch,
) -> Task<Event> {
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features.tab_mut().reduce(
        tab::TabEvent::OpenQuickLaunchWizardEditTab {
            path,
            command: Box::new(command),
        },
        &ctx,
    )
}

fn open_quick_launch_error_tab(
    app: &mut App,
    title: String,
    message: String,
) -> Task<Event> {
    let ctx = tab_ctx(&app.terminal_settings, &app.shell_session);
    app.features.tab_mut().reduce(
        tab::TabEvent::OpenQuickLaunchErrorTab { title, message },
        &ctx,
    )
}

fn toggle_full_screen(app: &mut App) -> Task<Event> {
    app.is_fullscreen = !app.is_fullscreen;

    let mode = if app.is_fullscreen {
        window::Mode::Fullscreen
    } else {
        window::Mode::Windowed
    };

    window::latest().and_then(move |id| window::set_mode(id, mode))
}

/// Build a terminal context snapshot from the current app state.
fn make_terminal_ctx(app: &App) -> TerminalCtx {
    TerminalCtx {
        active_tab_id: app.features.tab().active_tab_id(),
        pane_grid_size: current_pane_grid_size(app),
        screen_size: app.state.screen_size,
        sidebar_cursor: app.features.sidebar().cursor(),
    }
}

/// Propagate the current pane grid size to the terminal feature.
fn sync_terminal_grid_sizes(app: &mut App) {
    let size = current_pane_grid_size(app);
    app.features.terminal_mut().set_grid_size(size);
}

fn current_pane_grid_size(app: &App) -> iced::Size {
    let sidebar = app.features.sidebar();
    pane_grid_size(
        app.state.screen_size,
        sidebar.is_hidden(),
        sidebar::SIDEBAR_MENU_WIDTH,
        sidebar.effective_workspace_ratio(),
    )
}

fn quick_launch_ctx<'a>(
    terminal_settings: &'a otty_ui_term::settings::Settings,
    sidebar_cursor: Point,
    sidebar_is_resizing: bool,
) -> quick_launch::QuickLaunchCtx<'a> {
    quick_launch::QuickLaunchCtx {
        terminal_settings,
        sidebar_cursor,
        sidebar_is_resizing,
    }
}

fn tab_ctx<'a>(
    terminal_settings: &'a Settings,
    shell_session: &'a ShellSession,
) -> tab::TabCtx<'a> {
    tab::TabCtx {
        shell_session,
        terminal_settings,
    }
}
