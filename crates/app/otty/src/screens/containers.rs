//! The screen of Containers mode. The containers feature fills it: the engine
//! tree in the left panel, the lists or the Run container form in the main
//! area, the inspector in the right panel, the left side of the status bar,
//! and the row menu and the dialogs as the mode overlays.

use iced::Element;
use otty_app_domain::mode::Mode;
use otty_app_i18n as i18n;
use otty_app_modes::ModesIntent;
use otty_app_slots::{MainArea, Overlays, RightPanel, StatusBar as _};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};

use crate::app::App;
use crate::message::AppEvent;

/// Left panel of Containers mode under the switcher: the engine tree.
pub(crate) fn left_panel<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.containers.engine_tree(theme).map(AppEvent::Containers)
}

/// Main area of Containers mode: the Run container form while it is open,
/// else the open list of an engine or a card while there is none.
pub(crate) fn main<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.containers
        .main_area(theme)
        .into_element(theme)
        .map(AppEvent::Containers)
}

/// Right panel of Containers mode: what the image sets while the Run form is
/// open, else the inspector of the selected row.
pub(crate) fn right_panel<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.containers.right_panel(theme).map(AppEvent::Containers)
}

/// Status bar of Containers mode: the engine and the counts of the open
/// list on the left, from the containers feature; on the right the sessions
/// button, which switches to Terminal mode, and the OTTY version.
pub(crate) fn status_bar<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let mut bar = StatusBar::new(theme.theme.ui());
    for item in app.containers.status_bar() {
        bar = bar.left(item.map(AppEvent::Containers));
    }

    bar.right(StatusItem::button(
        i18n::status_sessions(app.tabs.len()),
        AppEvent::Modes(ModesIntent::Select(Mode::Terminal)),
    ))
    .right(StatusItem::text(format!(
        "OTTY {}",
        env!("CARGO_PKG_VERSION")
    )))
    .into()
}

/// Return whether the row menu, the confirmation dialog or a create dialog
/// of Containers mode is open.
pub(crate) fn has_overlay(app: &App) -> bool {
    app.containers.has_row_menu()
        || app.containers.has_confirm()
        || app.containers.has_create_dialog()
}

