use iced::Task;

use super::{App, AppEvent};
use crate::events;
use crate::guards::{
    MenuGuard, context_menu_guard, inline_edit_guard, is_escape_press,
    is_header_menu_choice, is_terminal_input,
};
use crate::shortcuts::{AppShortcut, Platform, shortcut_for_key_press};
use crate::widgets::modes::{ModesEvent, ModesIntent};
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

pub(super) fn update(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    // Keys typed while the command palette is open belong to the palette,
    // even if a terminal still had keyboard focus.
    if app.widgets.palette.is_open() && is_terminal_input(&event) {
        return Task::none();
    }

    // Esc closes the All modes menu. The menu takes keyboard focus away
    // from the terminal when it opens, so the key reaches the app.
    if app.widgets.modes.is_menu_open() && is_escape_press(&event) {
        return close_modes_menu(app);
    }

    // Mode shortcuts still work while the All modes menu is open: the menu
    // shortcut closes it and a mode shortcut switches, which closes it too.
    // Every other key stays ignored like for the other menus.
    if app.widgets.modes.is_menu_open() && is_mode_shortcut(&event) {
        return events::handle(app, event);
    }

    let mut pre_dispatch_tasks = Vec::new();

    if app.widgets.quick_launch.has_inline_edit() && inline_edit_guard(&event) {
        pre_dispatch_tasks.push(events::handle(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::CancelInlineEdit,
            )),
        ));
    }

    if app.widgets.sidebar.has_header_menu_open()
        && is_header_menu_choice(&event)
    {
        pre_dispatch_tasks.push(events::handle(
            app,
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuDismiss,
            )),
        ));
    } else if any_context_menu_open(app) {
        match context_menu_guard(&event) {
            MenuGuard::Allow => {},
            MenuGuard::Ignore => return Task::none(),
            MenuGuard::Dismiss => return close_all_context_menus(app),
        }
    }

    let dispatch_task = events::handle(app, event);
    if pre_dispatch_tasks.is_empty() {
        dispatch_task
    } else {
        pre_dispatch_tasks.push(dispatch_task);
        Task::batch(pre_dispatch_tasks)
    }
}

fn is_mode_shortcut(event: &AppEvent) -> bool {
    let AppEvent::Keyboard(key_event) = event else {
        return false;
    };

    match shortcut_for_key_press(key_event, Platform::current()) {
        Some(AppShortcut::AllModesMenu) => true,
        Some(shortcut) => shortcut.mode_position().is_some(),
        None => false,
    }
}

fn any_context_menu_open(app: &App) -> bool {
    if app.widgets.sidebar.has_header_menu_open()
        || app.widgets.quick_launch.context_menu().is_some()
        || app.widgets.modes.is_menu_open()
    {
        return true;
    }

    app.widgets.terminal_workspace.has_any_context_menu()
}

fn close_all_context_menus(app: &mut App) -> Task<AppEvent> {
    Task::batch(vec![
        events::handle(
            app,
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::HeaderMenuDismiss,
            )),
        ),
        events::handle(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::ContextMenuDismiss,
            )),
        ),
        events::handle(
            app,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::CloseAllContextMenus,
            )),
        ),
        close_modes_menu(app),
    ])
}

fn close_modes_menu(app: &mut App) -> Task<AppEvent> {
    events::handle(
        app,
        AppEvent::Modes(ModesEvent::Intent(ModesIntent::CloseMenu)),
    )
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{Code, Named, NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::modes::types::Mode;

    fn app_with_open_modes_menu() -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        let _ = update(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu)),
        );
        app
    }

    fn escape_press() -> AppEvent {
        AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        })
    }

    /// The key press the keyboard subscription delivers for `shortcut`.
    fn shortcut_press(shortcut: AppShortcut) -> AppEvent {
        let keys = shortcut.keys(Platform::current());
        let physical_key = keys
            .code()
            .map(Physical::Code)
            .unwrap_or(Physical::Unidentified(NativeCode::Unidentified));

        AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Character(keys.key().into()),
            modified_key: Key::Character(keys.key().into()),
            physical_key,
            location: Location::Standard,
            modifiers: keys.modifiers(),
            text: None,
            repeat: false,
        })
    }

    /// Press `shortcut` and run the events its tasks produce through
    /// `update` again, as the iced runtime does.
    fn press(app: &mut App, shortcut: AppShortcut) {
        let follow_ups = collect_outputs(update(app, shortcut_press(shortcut)));

        for event in follow_ups {
            let _ = update(app, event);
        }
    }

    #[test]
    fn all_modes_shortcut_closes_the_open_modes_menu() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::AllModesMenu);

        assert!(!app.widgets.modes.is_menu_open());
    }

    #[test]
    fn mode_shortcut_switches_while_the_modes_menu_is_open() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::PinnedMode1);

        assert_eq!(app.widgets.modes.active(), Mode::Docker);
        assert!(!app.widgets.modes.is_menu_open());
    }

    #[test]
    fn other_shortcuts_stay_ignored_while_the_modes_menu_is_open() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::FocusMode);

        assert!(app.widgets.modes.is_menu_open());
        assert!(!app.widgets.sidebar.vm().is_focus_mode);
    }

    #[test]
    fn esc_closes_the_open_modes_menu() {
        let mut app = app_with_open_modes_menu();
        assert!(app.widgets.modes.is_menu_open());

        let _ = update(&mut app, escape_press());

        assert!(!app.widgets.modes.is_menu_open());
    }

    #[test]
    fn pin_and_unpin_keep_the_modes_menu_open() {
        let mut app = app_with_open_modes_menu();

        let _ = update(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Unpin(
                Mode::Docker,
            ))),
        );

        assert!(app.widgets.modes.is_menu_open());
        assert_eq!(app.widgets.modes.vm().pinned_count, 0);
    }

    #[test]
    fn choosing_a_mode_in_the_menu_switches_and_closes_it() {
        let mut app = app_with_open_modes_menu();

        let _ = update(
            &mut app,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Docker,
            ))),
        );

        assert_eq!(app.widgets.modes.active(), Mode::Docker);
        assert!(!app.widgets.modes.is_menu_open());
    }

    #[test]
    fn another_app_action_closes_the_modes_menu_like_the_header_menus() {
        let mut app = app_with_open_modes_menu();
        let was_left_open = app.widgets.sidebar.vm().is_left_open;

        let _ = update(
            &mut app,
            AppEvent::Sidebar(SidebarEvent::Intent(SidebarIntent::ToggleLeft)),
        );

        assert!(!app.widgets.modes.is_menu_open());
        assert_eq!(app.widgets.sidebar.vm().is_left_open, was_left_open);
    }
}
