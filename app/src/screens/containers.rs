//! The screen of Containers mode: the engine tree, the open list or the Run
//! form, the inspector, the status bar, the row menu and the dialogs.

use iced::{Element, Theme};

use crate::app::App;
use crate::app::view::{MainContent, ScreenId};
use crate::domain::mode::Mode;
use crate::router::AppEvent;
use crate::theme::ThemeProps;
use crate::widgets::containers::MainViewModel;
use crate::widgets::containers::actions::view::{
    confirm_dialog as containers_confirm, row_menu as containers_row_menu,
};
use crate::widgets::containers::create::view::form_dialog as containers_form;
use crate::widgets::containers::event::{ContainersEvent, ContainersIntent};
use crate::widgets::containers::run::view::{
    run_form as containers_run_form, run_panel as containers_run_panel,
};
use crate::widgets::containers::view::{
    engine_tree, inspector as containers_inspector,
    main_area as containers_main, status_bar as containers_status_bar,
};
use crate::widgets::modes::event::{ModesEvent, ModesIntent};

/// Left panel of Containers mode: the engine tree.
pub(crate) fn left_panel<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    engine_tree::view(engine_tree::EngineTreeProps {
        vm: app.widgets.containers.tree_vm(),
        theme: theme_props,
    })
    .map(|intent| AppEvent::Containers(ContainersEvent::Intent(intent)))
}

/// Main area of Containers mode: the Run container form while it is open,
/// else the open list of an engine or a card while there is none.
pub(crate) fn main<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> MainContent<'a> {
    let to_app_event =
        |intent| AppEvent::Containers(ContainersEvent::Intent(intent));

    if let Some(vm) = app.widgets.containers.run_form_vm() {
        return MainContent::Screen {
            id: ScreenId::RunForm,
            content: containers_run_form::view(
                containers_run_form::RunFormProps {
                    vm,
                    theme: theme_props,
                },
            )
            .map(ContainersIntent::Run)
            .map(to_app_event),
        };
    }

    let vm = app.widgets.containers.main_vm();
    let id = match &vm {
        MainViewModel::Table(_) => None,
        MainViewModel::Searching => Some(ScreenId::Searching),
        MainViewModel::NoEngine(card) => Some(ScreenId::NoEngine(card.case)),
    };
    let content = containers_main::view(containers_main::MainAreaProps {
        vm,
        theme: theme_props,
    })
    .map(to_app_event);

    match id {
        Some(id) => MainContent::Screen { id, content },
        None => MainContent::ScrollsOnItsOwn(content),
    }
}

/// Right panel of Containers mode: what the image sets while the Run form is
/// open, else the inspector of the selected row.
pub(crate) fn right_panel<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    if let Some(vm) = app.widgets.containers.image_panel_vm() {
        return containers_run_panel::view(
            containers_run_panel::RunPanelProps {
                vm,
                theme: theme_props,
            },
        )
        .map(ContainersIntent::Run)
        .map(|intent| AppEvent::Containers(ContainersEvent::Intent(intent)));
    }

    containers_inspector::view(containers_inspector::InspectorProps {
        vm: app.widgets.containers.inspector_vm(),
        tabs: app.widgets.containers.inspector_tabs_vm(),
        actions: app.widgets.containers.actions_vm(),
        theme: theme_props,
    })
    .map(|intent| AppEvent::Containers(ContainersEvent::Intent(intent)))
}

/// Status bar of Containers mode: the engine and the counts of the open list
/// on the left; on the right the sessions button, which switches to Terminal
/// mode.
pub(crate) fn status_bar<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    containers_status_bar::view(
        &app.widgets.containers.status_vm(),
        app.widgets.tabs.len(),
        env!("CARGO_PKG_VERSION"),
        AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
            Mode::Terminal,
        ))),
        theme_props,
    )
}

