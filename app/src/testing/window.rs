//! The whole window in view tests: an app in a given state, its frames one
//! after another, and what the tests look for in them. The tests of the root
//! view and of the mode screens share these.

use iced::Point;
use otty_ui_kit::size;

use crate::app::App;
use crate::app::view::view;
use crate::domain::mode::Mode;
use crate::router::AppEvent;
use crate::widgets::containers::event::ContainersIntent;
use crate::widgets::modes::ModesWidget;
use crate::widgets::modes::event::ModesIntent;
use crate::widgets::quick_launch::event::QuickLaunchIntent;
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::tabs::TabsCtx;
use crate::widgets::tabs::event::TabsIntent;

/// A window where the main area between the panels at their smallest is
/// too narrow and too low for the no-engine card. Panels dragged wide do
/// the same in a larger window.
pub(crate) const SMALL_WINDOW: iced::Size = iced::Size::new(400.0, 250.0);

/// One frame of the app after another, as the running app builds them: the
/// widget state carries over from one to the next.
pub(crate) struct Frames {
    renderer: iced::Renderer,
    cache: Option<iced_test::runtime::user_interface::Cache>,
}

impl Frames {
    pub(crate) fn new() -> Self {
        use iced_test::core::renderer::Headless;

        let renderer =
            iced_test::runtime::futures::futures::executor::block_on(
                <iced::Renderer as Headless>::new(
                    iced::Font::DEFAULT,
                    iced::Pixels(16.0),
                    None,
                ),
            )
            .expect("a headless renderer");

        Self {
            renderer,
            cache: Some(iced_test::runtime::user_interface::Cache::default()),
        }
    }

    /// Builds a frame of `app`, gives it the wheel at `pointer` if any, and
    /// returns how far the scroller of the screen is scrolled.
    pub(crate) fn frame(
        &mut self,
        app: &App,
        wheel: Option<(Point, iced::Vector)>,
    ) -> iced::Vector {
        use iced::advanced::widget::{Operation, operation};
        use iced_test::runtime::user_interface::UserInterface;
        use iced_test::selector::{Candidate, Selector};

        let mut ui = UserInterface::build(
            view(app),
            SMALL_WINDOW,
            self.cache.take().expect("the frames keep their cache"),
            &mut self.renderer,
        );
        if let Some((pointer, delta)) = wheel {
            let mut messages = Vec::new();
            let _ = ui.update(
                &[iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                    delta: iced::mouse::ScrollDelta::Pixels {
                        x: delta.x,
                        y: delta.y,
                    },
                })],
                iced::mouse::Cursor::Available(pointer),
                &mut self.renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
        }

        let mut find = (|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                content_bounds,
                translation,
                ..
            } if content_bounds.width == size::SCREEN_MIN => Some(translation),
            _ => None,
        })
        .find();
        ui.operate(&self.renderer, &mut operation::black_box(&mut find));
        let offset = match find.finish() {
            operation::Outcome::Some(Some(offset)) => offset,
            _ => panic!("the screen should have its scroller"),
        };

        self.cache = Some(ui.into_cache());

        offset
    }
}

/// A change to the app between two frames.
pub(crate) type AppChange = Box<dyn Fn(&mut App)>;

/// A new app in `mode`, with the modes as they are on a first start.
pub(crate) fn app_in(mode: Mode) -> App {
    let (mut app, _) = App::new();
    app.widgets.modes = ModesWidget::new();
    let _ = app.widgets.modes.reduce(ModesIntent::Select(mode));
    app
}

/// A no-engine card with these Docker and Podman commands found.
pub(crate) fn app_with_card(discovery: otty_containers::Discovery) -> App {
    let mut app = app_in(Mode::Containers);
    app.state.window_size = SMALL_WINDOW;
    let _ = app.widgets.containers.reduce(ContainersIntent::Enter);
    let _ = app
        .widgets
        .containers
        .reduce(ContainersIntent::DiscoveryFinished(discovery));

    app
}

/// The search that finds a refused Podman socket and nothing else.
pub(crate) fn denied_podman_search() -> otty_containers::Discovery {
    use otty_containers::{CandidateSource, fixtures};

    fixtures::denied_discovery_with_cli(
        fixtures::PODMAN_SOCKET,
        CandidateSource::Podman,
        None,
        Some("/usr/bin/podman"),
    )
}