/// Overlays of Containers mode: the row menu, the confirmation dialog or the
/// create dialog, whichever is on top. The row menu opens at the pointer in
/// window coordinates, so they lie over the whole window.
pub(crate) fn overlays<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent>> {
    app.containers
        .overlay(theme)
        .map(|overlay| overlay.into_content().map(AppEvent::Containers))
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;
    use otty_app_containers::{
        ActionsIntent, ContainersIntent, Place, ResourceKind, RowAction,
    };
    use otty_app_slots::Feature;
    use otty_app_theme::theme::AppTheme;
    use otty_containers::{EngineKind, fixtures};
    use otty_ui_kit::{size, spacing};

    use super::*;
    use crate::dispatch;
    use crate::testing::frames::{
        SMALL_WINDOW, card_area_scroller, press_and_move, right_panel_split,
        sent_a_panel_resize,
    };
    use crate::testing::window::{
        app_with_card, containers_app_with_lists, denied_podman_search,
    };
    use crate::view::view;

    /// The scroller of the command on the card: where it is, how far it is
    /// scrolled sideways and whether the command is wider than it. Only a
    /// command block has a scroller of one control's height.
    fn command_scroller(
        ui: &mut iced_test::Simulator<'_, AppEvent>,
    ) -> (iced::Rectangle, f32, bool) {
        use iced_test::selector::Candidate;

        ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                bounds,
                content_bounds,
                translation,
                ..
            } if bounds.height == size::CONTROL_SM => Some((
                bounds,
                translation.x,
                content_bounds.width > bounds.width,
            )),
            _ => None,
        })
        .expect("the card should have a command")
    }

    /// Dragging a panel edge over the no-engine card moves the panel and
    /// scrolls nothing (ticket 40 of the old app), over the command and over
    /// the bars of the area.
    #[test]
    fn dragging_the_panel_split_across_the_card_does_not_scroll_it() {
        let app = app_with_card(denied_podman_search());
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            SMALL_WINDOW,
            view(&app),
        );
        let (area, offset) = card_area_scroller(&mut ui);
        let (scroller, command_offset, _) = command_scroller(&mut ui);
        let line = right_panel_split(&app, SMALL_WINDOW);
        let inside = area.x + area.width - 10.0;
        let text_y = scroller.center_y().min(area.y + area.height - 20.0);
        let bar_y = area.y + area.height - 2.0;

        press_and_move(
            &mut ui,
            &[
                Point::new(line + 0.5, text_y),
                Point::new(inside, text_y),
                Point::new(area.x + area.width - 2.0, area.y + 40.0),
                Point::new(area.x + area.width - 2.0, area.y + 120.0),
                Point::new(inside, bar_y),
                Point::new(area.x + 10.0, bar_y),
            ],
        );

        let (_, offset_after) = card_area_scroller(&mut ui);
        let (_, command_after, _) = command_scroller(&mut ui);
        assert_eq!(offset_after, offset, "the area should not scroll");
        assert_eq!(command_after, command_offset);
        assert!(sent_a_panel_resize(ui), "the panel should resize");
    }

    /// Dragging the bar of the no-engine card's area scrolls the area and
    /// moves no panel.
    #[test]
    fn dragging_a_bar_of_the_card_area_only_scrolls_it() {
        let app = app_with_card(denied_podman_search());
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            SMALL_WINDOW,
            view(&app),
        );
        let (area, _) = card_area_scroller(&mut ui);
        let x = area.x + area.width - 2.0;

        press_and_move(
            &mut ui,
            &[Point::new(x, area.y + 10.0), Point::new(x, area.y + 40.0)],
        );

        let (_, offset) = card_area_scroller(&mut ui);
        assert!(offset.y > 0.0, "the area should scroll down");
        assert!(!sent_a_panel_resize(ui), "the panel should not resize");
    }

    /// The page intent a press in the main area sent.
    fn page_intent(event: &AppEvent) -> Option<&ContainersIntent> {
        match event {
            AppEvent::Containers(ContainersIntent::FromPage {
                intent, ..
            }) => Some(intent),
            _ => None,
        }
    }

    #[test]
    fn containers_mode_shows_the_engine_tree_and_the_table() {
        let app = containers_app_with_lists();
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(i18n::Key::ContainersImages))
            .expect("tree should list the images");
        ui.find("otty-fx-exit3")
            .expect("table should list containers");
        ui.click("otty-fx-exit3").expect("row should be clickable");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            page_intent(event),
            Some(ContainersIntent::SelectRow(_))
        )));
    }

    #[test]
    fn the_run_form_replaces_the_table_and_the_inspector() {
        let mut app = containers_app_with_lists();
        let _ = app.containers.reduce(ContainersIntent::OpenCreate, ());
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(i18n::Key::ContainersRunSubtitle))
            .expect("main area should show the run form");
        ui.find(i18n::t(i18n::Key::ContainersRunNoImage))
            .expect("right panel should show the image panel");
        assert!(ui.find("otty-fx-exit3").is_err());
    }

    #[test]
    fn the_right_panel_shows_the_inspector_of_the_selected_row() {
        let mut app = containers_app_with_lists();
        let _shown = dispatch::dispatch(
            &mut app,
            AppEvent::Sidebar(crate::sidebar::SidebarIntent::RevealRight),
        );
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (1600.0, 900.0),
            view(&app),
        );

        ui.click(i18n::t(i18n::Key::ContainersTabLogs))
            .expect("the inspector should show its tabs");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersIntent::SelectInspectorTab(_))
        )));
    }

    #[test]
    fn containers_mode_shows_the_row_menu_and_the_remove_dialog_over_the_table()
    {
        let mut app = containers_app_with_lists();
        let docker = fixtures::engine(EngineKind::Docker);
        let _keyboard = app.containers.reduce(
            ContainersIntent::OpenRowMenu {
                index: 5,
                position: Point::new(400.0, 200.0),
            },
            (),
        );

        // The inspector shows a Start button too, under the menu's dismiss
        // layer; press the first menu item where the menu opened instead.
        let first_item = Point::new(
            400.0 + spacing::SPACE_2 + 40.0,
            200.0
                + spacing::SPACE_2
                + otty_ui_kit::components::menu::MENU_PADDING
                + size::ROW / 2.0,
        );
        let mut menu_ui = simulator(view(&app));
        menu_ui
            .find(i18n::t(i18n::Key::ContainersActionStart))
            .expect("menu item should show");
        menu_ui.point_at(first_item);
        let _ = menu_ui.simulate(iced_test::simulator::click());
        let events: Vec<AppEvent> = menu_ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersIntent::Actions(
                ActionsIntent::Act(RowAction::Start)
            ))
        )));

        let _ = app.containers.reduce(
            ContainersIntent::Actions(ActionsIntent::Act(RowAction::Remove)),
            (),
        );
        let mut dialog_ui = simulator(view(&app));
        dialog_ui
            .find(i18n::t(i18n::Key::ContainersRemoveContainerText))
            .expect("the remove dialog should show");
        drop(dialog_ui);
        assert!(has_overlay(&app));

        let _ = app.containers.reduce(
            ContainersIntent::Actions(ActionsIntent::ConfirmCancelled),
            (),
        );
        let _ = app.containers.reduce(
            ContainersIntent::Open(Place::new(
                docker.address().clone(),
                ResourceKind::Images,
            )),
            (),
        );
        let _ = app.containers.reduce(ContainersIntent::OpenCreate, ());
        let mut pull_ui = simulator(view(&app));
        pull_ui
            .find(i18n::t(i18n::Key::ContainersFieldImage))
            .expect("the pull dialog should show");
        assert!(has_overlay(&app));
    }

    /// Presses the first container row's "⋯" button in the window and
    /// returns the row's vertical center, the x of the press and the
    /// intents it sent.
    fn press_first_row_menu(app: &App) -> (f32, f32, Vec<AppEvent>) {
        let mut ui = simulator(view(app));
        let name = ui
            .find("otty-fx-web-1")
            .expect("the first container row should be visible");
        let row_center = name.bounds().center().y;

        // Sweep the row from the right until the "⋯" button takes the press.
        let mut x = 1000.0;
        while x > 600.0 {
            let mut probe = simulator(view(app));
            probe.point_at(Point::new(x, row_center));
            let _ = probe.simulate(iced_test::simulator::click());
            let events: Vec<AppEvent> = probe
                .into_messages()
                .filter(|event| {
                    matches!(
                        page_intent(event),
                        Some(ContainersIntent::RowMenuPressed { .. })
                    )
                })
                .collect();
            if !events.is_empty() {
                return (row_center, x, events);
            }
            x -= 2.0;
        }

        panic!("a press on the row button should open its menu");
    }

    #[test]
    fn the_row_menu_button_reports_its_bottom_right_corner_in_the_window() {
        let app = containers_app_with_lists();

        let (row_center, clicked_x, events) = press_first_row_menu(&app);

        let [event] = events.as_slice() else {
            panic!("one row menu intent expected");
        };
        let Some(ContainersIntent::RowMenuPressed { position, .. }) =
            page_intent(event)
        else {
            panic!("a row menu intent expected");
        };
        assert!(
            (position.y - (row_center + size::CONTROL_SM / 2.0)).abs() < 1.0,
            "the point should be the button's bottom edge, got {position:?} \
             for a row centered at {row_center}"
        );
        assert!(
            position.x >= clicked_x
                && position.x - clicked_x <= size::CONTROL_SM,
            "the point should be the button's right edge, got {position:?} \
             for a press at {clicked_x}"
        );
    }

    #[test]
    fn the_row_menu_opens_right_under_its_button() {
        let mut app = containers_app_with_lists();
        let (row_center, clicked_x, events) = press_first_row_menu(&app);
        for event in events {
            let _ = app.update(crate::message::Message::Input(event));
        }

        let mut ui = simulator(view(&app));
        let item = ui
            .find(i18n::t(i18n::Key::ContainersActionStop))
            .expect("the menu should list Stop for a running container")
            .bounds();

        // The first row of the menu sits just under the button, and the
        // menu's right edge is at the button's right edge less the margin.
        let button_bottom = row_center + size::CONTROL_SM / 2.0;
        assert!(
            item.y >= button_bottom && item.y < button_bottom + 6.0 * size::ROW,
            "the menu should start under the button, item {item:?}, button \
             bottom {button_bottom}"
        );
        let button_right_min = clicked_x;
        let button_right_max = clicked_x + size::CONTROL_SM;
        let menu_left_min = button_right_min
            - spacing::SPACE_2
            - otty_ui_kit::components::menu::MENU_WIDTH;
        let menu_left_max = button_right_max
            - spacing::SPACE_2
            - otty_ui_kit::components::menu::MENU_WIDTH;
        assert!(
            item.x >= menu_left_min && item.x < menu_left_max + 2.0 * size::ROW,
            "the menu should end at the button, not in the middle of the \
             table: item {item:?}, menu left edge between {menu_left_min} \
             and {menu_left_max}"
        );
    }

    #[test]
    fn the_run_form_built_again_keeps_its_scrolling() {
        use otty_app_containers::{RunEdit, RunIntent};

        use crate::testing::frames::{Frames, scroll_main_area};

        let mut app = containers_app_with_lists();
        let _ = app.containers.reduce(ContainersIntent::OpenCreate, ());
        let mut frames = Frames::new();

        let _ = frames.frame(&app, None);
        let mut offsets = vec![frames.frame(&app, scroll_main_area())];
        for edit in [
            RunEdit::Image(String::from("nginx:1")),
            RunEdit::AddPort,
            RunEdit::PortHost(0, String::from("not a port")),
        ] {
            let _ = app
                .containers
                .reduce(ContainersIntent::Run(RunIntent::Edited(edit)), ());
            offsets.push(frames.frame(&app, None));
        }

        assert_eq!(offsets[0], iced::Vector::new(30.0, 50.0));
        assert!(offsets.iter().all(|offset| *offset == offsets[0]));
    }

    /// A Containers dialog sends nothing from its dimmed area: a press there
    /// leaves it open (old overlay test).
    #[test]
    fn a_press_on_the_dimmed_area_of_a_containers_dialog_sends_nothing() {
        let mut app = containers_app_with_lists();
        crate::testing::window::open_confirm(&mut app);
        let mut ui = simulator(view(&app));

        ui.point_at(Point::new(5.0, 300.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::Containers(_))),
            "{events:?}"
        );
    }

    #[test]
    fn a_press_on_a_list_of_the_tree_opens_it() {
        let app = containers_app_with_lists();
        let mut ui = simulator(view(&app));

        ui.click(i18n::t(i18n::Key::ContainersImages))
            .expect("tree should list the images");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersIntent::Open(_))
        )));
    }

    #[test]
    fn before_an_engine_is_found_the_main_area_says_it_searches() {
        let mut app = crate::testing::window::app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Containers)),
        );
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(i18n::Key::ContainersSearching))
            .expect("the searching card should show");
    }

    #[test]
    fn with_an_engine_the_bar_shows_engine_counts_sessions_and_version() {
        let app = containers_app_with_lists();
        let theme = AppTheme::default();
        let containers = fixtures::resources(EngineKind::Docker);
        let running = containers
            .containers()
            .iter()
            .filter(|c| c.state() == otty_containers::ContainerState::Running)
            .count();
        let total = containers.containers().len();
        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        for label in [
            String::from("Docker 29.9.1"),
            format!("{total} containers, {running} running"),
            i18n::status_sessions(0),
            format!("OTTY {}", env!("CARGO_PKG_VERSION")),
        ] {
            ui.find(label.as_str())
                .expect("status bar item should be visible");
        }
        assert!(ui.find(i18n::t(i18n::Key::ContainersStatusMode)).is_err());
    }

    #[test]
    fn the_sessions_button_switches_to_terminal_mode() {
        let app = containers_app_with_lists();
        let theme = AppTheme::default();
        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        ui.click(i18n::status_sessions(0).as_str())
            .expect("button should be visible");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesIntent::Select(Mode::Terminal))]
        ));
    }
}
