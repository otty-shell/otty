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
        Mode::Terminal => terminal::overlays(theme),
        Mode::Containers => containers::overlays(theme),
    };
    // The mode overlays share one layer, so that one of them opening or
    // closing does not move the layers above it.
    let mode_overlays: Element<'_, AppEvent> =
        Stack::with_children(mode_overlays)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    let modes_menu = view_modes_menu(app, theme);
    let notification_list = view_notification_list(app, theme);
    let resize_grips = view_resize_grips(app);

    // The header menus of the Sessions and Launches sections lie in the
    // layer of the modes menu, under it, anchored where they were opened
    // (issues 10 and 12). The palette (issue 08) goes between the modes
    // menu and the notifications list; Settings (issue 09) and the SSH
    // secret prompt (issue 12) follow the list, in this order. All of them
    // lie in the main area, between the header and the status bar.
    let mut layers = vec![window_content, mode_overlays];
    layers.extend(modes_menu.map(over_main_area));
    layers.extend(notification_list.map(over_main_area));
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
            (SidebarPane::Left, _) => view_left_panel(app, mode, theme),
            (SidebarPane::Main, Mode::Terminal) => terminal::main(theme),
            (SidebarPane::Main, Mode::Containers) => containers::main(theme),
            (SidebarPane::Right, Mode::Terminal) => {
                terminal::right_panel(theme)
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
        // The tabs feature (issue 10) gives the title of the active tab.
        title: None,
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
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let switcher = mode_switcher::view(mode_switcher::ModeSwitcherProps {
        vm: app.modes.vm(),
        theme,
    })
    .map(AppEvent::Modes);

    let content = match mode {
        Mode::Terminal => terminal::left_panel(theme),
        Mode::Containers => containers::left_panel(theme),
    };

    column![switcher, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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

/// The notifications list under the bell, while it is open, in either mode.
fn view_notification_list<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent>> {
    let list = app.notifications.overlay(theme)?;

    Some(list.into_content().map(AppEvent::Notifications))
}

/// The grips that resize the window at its edges. While a menu or the
/// notifications list is open, an empty layer stands in for them. The
/// features add their overlays to this check as they move in.
#[cfg(not(target_os = "macos"))]
fn view_resize_grips(app: &App) -> Element<'_, AppEvent> {
    if app.sidebar.has_header_menu_open()
        || app.modes.is_menu_open()
        || app.notifications.is_list_open()
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
    use otty_app_slots::Feature;
    use otty_ui_kit::size;

    use super::*;
    use crate::chrome::ChromeIntent;
    use crate::message::Message;
    use crate::modes::ModesIntent;
    use crate::testing::window::{app, open_notification_list, show_toast};

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
}
