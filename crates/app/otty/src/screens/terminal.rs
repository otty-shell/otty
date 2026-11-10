//! The screen of Terminal mode: the Sessions section of the tabs and the
//! Launches section of quick launch (issue 12) in the left panel, the start
//! card or the active tab in the main area (the panes of the terminal, and
//! the launch editor of issue 12), the Files section of the explorer in the
//! right panel, the status bar, the pane menu of the terminal and the menus
//! of the section headers.

use iced::widget::{container, mouse_area, pane_grid, text};
use iced::{Element, Length, alignment};
use otty_app_domain::commands::AppCommand;
use otty_app_slots::{
    LeftPanel, MainArea, MainContent, Overlays, RightPanel, SectionMessage,
    SectionState,
};
use otty_app_tabs::{TabContent, TabsIntent};
use otty_app_terminal::TerminalIntent;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::geometry::anchor_position;
use otty_ui_kit::spacing;

use super::{empty_place, over_main_area, start_card};
use crate::app::App;
use crate::chrome::view::status_bar;
use crate::chrome::{StatusBarInput, StatusBarViewModel};
use crate::message::AppEvent;
use crate::sidebar::types::{HeaderMenu, OpenHeaderMenu};
use crate::sidebar::view::{left_panel, new_session_menu};
use crate::sidebar::{SidebarIntent, SidebarViewModel};

/// The screens of the main area in Terminal mode. A change of screen starts
/// at the top left; the same screen built again keeps its scrolling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalScreen {
    /// The start card: no tab is open.
    StartCard,
}

/// Left panel of Terminal mode under the switcher: the Sessions section
/// over the Launches section, sharing the panel height (see
/// [`left_panel::view`]). The sidebar keeps which sections are open and
/// which header menu is open.
pub(crate) fn left_panel<'a>(
    app: &'a App,
    sidebar_vm: SidebarViewModel,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let sessions_state = SectionState::new(
        sidebar_vm.is_sessions_open,
        sidebar_vm
            .open_header_menu
            .is_some_and(|open| open.menu() == HeaderMenu::Sessions),
    );

    left_panel::view(left_panel::LeftPanelProps {
        vm: sidebar_vm,
        theme,
        sections: move || {
            let sessions = app.tabs.left_panel(theme, sessions_state);
            let sessions_content_height = sessions.content_height();

            left_panel::LeftPanelSections {
                sessions: sessions.into_content().map(sessions_event),
                sessions_content_height,
                // The quick launch feature (issue 12) puts the Launches
                // section here.
                launches: empty_place(),
                launches_content_height: 0.0,
            }
        },
    })
}

/// Main area of Terminal mode: the panes of a terminal tab, or the start
/// card while no tab is open. The quick launch feature (issue 12) shows the
/// form of a launch editor tab; until then that tab shows nothing here.
pub(crate) fn main<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    match app.tabs.active_tab_content() {
        Some(TabContent::Terminal) => app
            .terminal
            .main_area(theme)
            .into_element(theme)
            .map(AppEvent::Terminal),
        Some(TabContent::QuickLaunchWizard) => empty_place(),
        None => view_start_card(theme),
    }
}

/// Right panel of Terminal mode: the Files section of the explorer.
pub(crate) fn right_panel<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.explorer.right_panel(theme).map(AppEvent::Explorer)
}

/// Status bar of Terminal mode: the machine of the active tab (this
/// computer while no tab is open) and the folder of its focused pane on the
/// left, the pane and session counts and the version on the right.
pub(crate) fn status_bar<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let vm = StatusBarViewModel::new(StatusBarInput {
        machine_name: app.tabs.active_machine_name(),
        machine_target: app.tabs.active_machine_target(),
        folder: app.terminal.focused_folder_path(),
        pane_count: app.terminal.pane_count(),
        session_count: app.tabs.len(),
    });

    status_bar::view(&vm, env!("CARGO_PKG_VERSION"), theme)
}

