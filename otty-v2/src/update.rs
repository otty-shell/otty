use iced::Task;
use iced::widget::operation::snap_to_end;

use super::{App, Event, routers};
use crate::ui::widgets::tab_bar;

pub(super) fn update(app: &mut App, event: Event) -> Task<Event> {
    let mut pre_dispatch_tasks = Vec::new();
    if let Some(task) =
        routers::quick_launch::pre_dispatch_inline_edit_cancel(app, &event)
    {
        pre_dispatch_tasks.push(task);
    }

    if let Some(task) = routers::window::resolve_context_menu_guard(app, &event)
    {
        return task;
    }

    let tabs_before = app.widgets.tab().len();
    let dispatch_task = route(app, event);
    let task = if pre_dispatch_tasks.is_empty() {
        dispatch_task
    } else {
        pre_dispatch_tasks.push(dispatch_task);
        Task::batch(pre_dispatch_tasks)
    };

    if app.widgets.tab().len() > tabs_before {
        Task::batch(vec![task, snap_to_end(tab_bar::TAB_BAR_SCROLL_ID)])
    } else {
        task
    }
}

fn route(app: &mut App, event: Event) -> Task<Event> {
    use Event::*;

    match event {
        IcedReady => routers::tabs::route_iced_ready(app),
        ActionBar(event) => routers::window::route_action_bar(app, event),
        SidebarUi(event) => routers::sidebar::route_event(app, event),
        SidebarEffect(event) => routers::sidebar::route_effect(event),
        ExplorerUi(event) => routers::explorer::route_event(app, event),
        ExplorerEffect(event) => routers::explorer::route_effect(event),
        QuickLaunch(event) => routers::quick_launch::route_event(app, event),
        ActivateTab { tab_id } => {
            routers::tabs::route_activate_tab(app, tab_id)
        },
        CloseTabRequested { tab_id } => {
            routers::tabs::route_close_tab(app, tab_id)
        },
        SetTabTitle { tab_id, title } => {
            routers::tabs::route_set_tab_title(app, tab_id, title)
        },
        OpenCommandTerminalTab { title, settings } => {
            routers::tabs::open_command_terminal_tab(app, title, *settings)
        },
        OpenQuickLaunchCommandTerminalTab {
            title,
            settings,
            command,
        } => routers::tabs::open_quick_launch_command_terminal_tab(
            app, title, *settings, command,
        ),
        OpenQuickLaunchWizardCreateTab { parent_path } => {
            routers::tabs::open_quick_launch_wizard_create_tab(app, parent_path)
        },
        OpenQuickLaunchWizardEditTab { path, command } => {
            routers::tabs::open_quick_launch_wizard_edit_tab(
                app, path, *command,
            )
        },
        OpenQuickLaunchErrorTab { title, message } => {
            routers::tabs::open_quick_launch_error_tab(app, title, message)
        },
        OpenTerminalTab => routers::tabs::open_terminal_tab(app),
        OpenSettingsTab => routers::tabs::open_settings_tab(app),
        SyncTerminalGridSizes => {
            routers::runtime::sync_terminal_grid_sizes(app);
            Task::none()
        },
        Terminal(event) => routers::terminal::route_event(app, event),
        QuickLaunchWizardUi { tab_id, event } => {
            routers::quick_launch_wizard::route_event(app, tab_id, event)
        },
        QuickLaunchWizardEffect(event) => {
            routers::quick_launch_wizard::route_effect(event)
        },
        Settings(event) => routers::settings::route_event(app, event),
        SettingsApplied(settings) => {
            routers::settings::route_applied(app, &settings)
        },
        Keyboard(event) => routers::quick_launch::route_keyboard(app, event),
        Window(event) => routers::window::route_window_event(app, event),
        ResizeWindow(dir) => routers::window::route_resize_window(dir),
    }
}
