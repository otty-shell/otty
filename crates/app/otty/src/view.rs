use iced::widget::{Space, Stack, column, container, mouse_area};
use iced::{Element, Length};
use otty_app_domain::mode::Mode;
use otty_app_slots::Overlays;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::divider;
#[cfg(not(target_os = "macos"))]
use otty_ui_kit::components::resize_grips::resize_grips;

use crate::app::App;
use crate::chrome::view::action_bar;
use crate::message::AppEvent;
use crate::modes::view::{mode_menu, mode_switcher};
use crate::screens::{containers, over_main_area, terminal};
use crate::sidebar::types::SidebarPane;
use crate::sidebar::view::{focus_note, panels};
use crate::sidebar::{SidebarIntent, SidebarViewModel};

/// Render the window: the window content and the overlays over it, in one
/// order for every mode.
///
/// The layers from bottom to top:
///
/// 1. the window content: the header, the panels with the focus note and
///    the toasts over them, and the status bar;
/// 2. the overlays of the active mode, in one layer;
/// 3. the header menus and the All modes menu;
/// 4. the command palette;
/// 5. the notifications list;
/// 6. Settings;
/// 7. the SSH secret prompt;
/// 8. the grips at the window edges.
///
/// The layers of the features that have not moved in yet are missing; their
/// issues put them at their place in this order.
pub(crate) fn view(app: &App) -> Element<'_, AppEvent> {
    let theme = ThemeProps::new(app.theme_manager.current());
    let sidebar_vm = app.sidebar.vm();
    let mode = app.modes.active();

    let window_content = view_window_content(app, mode, sidebar_vm, theme);
    let mode_overlays = match mode {
        Mode::Terminal => terminal::overlays(app, theme),
        Mode::Containers => containers::overlays(theme),
    };
    // The mode overlays share one layer, so that one of them opening or
    // closing does not move the layers above it.
    let mode_overlays: Element<'_, AppEvent> =
        Stack::with_children(mode_overlays)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    let header_menu = view_header_menu(app, mode, sidebar_vm, theme);
    let modes_menu = view_modes_menu(app, theme);
    let palette = view_palette(app, theme);
    let notification_list = view_notification_list(app, theme);
    let settings = view_settings(app, theme);
    let resize_grips = view_resize_grips(app);

    // The header menus of the Sessions and Launches sections lie under the
    // modes menu, anchored where they were opened. The SSH secret prompt
    // (issue 12) follows Settings. All of them lie in the main area, between
    // the header and the status bar, so the window buttons stay usable
    // under them.
    let mut layers = vec![window_content, mode_overlays];
    layers.extend(header_menu.map(over_main_area));
    layers.extend(modes_menu.map(over_main_area));
    layers.extend(palette.map(over_main_area));
    layers.extend(notification_list.map(over_main_area));
    layers.extend(settings.map(over_main_area));
    layers.push(resize_grips);

    Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The window content: the header, the panels of the active mode with the
/// focus note over them, and the status bar of the active mode.
fn view_window_content<'a>(
    app: &'a App,
    mode: Mode,
    sidebar_vm: SidebarViewModel,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    // The active mode decides what goes into each slot; the sidebar keeps
    // the panels open or closed and their widths.
    let content_row = panels::view(panels::PanelsProps {
        panes: app.sidebar.panes(),
        vm: sidebar_vm,
        theme,
        content: |slot| match (slot, mode) {
            (SidebarPane::Left, _) => {
                view_left_panel(app, mode, sidebar_vm, theme)
            },
            (SidebarPane::Main, Mode::Terminal) => terminal::main(app, theme),
            (SidebarPane::Main, Mode::Containers) => containers::main(theme),
            (SidebarPane::Right, Mode::Terminal) => {
                terminal::right_panel(app, theme)
            },
            (SidebarPane::Right, Mode::Containers) => {
                containers::right_panel(theme)
            },
        },
        on_intent: AppEvent::Sidebar,
    });

    let content_row = mouse_area(content_row).on_move(|position| {
        AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { position })
    });

    let mut main_area: Vec<Element<'_, AppEvent>> = vec![content_row.into()];
    if let Some(note) = focus_note::view(sidebar_vm.focus_note_opacity, theme) {
        main_area.push(note);
    }
    // The toasts lie over the panels and the focus note, under every
    // overlay.
    if let Some(toasts) = app.notifications.toasts(theme) {
        main_area.push(toasts.map(AppEvent::Notifications));
    }
    let main_area = Stack::with_children(main_area)
        .width(Length::Fill)
        .height(Length::Fill);

    let header = view_header(app, sidebar_vm, theme);
    let status_bar = match mode {
        Mode::Terminal => terminal::status_bar(app, theme),
        Mode::Containers => containers::status_bar(theme),
    };

    column![header, main_area, status_bar]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render the header: action bar + separator.