/// Overlays of Terminal mode, bottom to top: the context menu of the
/// Launches tree (issue 12 puts it here) and the context menu of a
/// terminal pane. They open at the pointer in the main area, so they lie
/// there, under the header.
pub(crate) fn overlays<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent>> {
    let mut layers = Vec::new();

    if let Some(pane_menu) = app.terminal.overlay(theme) {
        layers.push(over_main_area(
            pane_menu.into_content().map(AppEvent::Terminal),
        ));
    }

    layers
}

/// The open menu of a section header with its dismiss layer, anchored at
/// the point where it was opened and kept inside the main area. The quick
/// launch feature (issue 12) brings the Launches menu; until then it shows
/// nothing.
pub(crate) fn header_menu<'a>(
    app: &'a App,
    open: OpenHeaderMenu,
    theme: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent>> {
    let (menu, menu_width, menu_height) = match open.menu() {
        HeaderMenu::Sessions => view_new_session_menu(app, theme),
        HeaderMenu::Launches => return None,
    };

    let anchor = anchor_position(
        open.cursor(),
        app.screen_size,
        menu_width,
        menu_height,
        spacing::SPACE_2,
    );
    let padding = iced::Padding {
        top: anchor.y,
        left: anchor.x,
        ..iced::Padding::ZERO
    };
    let positioned_menu = container(menu)
        .padding(padding)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Top);

    let dismiss = || AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss);
    let dismiss_layer = mouse_area(
        container(text("")).width(Length::Fill).height(Length::Fill),
    )
    .on_press(dismiss())
    .on_right_press(dismiss())
    .on_middle_press(dismiss())
    .on_move(|position| {
        AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { position })
    });

    Some(
        iced::widget::stack!(dismiss_layer, positioned_menu)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    )
}

/// Turn a press in the Sessions section into an app event: the title and
/// the chevron belong to the sidebar, the rest to the tabs.
fn sessions_event(message: SectionMessage<TabsIntent>) -> AppEvent {
    match message {
        SectionMessage::Toggle => {
            AppEvent::Sidebar(SidebarIntent::ToggleSessions)
        },
        SectionMessage::OpenMenu => AppEvent::Sidebar(
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        ),
        SectionMessage::Feature(intent) => AppEvent::Tabs(intent),
    }
}

/// The New session menu with its width and height: Split splits the
/// focused pane of a terminal tab to the right, and is off without one. The
/// quick launch feature (issue 12) gives the top-level launches; until then
/// no launch is listed.
fn view_new_session_menu<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> (Element<'a, AppEvent>, f32, f32) {
    let launches = Vec::new();
    let menu_height = new_session_menu::menu_height(launches.len());

    let menu = new_session_menu::view(new_session_menu::NewSessionMenuProps {
        theme,
        shell_name: app.shell_session.name(),
        on_new_terminal: AppEvent::Tabs(TabsIntent::OpenTerminalTab),
        on_split: app.terminal.focused_pane().map(|(tab_id, pane)| {
            AppEvent::Terminal(TerminalIntent::SplitPane {
                tab_id,
                pane,
                axis: pane_grid::Axis::Vertical,
            })
        }),
        launches,
        // The quick launch feature (issue 12) opens the launch editor; the
        // old menu sent its `HeaderCreateCommand` here.
        on_new_launch: AppEvent::Command(AppCommand::NewLaunch),
    });

    (menu, new_session_menu::NEW_SESSION_MENU_WIDTH, menu_height)
}