/// The scroller of the main area: where it is and how far it is scrolled.
/// It is the only one whose content has the minimum width of a screen, and
/// the tests that use this one need a page that overflows it both ways.
pub(crate) fn card_area_scroller(
    ui: &mut iced_test::Simulator<'_, AppEvent>,
) -> (iced::Rectangle, iced::Vector) {
    use iced_test::selector::Candidate;

    ui.find(|candidate: Candidate<'_>| match candidate {
        Candidate::Scrollable {
            bounds,
            content_bounds,
            translation,
            ..
        } if content_bounds.width == size::SCREEN_MIN
            && content_bounds.height > bounds.height =>
        {
            Some((bounds, translation))
        },
        _ => None,
    })
    .expect("the card should overflow the main area both ways")
}

/// Presses at the first point, moves along the others and stays pressed.
pub(crate) fn press_and_move(
    ui: &mut iced_test::Simulator<'_, AppEvent>,
    points: &[Point],
) {
    for (index, position) in points.iter().enumerate() {
        ui.point_at(*position);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved {
                position: *position,
            },
        )]);
        if index == 0 {
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left),
            )]);
        }
    }
}

/// Whether the window sent a resize of a panel.
pub(crate) fn sent_a_panel_resize(
    ui: iced_test::Simulator<'_, AppEvent>,
) -> bool {
    ui.into_messages().any(|event| {
        matches!(
            event,
            AppEvent::Sidebar(SidebarEvent::Intent(SidebarIntent::Resized(_)))
        )
    })
}

/// The panel split at the right of the main area.
pub(crate) fn right_panel_split(app: &App, window: iced::Size) -> f32 {
    app.widgets
        .sidebar
        .panes()
        .layout()
        .split_regions(size::LINE, 0.0, window)
        .values()
        .map(|(axis, region, ratio)| {
            axis.split_line_bounds(*region, *ratio, size::LINE).x
        })
        .fold(0.0, f32::max)
}

/// Terminal mode with no tab open: the start card in a [`SMALL_WINDOW`].
pub(crate) fn app_with_start_card() -> App {
    let mut app = app_in(Mode::Terminal);
    app.state.window_size = SMALL_WINDOW;

    app
}

/// A wheel in the main area of a [`SMALL_WINDOW`] that moves a page 30 px
/// right and 50 px down.
pub(crate) fn scroll_main_area() -> Option<(Point, iced::Vector)> {
    Some((Point::new(200.0, 150.0), iced::Vector::new(-30.0, -50.0)))
}

/// Changes the quick launch widget as the running app does.
pub(crate) fn reduce_quick_launch(app: &mut App, intent: QuickLaunchIntent) {
    let settings = app.terminal_settings.clone();
    let ctx = crate::widgets::quick_launch::QuickLaunchCtx {
        terminal_settings: &settings,
        sidebar_cursor: Point::ORIGIN,
        sidebar_is_resizing: false,
    };

    let _ = app.widgets.quick_launch.reduce(intent, &ctx);
}

/// Opens Settings as a tab and returns its id.
pub(crate) fn open_settings(app: &mut App) -> u64 {
    let _ = app
        .widgets
        .tabs
        .reduce(TabsIntent::OpenSettingsTab, &TabsCtx { shell_name: "sh" });

    app.widgets.tabs.active_tab_id().expect("the form's tab")
}

/// Opens a launch editor as a tab, its state in place, and returns its id.
pub(crate) fn open_launch_editor(app: &mut App) -> u64 {
    let _ = app.widgets.tabs.reduce(
        TabsIntent::OpenWizardTab {
            title: String::from("New launch"),
            init: crate::domain::quick_launch::WizardTabInit::Create {
                parent_path: Vec::new(),
            },
        },
        &TabsCtx { shell_name: "sh" },
    );
    let tab_id = app.widgets.tabs.active_tab_id().expect("the form's tab");
    reduce_quick_launch(
        app,
        QuickLaunchIntent::WizardInitializeCreate {
            tab_id,
            parent_path: Vec::new(),
        },
    );

    tab_id
}