fn view_header<'a>(
    app: &'a App,
    sidebar_vm: SidebarViewModel,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let action_bar = action_bar::view(action_bar::ActionBarProps {
        title: app.tabs.active_tab_title(),
        theme,
        left_open: sidebar_vm.is_left_open,
        right_open: sidebar_vm.is_right_open,
        focus_mode: sidebar_vm.is_focus_mode,
        notifications_open: app.notifications.is_list_open(),
        unseen_notifications: app.notifications.unseen_count(),
        is_title_bar_pressed: app.chrome.is_title_bar_pressed(),
    })
    .map(AppEvent::Chrome);

    let separator = divider::horizontal(theme.theme.ui());

    column![action_bar, separator]
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
}

/// Left panel content: the mode switcher, then the active mode's content.
/// The panel and the switcher hide together.
fn view_left_panel<'a>(
    app: &'a App,
    mode: Mode,
    sidebar_vm: SidebarViewModel,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let switcher = mode_switcher::view(mode_switcher::ModeSwitcherProps {
        vm: app.modes.vm(),
        theme,
    })
    .map(AppEvent::Modes);

    let content = match mode {
        Mode::Terminal => terminal::left_panel(app, sidebar_vm, theme),
        Mode::Containers => containers::left_panel(theme),
    };

    column![switcher, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The open menu of a section header, in Terminal mode, where the section
/// headers are.
fn view_header_menu<'a>(
    app: &'a App,
    mode: Mode,
    sidebar_vm: SidebarViewModel,
    theme: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent>> {
    let open = sidebar_vm.open_header_menu?;

    match mode {
        Mode::Terminal => terminal::header_menu(app, open, theme),
        Mode::Containers => None,
    }
}

/// The All modes menu under the switcher, while it is open.
fn view_modes_menu<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent>> {
    let vm = app.modes.vm();
    if !vm.is_menu_open {
        return None;
    }

    Some(
        mode_menu::view(mode_menu::ModeMenuProps { vm, theme })
            .map(AppEvent::Modes),
    )
}

/// The command palette, while it is open, in either mode.
fn view_palette<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent>> {
    let palette = app.palette.overlay(theme)?;

    Some(palette.into_content().map(AppEvent::Palette))
}

/// The notifications list under the bell, while it is open, in either mode.
fn view_notification_list<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent>> {
    let list = app.notifications.overlay(theme)?;

    Some(list.into_content().map(AppEvent::Notifications))
}

/// The Settings dialog, while it is open, in either mode. A press on the
/// dimmed area around it closes it, like Esc.
fn view_settings<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent>> {
    let dialog = app.settings.overlay(theme)?;

    Some(dialog.into_content().map(AppEvent::Settings))
}

/// The grips that resize the window at its edges. While a menu (a header
/// menu, the All modes menu, a pane menu), the notifications list or
/// Settings is open, an empty layer stands in for them. The features add
/// their overlays to this check as they move in.
#[cfg(not(target_os = "macos"))]
fn view_resize_grips(app: &App) -> Element<'_, AppEvent> {
    if app.sidebar.has_header_menu_open()
        || app.modes.is_menu_open()
        || app.notifications.is_list_open()
        || app.settings.is_open()
        || app.terminal.has_context_menu()
    {
        return container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    resize_grips(AppEvent::ResizeWindow)
}

/// The system resizes the window on macOS, so an empty layer stands in for
/// the grips.
#[cfg(target_os = "macos")]
fn view_resize_grips(_app: &App) -> Element<'_, AppEvent> {
    container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_notifications::NotificationsIntent;
    use otty_app_palette::PaletteIntent;
    use otty_app_settings::SettingsIntent;
    use otty_app_slots::Feature;
    use otty_app_theme::theme::BuiltinTheme;
    use otty_ui_kit::components::command_palette::PaletteAction;
    use otty_ui_kit::size;

    use super::*;
    use crate::chrome::ChromeIntent;
    use crate::message::Message;
    use crate::modes::ModesIntent;
    use crate::testing::frames::{Frames, scroll_main_area};
    use crate::testing::window::{
        app, edit_the_shell, open_notification_list, open_palette,
        open_settings, show_toast,
    };

    #[test]
    fn the_window_shows_the_header_and_the_status_bar() {
        let app = app();
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::TitleSearchNoSessions))
            .expect("the title search should show");
        ui.find(format!("OTTY {}", env!("CARGO_PKG_VERSION")))
            .expect("the status bar should show");
    }

    fn click_in_window(app: &App, point: Point) -> Vec<AppEvent> {
        let mut ui = simulator(view(app));
        ui.point_at(point);
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    /// The center of the switcher tab at `index`: under the title bar and
    /// its line, inside the row padding and the group.
    fn switcher_tab(index: f32) -> Point {
        Point::new(
            8.0 + 3.0 + 15.0 + index * 32.0,
            size::TITLEBAR + size::LINE + 8.0 + 3.0 + 13.0,
        )
    }

    fn mode_events(events: Vec<AppEvent>) -> Vec<AppEvent> {
        events
            .into_iter()
            .filter(|event| matches!(event, AppEvent::Modes(_)))
            .collect()
    }

    #[test]
    fn a_hidden_left_panel_hides_the_switcher() {
        let mut app = app();
        let shown = mode_events(click_in_window(&app, switcher_tab(1.0)));

        let _ = app.sidebar.reduce(SidebarIntent::ToggleLeft, ());
        let hidden = mode_events(click_in_window(&app, switcher_tab(1.0)));

        assert!(!app.sidebar.vm().is_left_open);
        assert_eq!(shown.len(), 1);
        assert!(hidden.is_empty());
    }

    #[test]
    fn clicking_tabs_and_more_in_the_window_switches_modes_and_opens_the_menu()
    {
        let mut app = app();

        for (point, expected) in [
            (switcher_tab(1.0), Mode::Containers),
            (switcher_tab(0.0), Mode::Terminal),
        ] {
            for event in mode_events(click_in_window(&app, point)) {
                let _ = app.update(Message::Input(event));
            }
            assert_eq!(app.modes.active(), expected);
        }

        for event in mode_events(click_in_window(&app, switcher_tab(2.0))) {
            let _ = app.update(Message::Input(event));
        }
        assert!(app.modes.is_menu_open());
    }

    #[test]
    fn an_open_modes_menu_is_drawn_over_the_window() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::ModesPinnedSection))
            .expect("all modes menu should be visible");
    }

    #[test]
    fn clicking_a_mode_name_in_the_open_menu_selects_it() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());
        let mut ui = simulator(view(&app));

        ui.click(i18n::t(Key::ModeContainers))
            .expect("containers row should be visible");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesIntent::Select(Mode::Containers))]
        ));
    }

    #[test]
    fn terminal_mode_shows_the_sessions_and_the_start_card() {
        let app = app();
        let mut ui = simulator(view(&app));

        assert!(
            ui.find(i18n::t(Key::ModeTerminal)).is_err(),
            "the switcher row has no mode name"
        );
        ui.find(i18n::t(Key::SessionsPanelTitle))
            .expect("sessions should be visible");
        ui.find(i18n::t(Key::EmptyTitle))
            .expect("the start card should be visible");
    }

    /// The title search shows the title of the active tab.
    #[test]
    fn the_title_bar_shows_the_active_tab() {
        let mut app = app();
        crate::testing::window::open_terminal_tab(&mut app);
        let title = app.shell_session.name().to_string();

        let mut ui = simulator(view(&app));

        assert!(ui.find(i18n::t(Key::TitleSearchNoSessions)).is_err());
        assert!(ui.find(title.as_str()).is_ok());
    }

    /// The New session menu lies under the All modes menu.
    #[test]
    fn the_modes_menu_lies_over_the_new_session_menu() {
        let mut app = app();
        let _ = app.sidebar.reduce(
            SidebarIntent::HeaderMenuOpen(
                crate::sidebar::types::HeaderMenu::Sessions,
            ),
            (),
        );
        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());

        let mut ui = simulator(view(&app));

        assert!(ui.find(i18n::t(Key::MenuNewLaunch)).is_ok());
        ui.click(i18n::t(Key::ModeContainers))
            .expect("the modes menu should take the press");
        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesIntent::Select(Mode::Containers))]
        ));
    }

    #[test]
    fn containers_mode_keeps_the_header_switcher_and_status_bar() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::TitleSearchNoSessions))
            .expect("the title search should show");
        assert!(ui.find(i18n::t(Key::ThisComputer)).is_err());
        drop(ui);
        let switched = mode_events(click_in_window(&app, switcher_tab(0.0)));
        assert!(matches!(
            switched.as_slice(),
            [AppEvent::Modes(ModesIntent::Select(Mode::Terminal))]
        ));
    }

    #[test]
    fn the_window_buttons_stay_usable_under_the_modes_menu() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());
        let close_window = Point::new(
            1024.0 - otty_ui_kit::spacing::SPACE_2 - size::CONTROL_MD / 2.0,
            size::TITLEBAR / 2.0,
        );

        let events = click_in_window(&app, close_window);

        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Chrome(ChromeIntent::CloseWindow)
        )));
    }

    /// The grips at the window edges lie over every layer, so they stay
    /// away while a menu is open.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_resize_grips_are_off_while_the_modes_menu_is_open() {
        let resizes = |app: &App| {
            click_in_window(app, Point::new(1023.0, 400.0))
                .into_iter()
                .filter(|event| matches!(event, AppEvent::ResizeWindow(_)))
                .count()
        };
        let mut app = app();
        let without_menu = resizes(&app);

        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());

        assert_eq!(without_menu, 1);
        assert_eq!(resizes(&app), 0);
    }

    /// The middle of the cross of the only toast, which shows `text` on
    /// one line: the toast is 420 px wide in the middle of the window.
    fn toast_cross(
        ui: &mut iced_test::Simulator<'_, AppEvent>,
        text: &str,
    ) -> Point {
        let line = ui.find(text).expect("the toast should show").bounds();

        Point::new(512.0 + 210.0 - 12.0 - 8.0, line.center_y())
    }

    fn in_mode(mode: Mode) -> App {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(mode), ());

        app
    }

    /// A toast lies at the top of the main area, under the header, in
    /// either mode, and its cross hides it.
    #[test]
    fn a_toast_shows_under_the_header_and_its_cross_hides_it() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = in_mode(mode);
            show_toast(&mut app, "Could not save the pinned modes");
            let mut ui = simulator(view(&app));

            let line = ui
                .find("Could not save the pinned modes")
                .expect("the toast should show")
                .bounds();
            let cross = toast_cross(&mut ui, "Could not save the pinned modes");
            ui.point_at(cross);
            let _ = ui.simulate(iced_test::simulator::click());

            let events: Vec<AppEvent> = ui.into_messages().collect();
            assert!(line.y > size::TITLEBAR + size::LINE, "{line:?}");
            assert!(line.y < size::TITLEBAR + 40.0, "{line:?}");
            assert!(
                events.iter().any(|event| matches!(
                    event,
                    AppEvent::Notifications(NotificationsIntent::DismissToast(
                        _
                    ))
                )),
                "{mode:?}"
            );
        }
    }

    /// The list lies under the header at the right, in either mode, and a
    /// press outside it closes it.
    #[test]
    fn the_notification_list_shows_under_the_bell_and_a_press_outside_closes_it()
     {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = in_mode(mode);
            show_toast(&mut app, "Could not save the pinned modes");
            open_notification_list(&mut app);
            let mut ui = simulator(view(&app));

            let title = ui
                .find(i18n::t(Key::NotificationsTitle))
                .expect("the list should show")
                .bounds();
            ui.find("Could not save the pinned modes")
                .expect("the row should show");
            ui.point_at(Point::new(200.0, 600.0));
            let _ = ui.simulate(iced_test::simulator::click());

            let events: Vec<AppEvent> = ui.into_messages().collect();
            assert!(title.y > size::TITLEBAR, "{title:?}");
            assert!(title.x > 1024.0 / 2.0, "{title:?}");
            assert!(
                events.iter().any(|event| matches!(
                    event,
                    AppEvent::Notifications(NotificationsIntent::CloseList)
                )),
                "{mode:?}"
            );
        }
    }

    /// The palette lies in the main area under the header, over the
    /// overlays of either mode (a Containers dialog among them), and takes
    /// the click.
    #[test]
    fn the_palette_is_drawn_over_the_mode_overlays_and_takes_the_click() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = in_mode(mode);
            open_palette(&mut app);
            let mut ui = simulator(view(&app));

            let input = ui
                .find(i18n::t(Key::PalettePlaceholder))
                .expect("the palette input should show")
                .bounds();
            ui.click("Esc")
                .expect("the palette should show its Esc key");

            let events: Vec<AppEvent> = ui.into_messages().collect();
            assert!(input.y > size::TITLEBAR + size::LINE, "{input:?}");
            assert!(
                events.iter().any(|event| matches!(
                    event,
                    AppEvent::Palette(PaletteIntent::Action(
                        PaletteAction::Close
                    ))
                )),
                "{mode:?}"
            );
        }
    }

    /// The notifications list lies over the palette and takes the press
    /// over it.
    #[test]
    fn the_notification_list_is_drawn_over_the_palette() {
        let mut app = app();
        show_toast(&mut app, "disk full");
        open_palette(&mut app);
        open_notification_list(&mut app);
        let mut ui = simulator(view(&app));

        ui.click("disk full").expect("the list row should show");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::Palette(_))),
            "{events:?}"
        );
    }

    /// The bell counts the unseen notifications; opening the list marks
    /// them seen, so its badge goes away.
    #[test]
    fn the_bell_shows_the_unseen_count_until_the_list_opens() {
        let mut app = app();
        show_toast(&mut app, "disk full");
        show_toast(&mut app, "busy");
        let mut ui = simulator(view(&app));
        let badge = ui.find("2").is_ok();
        drop(ui);

        open_notification_list(&mut app);

        let mut ui = simulator(view(&app));
        assert!(badge);
        assert!(ui.find("2").is_err());
    }

    /// The grips lie over every layer, so they stay away while the list is
    /// open, as for every menu.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_resize_grips_are_off_while_the_notification_list_is_open() {
        let resizes = |app: &App| {
            click_in_window(app, Point::new(1023.0, 400.0))
                .into_iter()
                .filter(|event| matches!(event, AppEvent::ResizeWindow(_)))
                .count()
        };
        let mut app = app();
        let without_list = resizes(&app);

        open_notification_list(&mut app);

        assert_eq!(without_list, 1);
        assert_eq!(resizes(&app), 0);
    }

    /// Settings is a dialog over every mode: it is drawn over the window and
    /// takes the click.
    #[test]
    fn settings_is_drawn_over_either_mode_and_takes_the_click() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = in_mode(mode);
            open_settings(&mut app);
            let mut ui = simulator(view(&app));

            ui.click("OTTY Light").expect("the dialog should show");

            let events: Vec<AppEvent> = ui.into_messages().collect();
            assert!(
                matches!(
                    events.as_slice(),
                    [AppEvent::Settings(SettingsIntent::ThemeSelected(
                        BuiltinTheme::OttyLight
                    ))]
                ),
                "{mode:?}: {events:?}"
            );
        }
    }

    /// Settings lies in the main area, so the buttons of the title bar can
    /// still close the window.
    #[test]
    fn the_window_buttons_stay_usable_under_settings() {
        let mut app = app();
        open_settings(&mut app);
        let close_window = Point::new(
            1024.0 - otty_ui_kit::spacing::SPACE_2 - size::CONTROL_MD / 2.0,
            size::TITLEBAR / 2.0,
        );

        let events = click_in_window(&app, close_window);

        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Chrome(ChromeIntent::CloseWindow)
        )));
    }

    /// The grips at the window edges lie over every layer, so they stay
    /// away while Settings waits for its answer.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_resize_grips_are_off_while_settings_is_open() {
        let resizes = |app: &App| {
            click_in_window(app, Point::new(1023.0, 400.0))
                .into_iter()
                .filter(|event| matches!(event, AppEvent::ResizeWindow(_)))
                .count()
        };
        let mut app = app();
        let without_settings = resizes(&app);

        open_settings(&mut app);

        assert_eq!(without_settings, 1);
        assert_eq!(resizes(&app), 0);
    }

    /// A point on the dimmed area of a dialog in the main area: left of the
    /// wide card, under the header.
    const ON_THE_DIM: Point = Point::new(100.0, 400.0);

    /// Presses the left button at `point` in the whole window and passes
    /// what the window sends through the app, as the runtime does.
    fn press_in_window(app: &mut App, point: Point) {
        for event in click_in_window(app, point) {
            let _ = app.update(Message::Input(event));
        }
    }

    /// Owner, 2026-11-09: a press on the dimmed area around Settings closes
    /// it like Esc, and the edits that were not saved are dropped.
    #[test]
    fn a_press_on_the_dimmed_area_closes_settings_and_drops_the_edits() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = in_mode(mode);
            open_settings(&mut app);
            edit_the_shell(&mut app, "/bin/edited");
            assert!(app.settings.has_unsaved_edits());

            press_in_window(&mut app, ON_THE_DIM);

            assert!(!app.settings.is_open(), "{mode:?}");
            assert!(!app.settings.has_unsaved_edits(), "{mode:?}");
        }
    }

    /// The dimmed area of Settings covers a menu under it: the press closes
    /// only Settings, and the menu stays open until the dialog is answered.
    #[test]
    fn a_press_on_the_dimmed_area_over_a_menu_keeps_the_menu() {
        let mut app = app();
        open_notification_list(&mut app);
        open_settings(&mut app);

        press_in_window(&mut app, ON_THE_DIM);

        assert!(!app.settings.is_open());
        assert!(app.notifications.is_list_open());
    }

    /// Owner, Q2 of the old app: a toast is no overlay; under a dialog's
    /// dimmed area its cross can not be pressed until the dialog is
    /// answered.
    #[test]
    fn a_dialog_lies_over_the_toasts() {
        let mut app = app();
        show_toast(&mut app, "Could not save the pinned modes");
        open_settings(&mut app);
        let mut ui = simulator(view(&app));

        let cross = toast_cross(&mut ui, "Could not save the pinned modes");
        ui.point_at(cross);
        let _ = ui.simulate(iced_test::simulator::click());

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(
            !events.iter().any(|event| matches!(
                event,
                AppEvent::Notifications(NotificationsIntent::DismissToast(_))
            )),
            "{events:?}"
        );
    }

    /// A dialog opened again starts at its top left, whatever it was
    /// scrolled to before it closed.
    #[test]
    fn settings_opened_again_starts_at_its_top() {
        let mut app = app();
        let mut frames = Frames::new();
        open_settings(&mut app);

        let _ = frames.dialog_frame(&app, None);
        let scrolled = frames.dialog_frame(&app, scroll_main_area());
        let _ = app.settings.reduce(SettingsIntent::Close, ());
        let _ = frames.frame(&app, None);
        open_settings(&mut app);
        let opened_again = frames.dialog_frame(&app, None);

        assert!(scrolled.x > 0.0 && scrolled.y > 0.0, "{scrolled:?}");
        assert_eq!(opened_again, iced::Vector::new(0.0, 0.0));
    }

    /// Typing and switching rebuild the dialog; it stays where it was
    /// scrolled.
    #[test]
    fn settings_built_again_keeps_its_scrolling() {
        let mut app = app();
        let mut frames = Frames::new();
        open_settings(&mut app);

        let _ = frames.dialog_frame(&app, None);
        let scrolled = frames.dialog_frame(&app, scroll_main_area());
        edit_the_shell(&mut app, "/bin/z");
        let after_typing = frames.dialog_frame(&app, None);
        let _ = app
            .settings
            .reduce(SettingsIntent::EqualizePanesToggled(false), ());
        let after_switch = frames.dialog_frame(&app, None);

        assert_eq!(scrolled, iced::Vector::new(30.0, 50.0));
        assert_eq!(after_typing, scrolled);
        assert_eq!(after_switch, scrolled);
    }
}