/// The start card of a window with no tab: a new terminal or a new launch.
fn view_start_card(theme: ThemeProps<'_>) -> Element<'_, AppEvent> {
    let card = start_card::view(start_card::EmptyStateProps {
        theme,
        on_new_terminal: AppEvent::Tabs(TabsIntent::OpenTerminalTab),
        // The quick launch feature (issue 12) opens the launch editor; the
        // old card sent its `HeaderCreateCommand` here.
        on_new_launch: AppEvent::Command(AppCommand::NewLaunch),
    });

    MainContent::Screen {
        id: TerminalScreen::StartCard,
        content: card,
    }
    .into_element(theme)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::Point;
    use iced_test::simulator;
    use otty_app_domain::quick_launch::WizardTabInit;
    use otty_app_explorer::{ExplorerIntent, FileNode};
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_slots::Feature;
    use otty_app_testing::mouse::middle_click;
    use otty_ui_kit::size;

    use super::*;
    use crate::dispatch;
    use crate::message::Message;
    use crate::testing::frames::{
        SMALL_WINDOW, card_area_scroller, left_panel_split, press_and_move,
        right_panel_split, screen_scrollers, sent_a_panel_resize,
    };
    use crate::testing::window::{app, open_terminal_tab};
    use crate::view::view;

    /// The right panel shows the Files section of the explorer: the
    /// followed folder and its entries, and a press on a row reaches the
    /// explorer.
    #[test]
    fn the_right_panel_shows_the_files_of_the_followed_folder() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();
        let root = PathBuf::from("/tmp/otty-project");
        let _read = dispatch::dispatch(
            &mut app,
            AppEvent::Explorer(ExplorerIntent::SyncRoot { cwd: root.clone() }),
        );
        let _none = dispatch::dispatch(
            &mut app,
            AppEvent::Explorer(ExplorerIntent::RootLoaded {
                root: root.clone(),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    root.join("main.rs"),
                    false,
                )],
            }),
        );

        let mut ui = simulator(right_panel(&app, ThemeProps::new(&theme)));

        ui.find(i18n::t(Key::FilesTitle))
            .expect("the Files title should show");
        ui.find("otty-project").expect("the folder should show");
        ui.click("main.rs").expect("the file row should show");
        let pressed = ui.into_messages().any(|event| {
            matches!(
                event,
                AppEvent::Explorer(ExplorerIntent::NodePressed { .. })
            )
        });
        assert!(pressed);
    }

    /// While no terminal folder is followed the section says there is no
    /// active folder.
    #[test]
    fn without_a_followed_folder_the_right_panel_says_so() {
        let app = app();
        let theme = app.theme_manager.current().clone();

        let mut ui = simulator(right_panel(&app, ThemeProps::new(&theme)));

        ui.find(i18n::t(Key::NoActiveFolder))
            .expect("no active folder should show");
    }

    #[test]
    fn without_tabs_the_status_bar_names_this_computer_and_the_version() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();

        {
            let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

            ui.find(i18n::t(Key::ThisComputer))
                .expect("this computer should be named");
            ui.find(format!("OTTY {}", env!("CARGO_PKG_VERSION")))
                .expect("the version should show");
        }

        let _none = dispatch::dispatch(
            &mut app,
            AppEvent::HostNameRead(Some(String::from("build-box"))),
        );
        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        ui.find("build-box").expect("the host name should show");
    }

    /// With tabs the status bar names the machine of the active tab and
    /// counts the sessions.
    #[test]
    fn the_status_bar_names_the_machine_of_the_active_tab_and_counts_sessions()
    {
        let mut app = app();
        let theme = app.theme_manager.current().clone();
        let _none = dispatch::dispatch(
            &mut app,
            AppEvent::HostNameRead(Some(String::from("build-box"))),
        );
        open_terminal_tab(&mut app);
        open_terminal_tab(&mut app);

        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        ui.find("build-box").expect("the machine should show");
        ui.find(i18n::status_sessions(2))
            .expect("the sessions should be counted");
    }

    /// The Sessions section lists the open tabs under the switcher; the
    /// Launches section of quick launch (issue 12) goes under it.
    #[test]
    fn the_left_panel_shows_the_sessions_of_the_tabs() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();
        let _none = dispatch::dispatch(
            &mut app,
            AppEvent::HostNameRead(Some(String::from("build-box"))),
        );
        open_terminal_tab(&mut app);

        let mut ui = simulator(left_panel(
            &app,
            app.sidebar.vm(),
            ThemeProps::new(&theme),
        ));

        ui.find(i18n::t(Key::SessionsPanelTitle))
            .expect("the Sessions title should show");
        ui.find("build-box").expect("the machine group should show");
        ui.click(app.shell_session.name())
            .expect("the tab row should show");
        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsIntent::ActivateTab { tab_id: 0 })]
        ));
    }

    /// The title of the Sessions section and the chevron of its New session
    /// button belong to the sidebar, which keeps the section open or closed
    /// and opens the header menu.
    #[test]
    fn the_sessions_title_and_chevron_reach_the_sidebar() {
        let toggle = sessions_event(SectionMessage::Toggle);
        let menu = sessions_event(SectionMessage::OpenMenu);
        let row =
            sessions_event(SectionMessage::Feature(TabsIntent::CloseTab {
                tab_id: 3,
            }));

        assert!(matches!(
            toggle,
            AppEvent::Sidebar(SidebarIntent::ToggleSessions)
        ));
        assert!(matches!(
            menu,
            AppEvent::Sidebar(SidebarIntent::HeaderMenuOpen(
                HeaderMenu::Sessions
            ))
        ));
        assert!(matches!(
            row,
            AppEvent::Tabs(TabsIntent::CloseTab { tab_id: 3 })
        ));
    }

    /// A Sessions section closed in the sidebar hides its rows and counts
    /// them in its header.
    #[test]
    fn a_closed_sessions_section_hides_its_rows() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();
        open_terminal_tab(&mut app);
        let _none = dispatch::dispatch(
            &mut app,
            AppEvent::Sidebar(SidebarIntent::ToggleSessions),
        );

        let mut ui = simulator(left_panel(
            &app,
            app.sidebar.vm(),
            ThemeProps::new(&theme),
        ));

        assert!(ui.find(app.shell_session.name()).is_err());
        assert!(ui.find("1").is_ok());
    }

    #[test]
    fn without_tabs_the_main_area_shows_the_start_card() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();

        {
            let mut ui = simulator(main(&app, ThemeProps::new(&theme)));
            ui.click(i18n::t(Key::EmptyNewTerminal))
                .expect("the start card should show");
            let events: Vec<AppEvent> = ui.into_messages().collect();
            assert!(matches!(
                events.as_slice(),
                [AppEvent::Tabs(TabsIntent::OpenTerminalTab)]
            ));
        }

        open_terminal_tab(&mut app);
        let mut ui = simulator(main(&app, ThemeProps::new(&theme)));

        assert!(ui.find(i18n::t(Key::EmptyTitle)).is_err());
    }

    /// Opens the New session menu at `cursor` in the main area of a 1024 by
    /// 768 window.
    fn open_sessions_menu_at(app: &mut App, cursor: Point) {
        let window = iced::Size::new(1024.0, 768.0);
        app.screen_size = crate::layout::screen_size_from_window(window);

        let _ = app.sidebar.reduce(
            SidebarIntent::WorkspaceCursorMoved { position: cursor },
            (),
        );
        let _ = app
            .sidebar
            .reduce(SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions), ());
    }

    #[test]
    fn the_sessions_menu_opens_at_the_pointer_in_the_main_area() {
        let cursor = Point::new(300.0, 200.0);
        let mut app = app();
        open_sessions_menu_at(&mut app, cursor);
        let menu_height = new_session_menu::menu_height(0);
        let anchor = anchor_position(
            cursor,
            app.screen_size,
            new_session_menu::NEW_SESSION_MENU_WIDTH,
            menu_height,
            spacing::SPACE_2,
        );

        let mut ui = simulator(view(&app));
        let label = format!(
            "{} ({})",
            i18n::t(Key::MenuNewTerminal),
            app.shell_session.name()
        );
        let item = ui
            .find(label.as_str())
            .expect("the menu should show")
            .bounds();

        let top = size::TITLEBAR + size::LINE + anchor.y;
        let bottom = top + menu_height;
        let right = anchor.x + new_session_menu::NEW_SESSION_MENU_WIDTH;
        assert!(
            item.x >= anchor.x && item.x < right,
            "the item {item:?} should start inside the menu from {} to {right}",
            anchor.x
        );
        assert!(
            item.y >= top && item.y + item.height <= bottom,
            "the item {item:?} should lie inside the menu from {top} to {bottom}"
        );
    }

    #[test]
    fn new_terminal_in_the_sessions_menu_opens_a_tab_and_closes_the_menu() {
        let mut app = app();
        open_sessions_menu_at(&mut app, Point::new(300.0, 200.0));
        let label = format!(
            "{} ({})",
            i18n::t(Key::MenuNewTerminal),
            app.shell_session.name()
        );

        let mut ui = simulator(view(&app));
        ui.click(label.as_str()).expect("the menu should show");
        let events: Vec<AppEvent> = ui.into_messages().collect();
        for event in events {
            let _ = app.update(Message::Input(event));
        }

        assert!(!app.sidebar.has_header_menu_open());
        assert_eq!(app.tabs.len(), 1);
    }

    /// A middle press away from the New session menu closes it, as a left
    /// press does.
    #[test]
    fn a_middle_press_outside_a_header_menu_closes_it() {
        let mut app = app();
        open_sessions_menu_at(&mut app, Point::new(300.0, 200.0));

        let mut ui = simulator(view(&app));
        ui.point_at(Point::new(900.0, 600.0));
        let _ = ui.simulate(middle_click());

        // The layer also reports where the pointer is.
        let events: Vec<AppEvent> = ui
            .into_messages()
            .filter(|event| {
                !matches!(
                    event,
                    AppEvent::Sidebar(
                        SidebarIntent::WorkspaceCursorMoved { .. }
                    )
                )
            })
            .collect();
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss)]
        ));
    }

    /// A middle press on a Sessions row under an open header menu only
    /// closes the menu: the press does not reach the row, so its tab stays
    /// open.
    #[test]
    fn a_middle_press_on_a_sessions_row_under_a_header_menu_keeps_the_tab() {
        let mut app = app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Tabs(TabsIntent::OpenWizardTab {
                title: String::from("deploy"),
                init: WizardTabInit::Create {
                    parent_path: Vec::new(),
                },
            }),
        );
        // Another tab is active, so "deploy" shows only in its row.
        open_terminal_tab(&mut app);
        open_sessions_menu_at(&mut app, Point::new(600.0, 400.0));

        let mut ui = simulator(view(&app));
        let row = ui.find("deploy").expect("the row should show").bounds();
        ui.point_at(row.center());
        let _ = ui.simulate(middle_click());

        let events: Vec<AppEvent> = ui.into_messages().collect();
        for event in events {
            let _ = app.update(Message::Input(event));
        }

        assert!(!app.sidebar.has_header_menu_open());
        assert_eq!(app.tabs.len(), 2);
    }

    /// A terminal tab fills the main area with its panes, each with a
    /// header that names the program and the machine.
    #[test]
    fn a_terminal_tab_fills_the_main_area_with_its_panes() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();
        let _none = dispatch::dispatch(
            &mut app,
            AppEvent::HostNameRead(Some(String::from("build-box"))),
        );
        open_terminal_tab(&mut app);

        let mut ui = simulator(main(&app, ThemeProps::new(&theme)));

        assert!(ui.find("build-box").is_ok());
        assert!(ui.find(app.shell_session.name()).is_ok());
        assert!(ui.find(i18n::t(Key::EmptyTitle)).is_err());
    }

    /// The status bar counts the panes of the tab in front.
    #[test]
    fn the_status_bar_counts_the_panes_of_the_tab_in_front() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();
        open_terminal_tab(&mut app);
        let _split = dispatch::dispatch(
            &mut app,
            AppEvent::Command(AppCommand::SplitRight),
        );

        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        assert!(ui.find(i18n::status_panes(2)).is_ok());
    }

    /// Split in the New session menu splits the focused pane; without a
    /// terminal tab it does nothing.
    #[test]
    fn the_new_session_menu_splits_the_focused_pane() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();
        let pressed = |app: &App| {
            let (menu, _, _) =
                view_new_session_menu(app, ThemeProps::new(&theme));
            let mut ui = simulator(menu);
            let _ = ui.click(i18n::t(Key::MenuSplitCurrentPane));

            ui.into_messages().any(|event| {
                matches!(
                    event,
                    AppEvent::Terminal(TerminalIntent::SplitPane { .. })
                )
            })
        };
        let without_a_tab = pressed(&app);

        open_terminal_tab(&mut app);

        assert!(!without_a_tab);
        assert!(pressed(&app));
    }

    /// The panes of a terminal fill the height they get and scroll on their
    /// own, so no screen scroller is around them.
    #[test]
    fn a_terminal_tab_stays_out_of_the_scrolling_of_a_screen() {
        let mut app = app();
        let with_the_start_card = screen_scrollers(&app);

        open_terminal_tab(&mut app);

        assert_eq!(with_the_start_card, 1);
        assert_eq!(screen_scrollers(&app), 0);
    }

    /// Both panel splits, dragged over the start card and over its bars,
    /// move the panel and scroll nothing (ticket 40 of the old app).
    #[test]
    fn dragging_a_panel_split_across_the_start_card_does_not_scroll_it() {
        let app = app();
        let right = right_panel_split(&app, SMALL_WINDOW);
        let left = left_panel_split(&app, SMALL_WINDOW);

        for split in [left, right] {
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings::default(),
                SMALL_WINDOW,
                view(&app),
            );
            let (area, offset) = card_area_scroller(&mut ui);
            let (right_end, bottom) =
                (area.x + area.width - 2.0, area.y + area.height - 2.0);

            press_and_move(
                &mut ui,
                &[
                    Point::new(split + 0.5, area.center_y()),
                    area.center(),
                    Point::new(right_end, area.y + 40.0),
                    Point::new(right_end, area.y + 120.0),
                    Point::new(area.center_x(), bottom),
                    Point::new(area.x + 10.0, bottom),
                ],
            );

            let (_, after) = card_area_scroller(&mut ui);
            assert_eq!(after, offset, "the card should not scroll");
            assert!(sent_a_panel_resize(ui), "the panel should resize");
        }
    }

    #[test]
    fn dragging_a_bar_of_the_start_card_only_scrolls_it() {
        let app = app();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            SMALL_WINDOW,
            view(&app),
        );
        let (area, _) = card_area_scroller(&mut ui);
        let right_end = area.x + area.width - 2.0;
        let bottom = area.y + area.height - 2.0;

        press_and_move(
            &mut ui,
            &[
                Point::new(right_end, area.y + 10.0),
                Point::new(right_end, area.y + 40.0),
            ],
        );
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left),
        )]);
        press_and_move(
            &mut ui,
            &[
                Point::new(area.x + 20.0, bottom),
                Point::new(area.x + 60.0, bottom),
            ],
        );

        let (_, offset) = card_area_scroller(&mut ui);
        assert!(offset.y > 0.0, "the card should scroll down");
        assert!(offset.x > 0.0, "the card should scroll sideways");
        assert!(!sent_a_panel_resize(ui), "the panel should not resize");
    }

    /// In the real window, between the panels, each tile comes into view
    /// inside the main area and does what it says.
    #[test]
    fn in_a_small_window_each_tile_of_the_start_card_can_be_pressed() {
        let app = app();

        for (key, is_expected) in [
            (
                Key::EmptyNewTerminal,
                (|event: &AppEvent| {
                    matches!(event, AppEvent::Tabs(TabsIntent::OpenTerminalTab))
                }) as fn(&AppEvent) -> bool,
            ),
            (Key::EmptyNewLaunch, |event: &AppEvent| {
                matches!(event, AppEvent::Command(AppCommand::NewLaunch))
            }),
        ] {
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings::default(),
                SMALL_WINDOW,
                view(&app),
            );
            let (area, _) = card_area_scroller(&mut ui);

            ui.point_at(area.center());
            let _ =
                ui.simulate([iced::Event::Window(
                    iced::window::Event::RedrawRequested(
                        iced::time::Instant::now(),
                    ),
                )]);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::WheelScrolled {
                    delta: iced::mouse::ScrollDelta::Pixels {
                        x: 0.0,
                        y: -10_000.0,
                    },
                },
            )]);
            let label = i18n::t(key);
            let shown = ui
                .find(label)
                .expect("the tile should exist")
                .visible_bounds()
                .expect("the tile should come into view");
            assert!(area.contains(shown.center()), "{label}: {shown:?}");

            ui.click(label).expect("the tile should be in reach");

            let pressed = ui.into_messages().filter(is_expected).count();
            assert_eq!(pressed, 1, "{label}");
        }
    }
}