/// Overlays of Containers mode, bottom to top: the row menu, the
/// confirmation dialog and the create dialog. The row menu opens at the
/// pointer in window coordinates, so all of them lie over the whole window.
pub(crate) fn overlays<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent, Theme, iced::Renderer>> {
    let mut layers = Vec::new();

    if let Some(vm) = app.widgets.containers.row_menu_vm() {
        layers.push(
            containers_row_menu::view(containers_row_menu::RowMenuProps {
                vm,
                area: app.state.window_size,
                theme: theme_props,
            })
            .map(ContainersIntent::Actions),
        );
    }
    if let Some(vm) = app.widgets.containers.confirm_vm() {
        layers.push(
            containers_confirm::view(containers_confirm::ConfirmDialogProps {
                vm,
                theme: theme_props,
            })
            .map(ContainersIntent::Actions),
        );
    }
    if let Some(vm) = app.widgets.containers.form_vm() {
        layers.push(
            containers_form::view(containers_form::FormDialogProps {
                vm,
                theme: theme_props,
            })
            .map(ContainersIntent::Create),
        );
    }

    layers
        .into_iter()
        .map(|layer| {
            layer.map(|intent| {
                AppEvent::Containers(ContainersEvent::Intent(intent))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;
    use otty_ui_kit::components::menu;
    use otty_ui_kit::{size, spacing};

    use super::*;
    use crate::app::view::tests::{
        AppChange, Frames, SMALL_WINDOW, app_in, app_with_card,
        app_with_run_form, assert_each_screen_starts_at_its_top,
        card_area_scroller, containers_app_with_lists, denied_podman_search,
        offsets_through_rebuilds, press_and_move, right_panel_split,
        screen_scrollers, scroll_main_area, sent_a_panel_resize,
    };
    use crate::app::view::view;
    use crate::i18n::{self, Key};
    use crate::widgets::containers::run::event::RunIntent;
    use crate::widgets::containers::run::types::RunEdit;
    use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};

    #[test]
    fn containers_mode_shows_the_engine_tree_the_table_and_the_inspector() {
        use otty_containers::{EngineKind, fixtures};

        use crate::widgets::containers::event::ContainersIntent;
        use crate::widgets::containers::types::LoadedLists;

        let mut app = app_in(Mode::Containers);
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = app.widgets.containers.reduce(ContainersIntent::Enter);
        let _ =
            app.widgets
                .containers
                .reduce(ContainersIntent::DiscoveryFinished(
                    fixtures::discovery(&[EngineKind::Docker]),
                ));
        let _ = app
            .widgets
            .containers
            .reduce(ContainersIntent::ListsLoaded {
                engine: docker.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            });
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::ContainersImages))
            .expect("tree should list the images");
        ui.find("otty-fx-exit3")
            .expect("table should list containers");
        ui.click("otty-fx-exit3").expect("row should be clickable");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            page_intent(event),
            Some(ContainersIntent::SelectRow(5))
        )));
    }

    #[test]
    fn the_run_form_replaces_the_table_and_the_inspector() {
        use otty_containers::{EngineKind, fixtures};

        use crate::widgets::containers::event::ContainersIntent;
        use crate::widgets::containers::types::LoadedLists;

        let mut app = app_in(Mode::Containers);
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = app.widgets.containers.reduce(ContainersIntent::Enter);
        let _ =
            app.widgets
                .containers
                .reduce(ContainersIntent::DiscoveryFinished(
                    fixtures::discovery(&[EngineKind::Docker]),
                ));
        let _ = app
            .widgets
            .containers
            .reduce(ContainersIntent::ListsLoaded {
                engine: docker.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            });
        let _ = app.widgets.containers.reduce(ContainersIntent::OpenCreate);
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::ContainersRunSubtitle))
            .expect("main area should show the run form");
        ui.find(i18n::t(Key::ContainersRunNoImage))
            .expect("right panel should show the image panel");
        assert!(ui.find("otty-fx-exit3").is_err());
    }

    #[test]
    fn containers_mode_shows_the_row_menu_and_the_remove_dialog_over_the_table()
    {
        use otty_containers::{EngineKind, fixtures};

        use crate::widgets::containers::actions::event::ActionsIntent;
        use crate::widgets::containers::actions::types::RowAction;
        use crate::widgets::containers::event::ContainersIntent;
        use crate::widgets::containers::types::LoadedLists;

        let mut app = app_in(Mode::Containers);
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = app.widgets.containers.reduce(ContainersIntent::Enter);
        let _ =
            app.widgets
                .containers
                .reduce(ContainersIntent::DiscoveryFinished(
                    fixtures::discovery(&[EngineKind::Docker]),
                ));
        let _ = app
            .widgets
            .containers
            .reduce(ContainersIntent::ListsLoaded {
                engine: docker.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            });
        let _ = app
            .widgets
            .containers
            .reduce(ContainersIntent::OpenRowMenu {
                index: 5,
                position: Point::new(400.0, 200.0),
            });

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
            .find(i18n::t(Key::ContainersActionStart))
            .expect("menu item should show");
        menu_ui.point_at(first_item);
        let _ = menu_ui.simulate(iced_test::simulator::click());
        let events: Vec<AppEvent> = menu_ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Actions(ActionsIntent::Act(RowAction::Start))
            ))
        )));

        let _ = app.widgets.containers.reduce(ContainersIntent::Actions(
            ActionsIntent::Act(RowAction::Remove),
        ));
        let mut dialog_ui = simulator(view(&app));
        dialog_ui
            .find(i18n::t(Key::ContainersRemoveContainerText))
            .expect("the remove dialog should show");
        drop(dialog_ui);
        assert!(app.widgets.containers.has_overlay());

        let _ = app
            .widgets
            .containers
            .reduce(ContainersIntent::Actions(ActionsIntent::ConfirmCancelled));
        let _ = app.widgets.containers.reduce(ContainersIntent::Open(
            crate::widgets::containers::types::Place::new(
                docker.address().clone(),
                crate::widgets::containers::types::ResourceKind::Images,
            ),
        ));
        let _ = app.widgets.containers.reduce(ContainersIntent::OpenCreate);
        let mut pull_ui = simulator(view(&app));
        pull_ui
            .find(i18n::t(Key::ContainersFieldImage))
            .expect("the pull dialog should show");
        assert!(app.widgets.containers.has_overlay());
    }

    /// Height of a window where the Containers table has to scroll.
    const SHORT_WINDOW: iced::Size = iced::Size::new(1024.0, 250.0);

    /// Where the line of the `main | right panel` split starts.
    fn main_right_line(app: &App) -> f32 {
        app.widgets
            .sidebar
            .panes()
            .layout()
            .split_regions(size::LINE, 0.0, SHORT_WINDOW)
            .values()
            .map(|(axis, region, ratio)| {
                axis.split_line_bounds(*region, *ratio, size::LINE).x
            })
            .fold(0.0, f32::max)
    }

    /// Presses at `from` in a short window, moves along `path` and releases.
    /// Returns whether a panel resize was sent and how far the first table
    /// row moved up.
    fn drag_in_short_window(
        app: &App,
        from: Point,
        path: &[Point],
    ) -> (bool, f32) {
        const ROW: &str = "otty-fx-web-1";

        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            SHORT_WINDOW,
            view(app),
        );
        let row_y = |ui: &mut iced_test::Simulator<'_, AppEvent>| {
            let row = ui.find(ROW).expect("the first row should exist");
            row.visible_bounds().unwrap_or_default().y
        };
        let before = row_y(&mut ui);

        for (index, position) in std::iter::once(&from).chain(path).enumerate()
        {
            ui.point_at(*position);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::CursorMoved {
                    position: *position,
                },
            )]);
            if index == 0 {
                let _ = ui.simulate([iced::Event::Mouse(
                    iced::mouse::Event::ButtonPressed(
                        iced::mouse::Button::Left,
                    ),
                )]);
            }
        }
        let after = row_y(&mut ui);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left),
        )]);

        let resized = ui.into_messages().any(|event| {
            matches!(
                event,
                AppEvent::Sidebar(SidebarEvent::Intent(
                    SidebarIntent::Resized(_)
                ))
            )
        });

        (resized, before - after)
    }

    /// A height inside the table body of a [`SHORT_WINDOW`].
    const TABLE_Y: f32 = 190.0;

    #[test]
    fn dragging_the_panel_split_beside_the_table_bar_only_resizes() {
        let app = containers_app_with_lists();
        let line = main_right_line(&app);

        let (resized, scrolled) = drag_in_short_window(
            &app,
            Point::new(line + 0.5, TABLE_Y),
            &[
                Point::new(line - 40.0, TABLE_Y + 20.0),
                Point::new(line - 60.0, TABLE_Y + 40.0),
            ],
        );

        assert!(resized, "the panel should resize");
        assert_eq!(scrolled, 0.0, "the table should not scroll");
    }

    #[test]
    fn dragging_the_table_bar_beside_the_panel_split_only_scrolls() {
        let app = containers_app_with_lists();
        let line = main_right_line(&app);
        let bar = line - 2.0;

        let (resized, scrolled) = drag_in_short_window(
            &app,
            Point::new(bar, TABLE_Y),
            &[
                Point::new(bar, TABLE_Y + 20.0),
                Point::new(bar, TABLE_Y + 40.0),
            ],
        );

        assert!(!resized, "the panel should not resize");
        assert!(scrolled > 0.0, "the table should scroll, moved {scrolled}");
    }

    #[test]
    fn dragging_the_panel_split_across_the_table_does_not_scroll_it() {
        let app = containers_app_with_lists();
        let line = main_right_line(&app);

        let (resized, scrolled) = drag_in_short_window(
            &app,
            Point::new(line + 0.5, TABLE_Y),
            &[
                Point::new(line - 100.0, TABLE_Y),
                Point::new(line - 2.0, TABLE_Y),
                Point::new(line - 2.0, TABLE_Y + 40.0),
            ],
        );

        assert!(resized, "the panel should resize");
        assert_eq!(scrolled, 0.0, "the table should not scroll");
    }

    #[test]
    fn dragging_the_panel_split_across_a_narrow_run_form_does_not_scroll_it() {
        // Narrow and short, so the form overflows both ways under its bars.
        const WINDOW: iced::Size = iced::Size::new(800.0, 250.0);

        let mut app = containers_app_with_lists();
        app.state.window_size = WINDOW;
        let engine = otty_containers::fixtures::engine(
            otty_containers::EngineKind::Docker,
        );
        let _ = app.widgets.containers.reduce(ContainersIntent::Open(
            crate::widgets::containers::types::Place::new(
                engine.address().clone(),
                crate::widgets::containers::types::ResourceKind::Containers,
            ),
        ));
        let _ = app.widgets.containers.reduce(ContainersIntent::OpenCreate);
        let line = app
            .widgets
            .sidebar
            .panes()
            .layout()
            .split_regions(size::LINE, 0.0, WINDOW)
            .values()
            .map(|(axis, region, ratio)| {
                axis.split_line_bounds(*region, *ratio, size::LINE).x
            })
            .fold(0.0, f32::max);
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            WINDOW,
            view(&app),
        );
        let title = i18n::t(Key::ContainersRunContainer);
        let before = ui.find(title).expect("the form should show");

        let y = 150.0;
        for (index, x) in [line + 0.5, line - 100.0, line - 200.0, line - 2.0]
            .into_iter()
            .enumerate()
        {
            let position = Point::new(x, y);
            ui.point_at(position);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::CursorMoved { position },
            )]);
            if index == 0 {
                let _ = ui.simulate([iced::Event::Mouse(
                    iced::mouse::Event::ButtonPressed(
                        iced::mouse::Button::Left,
                    ),
                )]);
            }
        }
        let after = ui.find(title).expect("the form should show");
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left),
        )]);
        ui.point_at(Point::new(line - 100.0, y));
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Pixels { x: -10.0, y: 0.0 },
            },
        )]);
        let wheeled = ui.find(title).expect("the form should show");

        let resized = ui.into_messages().any(|event| {
            matches!(
                event,
                AppEvent::Sidebar(SidebarEvent::Intent(
                    SidebarIntent::Resized(_)
                ))
            )
        });
        assert!(resized, "the panel should resize");
        assert_eq!(before.visible_bounds(), after.visible_bounds());
        assert_ne!(
            before.visible_bounds(),
            wheeled.visible_bounds(),
            "the form should be able to scroll sideways"
        );
    }

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

    /// Dragging a panel edge over the card moves the panel and scrolls
    /// nothing (ticket 40), over the command and over the bars of the area.
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

    /// Neither split's grab zone reaches the part of a command bar that shows,
    /// even with the card wider than the main area, before it is scrolled: the
    /// padding of the box keeps it away.
    #[test]
    fn the_grab_zones_of_the_panel_splits_stay_off_a_command_bar() {
        // Enough room to see the command beside the empty side of the screen.
        const WINDOW: iced::Size = iced::Size::new(800.0, 320.0);

        let app = app_with_card(denied_podman_search());
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            WINDOW,
            view(&app),
        );
        let (area, _) = card_area_scroller(&mut ui);
        let (scroller, _, _) = command_scroller(&mut ui);
        let shown = scroller
            .intersection(&area)
            .expect("the command should show");
        let lines: Vec<f32> = app
            .widgets
            .sidebar
            .panes()
            .layout()
            .split_regions(size::LINE, 0.0, WINDOW)
            .values()
            .map(|(axis, region, ratio)| {
                axis.split_line_bounds(*region, *ratio, size::LINE).x
            })
            .collect();

        assert_eq!(lines.len(), 2);
        for line in lines {
            let zone_end = line + size::LINE + size::SPLIT_GRAB;

            assert!(
                zone_end <= shown.x || line >= shown.x + shown.width,
                "the zone at {line} reaches the bar {shown:?}"
            );
        }
    }

    /// The area's bar on the right stays clear of the right panel's grab zone:
    /// the zone starts at the split, on the panel's side.
    #[test]
    fn the_grab_zone_of_the_right_panel_stays_off_the_bar_of_the_card_area() {
        let app = app_with_card(denied_podman_search());
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            SMALL_WINDOW,
            view(&app),
        );
        let (area, _) = card_area_scroller(&mut ui);
        let line = right_panel_split(&app, SMALL_WINDOW);
        let x = area.x + area.width - 2.0;

        assert!(
            x < line,
            "the bar at {x} should be left of the split {line}"
        );
        press_and_move(
            &mut ui,
            &[Point::new(x, area.y + 10.0), Point::new(x, area.y + 40.0)],
        );

        assert!(!sent_a_panel_resize(ui));
    }

    /// The search every 5 seconds builds the view again; the card must stay
    /// where the user scrolled it (spec Decision 9).
    #[test]
    fn the_background_search_leaves_the_scrolled_card_where_it_is() {
        let mut app = app_with_card(denied_podman_search());
        let mut frames = Frames::new();
        let scrolled = iced::Vector::new(-30.0, -50.0);

        let start = frames.frame(&app, None);
        let (main_x, main_y) = (200.0, 150.0);
        let after_wheel =
            frames.frame(&app, Some((Point::new(main_x, main_y), scrolled)));
        let _ = app
            .widgets
            .containers
            .reduce(ContainersIntent::EngineSearchTick);
        let during_search = frames.frame(&app, None);
        let _ = app.widgets.containers.reduce(
            ContainersIntent::BackgroundSearchFinished(denied_podman_search()),
        );
        let after_search = frames.frame(&app, None);

        assert_eq!(start, iced::Vector::new(0.0, 0.0));
        assert_eq!(after_wheel, iced::Vector::new(30.0, 50.0));
        assert_eq!(during_search, after_wheel);
        assert_eq!(after_search, after_wheel);
    }

    /// Check again shows the searching card for a moment, and the card that
    /// comes back starts again at its top (spec Decision 9).
    #[test]
    fn check_again_starts_the_card_again_at_its_top() {
        let mut app = app_with_card(denied_podman_search());
        let mut frames = Frames::new();

        let _ = frames.frame(&app, None);
        let scrolled = frames.frame(&app, scroll_main_area());
        let _ = app.widgets.containers.reduce(ContainersIntent::CheckAgain);
        let searching = frames.frame(&app, None);
        let _ =
            app.widgets
                .containers
                .reduce(ContainersIntent::DiscoveryFinished(
                    denied_podman_search(),
                ));
        let after_search = frames.frame(&app, None);

        assert_eq!(scrolled, iced::Vector::new(30.0, 50.0));
        assert_eq!(searching, iced::Vector::new(0.0, 0.0));
        assert_eq!(after_search, iced::Vector::new(0.0, 0.0));
    }

    /// The Run form that waits through an outage comes back at its top, with
    /// what was typed; the card it gave way to starts at its top as well.
    #[test]
    fn the_run_form_and_the_card_of_an_outage_do_not_share_their_scrolling() {
        use otty_containers::{EngineKind, fixtures};

        let mut app = app_with_run_form();
        let _ = app.widgets.containers.reduce(ContainersIntent::Run(
            RunIntent::Edited(RunEdit::Image(String::from("kept:1"))),
        ));

        assert_each_screen_starts_at_its_top(
            app,
            |app| {
                let engine = fixtures::engine(EngineKind::Docker);
                let _ = app.widgets.containers.reduce(
                    ContainersIntent::EngineUnreachable(
                        engine.address().clone(),
                    ),
                );
                let _ = app.widgets.containers.reduce(
                    ContainersIntent::DiscoveryFinished(denied_podman_search()),
                );
            },
            |app| {
                let _ = app.widgets.containers.reduce(
                    ContainersIntent::DiscoveryFinished(fixtures::discovery(
                        &[EngineKind::Docker],
                    )),
                );
                assert_eq!(
                    app.widgets
                        .containers
                        .run_form_vm()
                        .map(|form| form.image.to_string()),
                    Some(String::from("kept:1")),
                );
            },
        );
    }

    /// A search that gets another answer shows another card: a new screen.
    #[test]
    fn another_answer_of_the_background_search_starts_the_new_card_at_its_top()
    {
        use otty_containers::fixtures;

        let mut app = app_with_card(denied_podman_search());
        let mut frames = Frames::new();

        let _ = frames.frame(&app, None);
        let scrolled = frames.frame(&app, scroll_main_area());
        let _ = app
            .widgets
            .containers
            .reduce(ContainersIntent::EngineSearchTick);
        let _ = app.widgets.containers.reduce(
            ContainersIntent::BackgroundSearchFinished(
                fixtures::discovery_with_cli(
                    &[],
                    Some("/usr/bin/docker"),
                    None,
                ),
            ),
        );
        let new_card = frames.frame(&app, None);

        assert_eq!(scrolled, iced::Vector::new(30.0, 50.0));
        assert_eq!(new_card, iced::Vector::new(0.0, 0.0));
    }

    #[test]
    fn the_run_form_built_again_keeps_its_scrolling() {
        let mut app = app_with_run_form();
        let edit = |edit: RunEdit| -> AppChange {
            Box::new(move |app| {
                let _ = app.widgets.containers.reduce(ContainersIntent::Run(
                    RunIntent::Edited(edit.clone()),
                ));
            })
        };

        let offsets = offsets_through_rebuilds(
            &mut app,
            vec![
                edit(RunEdit::Image(String::from("nginx:1"))),
                edit(RunEdit::AddPort),
                edit(RunEdit::PortHost(0, String::from("not a port"))),
            ],
        );

        assert_eq!(offsets[0], iced::Vector::new(30.0, 50.0));
        assert!(offsets.iter().all(|offset| *offset == offsets[0]));
    }

    /// The header of a table screen stays where it is and only the table
    /// scrolls, however narrow the area: its sticky header and its rows could
    /// not work in an outer scroller, where the height has no end.
    #[test]
    fn a_table_screen_stays_out_of_the_scrolling_of_a_screen() {
        const WINDOW: iced::Size = iced::Size::new(400.0, 300.0);
        const ROW: &str = "otty-fx-web-1";

        let mut app = containers_app_with_lists();
        app.state.window_size = WINDOW;
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleLeft);
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            WINDOW,
            view(&app),
        );
        let title = i18n::t(Key::ContainersContainers);

        let title_before = ui.find(title).expect("the title").visible_bounds();
        let row_before = ui.find(ROW).expect("the row").visible_bounds();
        let row_y = row_before.expect("the row should show").center_y();
        ui.point_at(Point::new(200.0, row_y));
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Pixels { x: 0.0, y: -10.0 },
            },
        )]);
        let title_after = ui.find(title).expect("the title").visible_bounds();
        let row_after = ui.find(ROW).expect("the row").visible_bounds();

        assert!(title_before.is_some(), "the title should show");
        assert_eq!(title_before, title_after);
        assert_ne!(row_before, row_after, "the table should scroll");
        assert_eq!(screen_scrollers(&app), 0);
    }

    #[test]
    fn in_focus_mode_the_panel_split_cannot_be_dragged() {
        let mut app = containers_app_with_lists();
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);
        let line = main_right_line(&app);

        let (resized, _) = drag_in_short_window(
            &app,
            Point::new(line + 0.5, TABLE_Y),
            &[Point::new(line - 100.0, TABLE_Y)],
        );

        assert!(!resized, "focus mode has no drag handles");
    }

    /// What the page on screen sent in `event`.
    fn page_intent(event: &AppEvent) -> Option<&ContainersIntent> {
        match event {
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::FromPage { intent, .. },
            )) => Some(intent),
            _ => None,
        }
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
            let _ = app.update(crate::app::Message::Input(event));
        }

        let mut ui = simulator(view(&app));
        let item = ui
            .find(i18n::t(Key::ContainersActionStop))
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
        let menu_left_min =
            button_right_min - spacing::SPACE_2 - menu::MENU_WIDTH;
        let menu_left_max =
            button_right_max - spacing::SPACE_2 - menu::MENU_WIDTH;
        assert!(
            item.x >= menu_left_min && item.x < menu_left_max + 2.0 * size::ROW,
            "the menu should end at the button, not in the middle of the \
             table: item {item:?}, menu left edge between {menu_left_min} \
             and {menu_left_max}"
        );
    }

    #[test]
    fn the_containers_sessions_button_switches_to_terminal_mode() {
        let app = app_in(Mode::Containers);
        let mut ui = simulator(view(&app));

        ui.click(i18n::status_sessions(0).as_str())
            .expect("sessions button should be visible");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Terminal
            )))
        )));
    }
}