/// Opens the error tab of a launch, its state in place.
pub(crate) fn open_error_tab(app: &mut App) -> u64 {
    let (title, message) = (String::from("Failed"), String::from("Why"));
    let _ = app.widgets.tabs.reduce(
        TabsIntent::OpenErrorTab {
            title: title.clone(),
            message: message.clone(),
        },
        &TabsCtx { shell_name: "sh" },
    );
    let tab_id = app.widgets.tabs.active_tab_id().expect("the tab");
    reduce_quick_launch(
        app,
        QuickLaunchIntent::OpenErrorTab {
            tab_id,
            title,
            message,
        },
    );

    tab_id
}

/// The Run form over the lists of the fixture Docker engine, in a window
/// too small for it.
pub(crate) fn app_with_run_form() -> App {
    let mut app = containers_app_with_lists();
    app.state.window_size = SMALL_WINDOW;
    let engine =
        otty_containers::fixtures::engine(otty_containers::EngineKind::Docker);
    let _ = app.widgets.containers.reduce(ContainersIntent::Open(
        crate::widgets::containers::types::Place::new(
            engine.address().clone(),
            crate::widgets::containers::types::ResourceKind::Containers,
        ),
    ));
    let _ = app.widgets.containers.reduce(ContainersIntent::OpenCreate);

    app
}

/// The offset of a page that was just scrolled once and is built again
/// after each of `changes`, which must all leave the same screen on show.
pub(crate) fn offsets_through_rebuilds(
    app: &mut App,
    changes: Vec<AppChange>,
) -> Vec<iced::Vector> {
    let mut frames = Frames::new();

    let _ = frames.frame(app, None);
    let mut offsets = vec![frames.frame(app, scroll_main_area())];
    for change in changes {
        change(app);
        offsets.push(frames.frame(app, None));
    }

    offsets
}

/// `app` is scrolled, then `leave` puts another screen on show. That screen
/// must open at its top left, and it can be scrolled on its own. The first
/// screen, shown again by `back`, starts at its top left too.
pub(crate) fn assert_each_screen_starts_at_its_top(
    mut app: App,
    leave: impl Fn(&mut App),
    back: impl Fn(&mut App),
) {
    let mut frames = Frames::new();
    let top = iced::Vector::new(0.0, 0.0);

    let _ = frames.frame(&app, None);
    let first_scrolled = frames.frame(&app, scroll_main_area());
    leave(&mut app);
    let second_opened = frames.frame(&app, None);
    let second_scrolled = frames.frame(&app, scroll_main_area());
    back(&mut app);
    let first_again = frames.frame(&app, None);

    assert!(first_scrolled.x > 0.0 && first_scrolled.y > 0.0);
    assert_eq!(second_opened, top);
    assert!(second_scrolled.x > 0.0 && second_scrolled.y > 0.0);
    assert_eq!(first_again, top);
}

/// How many scrollers in `app` have content of the minimum width of a
/// screen.
pub(crate) fn screen_scrollers(app: &App) -> usize {
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        SMALL_WINDOW,
        view(app),
    );
    let mut found = 0;

    let _ = ui.find(|candidate: iced_test::selector::Candidate<'_>| {
        if let iced_test::selector::Candidate::Scrollable {
            content_bounds, ..
        } = candidate
            && content_bounds.width == size::SCREEN_MIN
        {
            found += 1;
        }
        None::<()>
    });

    found
}

/// Containers mode over the fixture Docker engine with its lists loaded, in
/// the window size the simulator draws.
pub(crate) fn containers_app_with_lists() -> App {
    use otty_containers::{EngineKind, fixtures};

    use crate::widgets::containers::types::LoadedLists;

    let mut app = app_in(Mode::Containers);
    // The simulator draws the window at its default size.
    app.state.window_size = iced::Size::new(1024.0, 768.0);
    let docker = fixtures::engine(EngineKind::Docker);
    let resources = fixtures::resources(EngineKind::Docker);
    let _ = app.widgets.containers.reduce(ContainersIntent::Enter);
    let _ = app
        .widgets
        .containers
        .reduce(ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
            EngineKind::Docker,
        ])));
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

    app
}
