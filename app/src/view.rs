use iced::widget::{Space, column, container, mouse_area};
use iced::{Element, Length, Theme};
use otty_ui_kit::components::divider;
use otty_ui_kit::components::screen_area::ScreenArea;

use super::{App, AppEvent};
#[cfg(not(target_os = "macos"))]
use crate::components::primitive::resize_grips;
use crate::domain::mode::Mode;
use crate::layout;
use crate::screens::{containers, terminal};
use crate::theme::ThemeProps;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::chrome::view::action_bar;
use crate::widgets::containers::types::NoEngineCase;
use crate::widgets::modes::event::ModesEvent;
use crate::widgets::modes::view::{mode_menu, mode_switcher};
use crate::widgets::palette::event::PaletteEvent;
use crate::widgets::palette::view::command_palette as palette_view;
use crate::widgets::quick_launch::event::QuickLaunchEvent;
use crate::widgets::quick_launch::view::secret_prompt;
use crate::widgets::sidebar::SidebarViewModel;
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::sidebar::types::SidebarPane;
use crate::widgets::sidebar::view::{focus_note, panels};

/// Which screen fills the main area. Each value is one screen to the user: a
/// change of value starts at the top left, and the same value built again keeps
/// its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenId {
    /// The start card: no tab is open.
    StartCard,
    /// The content of a tab: Settings, a launch editor, an error tab.
    Tab(u64),
    /// The Run container form.
    RunForm,
    /// The card while the engine search runs.
    Searching,
    /// The no-engine card of one diagnosis.
    NoEngine(NoEngineCase),
}

/// What fills the main area. A function that chooses it has to say which of the
/// two it is, so a new screen can not miss the rule of [`ScreenArea`]: anything
/// that is not a terminal or a table is a [`MainContent::Screen`].
pub(crate) enum MainContent<'a> {
    /// A screen. It keeps a minimum width and scrolls when there is too little
    /// room.
    Screen {
        id: ScreenId,
        content: Element<'a, AppEvent, Theme, iced::Renderer>,
    },
    /// The panes of a terminal or the table of a Containers list. They scroll on
    /// their own and fill a height they are given: a table's sticky header and
    /// its rows can not work in an outer scroller, where the height has no end.
    ScrollsOnItsOwn(Element<'a, AppEvent, Theme, iced::Renderer>),
}

impl<'a> MainContent<'a> {
    fn into_element(
        self,
        theme_props: ThemeProps<'a>,
    ) -> Element<'a, AppEvent, Theme, iced::Renderer> {
        match self {
            Self::Screen { id, content } => {
                ScreenArea::new(id, content, theme_props.theme.ui()).into()
            },
            Self::ScrollsOnItsOwn(content) => content,
        }
    }
}

/// Render the root application view: the window content and the overlays
/// over it, in one order for every mode.
pub(crate) fn view(app: &App) -> Element<'_, AppEvent, Theme, iced::Renderer> {
    let theme = app.theme_manager.current();
    let theme_props: ThemeProps<'_> = ThemeProps::new(theme);

    let sidebar_vm = app.widgets.sidebar.vm();
    let mode = app.widgets.modes.active();

    let window_content =
        view_window_content(app, mode, sidebar_vm, theme_props);
    let mode_overlays = match mode {
        Mode::Terminal => terminal::overlays(app, sidebar_vm, theme_props),
        Mode::Containers => containers::overlays(app, theme_props),
    };
    // The mode overlays share one layer, so that one of them opening or
    // closing does not move the layers above it.
    let mode_overlays: Element<'_, AppEvent, Theme, iced::Renderer> =
        iced::widget::Stack::with_children(mode_overlays)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    let modes_menu = view_modes_menu(app, theme_props);
    let palette = view_palette(app, theme_props);
    let secret_prompt = view_secret_prompt(app, theme_props);
    let resize_grips = view_resize_grips(app, mode);

    // The layers from bottom to top, the same in every mode. Each mode lays
    // its overlays out itself; the modes menu, the palette and the SSH secret
    // prompt lie in the main area, between the header and the status bar.
    let mut layers = vec![window_content, mode_overlays];
    layers.extend(modes_menu.map(over_main_area));
    layers.extend(palette.map(over_main_area));
    layers.extend(secret_prompt.map(over_main_area));
    layers.push(resize_grips);

    iced::widget::Stack::with_children(layers)
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
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    // The active mode decides what goes into each slot (spec Decision 5);
    // the sidebar keeps the panels open or closed and their widths.
    let content_row = panels::view(panels::PanelsProps {
        panes: app.widgets.sidebar.panes(),
        vm: sidebar_vm,
        theme: theme_props,
        content: |slot| match (slot, mode) {
            (SidebarPane::Left, _) => {
                view_left_panel(app, sidebar_vm, theme_props)
            },
            (SidebarPane::Main, Mode::Terminal) => {
                terminal::main(app, theme_props).into_element(theme_props)
            },
            (SidebarPane::Main, Mode::Containers) => {
                containers::main(app, theme_props).into_element(theme_props)
            },
            (SidebarPane::Right, Mode::Terminal) => {
                terminal::right_panel(app, theme_props)
            },
            (SidebarPane::Right, Mode::Containers) => {
                containers::right_panel(app, theme_props)
            },
        },
        on_intent: |intent| AppEvent::Sidebar(SidebarEvent::Intent(intent)),
    });

    let content_row = mouse_area(content_row).on_move(|position| {
        AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::WorkspaceCursorMoved { position },
        ))
    });

    let mut main_area: Vec<Element<'_, AppEvent, Theme, iced::Renderer>> =
        vec![content_row.into()];
    // Focus mode note
    if let Some(note) =
        focus_note::view(sidebar_vm.focus_note_opacity, theme_props)
    {
        main_area.push(note);
    }
    let main_area = iced::widget::Stack::with_children(main_area)
        .width(Length::Fill)
        .height(Length::Fill);

    let header = view_header(app, sidebar_vm, theme_props);
    let status_bar = match mode {
        Mode::Terminal => terminal::status_bar(app, theme_props),
        Mode::Containers => containers::status_bar(app, theme_props),
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
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let palette = theme_props.theme.ui();

    let action_bar = action_bar::view(action_bar::ActionBarProps {
        title: app.widgets.tabs.active_tab_title(),
        theme: theme_props,
        left_open: sidebar_vm.is_left_open,
        right_open: sidebar_vm.is_right_open,
        focus_mode: sidebar_vm.is_focus_mode,
        is_title_bar_pressed: app.widgets.chrome.is_title_bar_pressed(),
    })
    .map(|event| AppEvent::Chrome(ChromeEvent::Intent(event)));

    let separator = divider::horizontal(palette);

    column![action_bar, separator]
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
}

/// Left panel content: the mode switcher, then the active mode's content.
/// The panel and the switcher hide together.
fn view_left_panel<'a>(
    app: &'a App,
    sidebar_vm: SidebarViewModel,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let modes_vm = app.widgets.modes.vm();
    let mode = modes_vm.active;

    let switcher = mode_switcher::view(mode_switcher::ModeSwitcherProps {
        vm: modes_vm,
        theme: theme_props,
    })
    .map(|intent| AppEvent::Modes(ModesEvent::Intent(intent)));

    let content = match mode {
        Mode::Terminal => terminal::left_panel(app, sidebar_vm, theme_props),
        Mode::Containers => containers::left_panel(app, theme_props),
    };

    column![switcher, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The All modes menu under the switcher, while it is open.
fn view_modes_menu<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent, Theme, iced::Renderer>> {
    let modes_vm = app.widgets.modes.vm();
    if !modes_vm.is_menu_open {
        return None;
    }

    Some(
        mode_menu::view(mode_menu::ModeMenuProps {
            vm: modes_vm,
            theme: theme_props,
        })
        .map(|intent| AppEvent::Modes(ModesEvent::Intent(intent))),
    )
}

/// The command palette, while it is open.
fn view_palette<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent, Theme, iced::Renderer>> {
    let vm = app.widgets.palette.vm()?;

    Some(
        palette_view::view(palette_view::PaletteProps {
            vm,
            theme: theme_props,
        })
        .map(|intent| AppEvent::Palette(PaletteEvent::Intent(intent))),
    )
}

/// The SSH secret prompt, while a launch waits for its password or
/// passphrase.
fn view_secret_prompt<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent, Theme, iced::Renderer>> {
    let vm = app.widgets.quick_launch.secret_prompt_vm()?;

    Some(
        secret_prompt::view(secret_prompt::SecretPromptProps {
            vm,
            theme: theme_props,
        })
        .map(|event| AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event))),
    )
}

/// The grips that resize the window at its edges. While a menu, the SSH
/// secret prompt or a Containers overlay is open, an empty layer stands in
/// for them.
#[cfg(not(target_os = "macos"))]
fn view_resize_grips(
    app: &App,
    mode: Mode,
) -> Element<'_, AppEvent, Theme, iced::Renderer> {
    if app.widgets.sidebar.has_header_menu_open()
        || app.widgets.modes.is_menu_open()
        || app.widgets.quick_launch.has_context_menu()
        || app.widgets.quick_launch.has_secret_prompt()
        || app.widgets.terminal_workspace.has_any_context_menu()
        || (mode == Mode::Containers && app.widgets.containers.has_overlay())
    {
        return container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    resize_grips::view().map(|event| match event {
        resize_grips::ResizeGripEvent::Resize(dir) => {
            AppEvent::ResizeWindow(dir)
        },
    })
}

/// The system resizes the window on macOS, so an empty layer stands in for
/// the grips.
#[cfg(target_os = "macos")]
fn view_resize_grips(
    _app: &App,
    _mode: Mode,
) -> Element<'_, AppEvent, Theme, iced::Renderer> {
    container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Lay `overlay` out in the main area of a layer over the whole window: under
/// the header and over the status bar. It gets the size of the main area, and
/// the pointer positions it reports are in the main area too.
pub(crate) fn over_main_area(
    overlay: Element<'_, AppEvent, Theme, iced::Renderer>,
) -> Element<'_, AppEvent, Theme, iced::Renderer> {
    container(overlay)
        .padding(layout::main_area_padding())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
pub(crate) mod tests {
    use iced::Point;
    use iced_test::simulator;
    use otty_ui_kit::size;

    use super::*;
    use crate::i18n::{self, Key};
    use crate::widgets::containers::event::ContainersIntent;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::modes::event::ModesIntent;
    use crate::widgets::quick_launch::event::QuickLaunchIntent;
    use crate::widgets::sidebar::types::HeaderMenu;
    use crate::widgets::tabs::TabsCtx;
    use crate::widgets::tabs::event::TabsIntent;

    // The `pub(crate)` helpers are shared with the tests of the mode screens
    // in `screens/`.

    pub(crate) fn app_in(mode: Mode) -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        let _ = app.widgets.modes.reduce(ModesIntent::Select(mode));
        app
    }

    #[test]
    fn containers_mode_swaps_the_left_panel_main_area_and_status_bar() {
        let app = app_in(Mode::Containers);
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::ContainersSearching))
            .expect("containers main area should be visible");
        assert!(ui.find(i18n::t(Key::SessionsPanelTitle)).is_err());
        assert!(ui.find(i18n::t(Key::LaunchesPanelTitle)).is_err());
        assert!(ui.find(i18n::t(Key::EmptyTitle)).is_err());
    }

    /// A window where the main area between the panels at their smallest is
    /// too narrow and too low for the no-engine card. Panels dragged wide do
    /// the same in a larger window.
    pub(crate) const SMALL_WINDOW: iced::Size = iced::Size::new(400.0, 250.0);

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
                    iced::mouse::Event::ButtonPressed(
                        iced::mouse::Button::Left,
                    ),
                )]);
            }
        }
    }

    pub(crate) fn sent_a_panel_resize(
        ui: iced_test::Simulator<'_, AppEvent>,
    ) -> bool {
        ui.into_messages().any(|event| {
            matches!(
                event,
                AppEvent::Sidebar(SidebarEvent::Intent(
                    SidebarIntent::Resized(_)
                ))
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
                cache: Some(
                    iced_test::runtime::user_interface::Cache::default(),
                ),
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
                } if content_bounds.width == size::SCREEN_MIN => {
                    Some(translation)
                },
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

    /// The main area of the two modes is one place to iced: changing mode must
    /// not carry the scrolling of one card over to the other.
    #[test]
    fn the_start_card_and_the_no_engine_card_do_not_share_their_scrolling() {
        let mut app = app_with_card(denied_podman_search());
        let mut frames = Frames::new();
        let select = |app: &mut App, mode: Mode| {
            let _ = app.widgets.modes.reduce(ModesIntent::Select(mode));
        };

        let _ = frames.frame(&app, None);
        let no_engine_scrolled = frames.frame(&app, scroll_main_area());
        select(&mut app, Mode::Terminal);
        let start_opened = frames.frame(&app, None);
        let start_scrolled = frames.frame(&app, scroll_main_area());
        select(&mut app, Mode::Containers);
        let no_engine_again = frames.frame(&app, None);

        assert_eq!(no_engine_scrolled, iced::Vector::new(30.0, 50.0));
        assert_eq!(start_opened, iced::Vector::new(0.0, 0.0));
        assert!(start_scrolled.x > 0.0 && start_scrolled.y > 0.0);
        assert_eq!(no_engine_again, iced::Vector::new(0.0, 0.0));
    }

    /// Changes the quick launch widget as the running app does.
    pub(crate) fn reduce_quick_launch(
        app: &mut App,
        intent: QuickLaunchIntent,
    ) {
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

        app
    }

    /// A change to the app between two frames.
    pub(crate) type AppChange = Box<dyn Fn(&mut App)>;

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

    #[test]
    fn settings_and_the_no_engine_card_do_not_share_their_scrolling() {
        let mut app = app_with_card(denied_podman_search());
        let _ = open_settings(&mut app);
        let select = |app: &mut App, mode: Mode| {
            let _ = app.widgets.modes.reduce(ModesIntent::Select(mode));
        };

        assert_each_screen_starts_at_its_top(
            app,
            move |app| select(app, Mode::Terminal),
            move |app| select(app, Mode::Containers),
        );
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
                content_bounds,
                ..
            } = candidate
                && content_bounds.width == size::SCREEN_MIN
            {
                found += 1;
            }
            None::<()>
        });

        found
    }

    /// Every way the main area can be filled, whatever the screen, keeps one
    /// minimum: nothing here knows about it. The terminal and the tables have
    /// their own tests below.
    #[test]
    fn every_screen_of_the_main_area_keeps_the_one_minimum() {
        use otty_containers::fixtures;

        let settings = {
            let mut app = app_with_start_card();
            let _ = open_settings(&mut app);
            app
        };
        let launch_editor = {
            let mut app = app_with_start_card();
            let _ = open_launch_editor(&mut app);
            app
        };
        // A tab whose state is missing shows a line about it, in a screen too.
        let missing_editor = {
            let mut app = app_with_start_card();
            let _ = app.widgets.tabs.reduce(
                TabsIntent::OpenWizardTab {
                    title: String::from("New launch"),
                    init: crate::domain::quick_launch::WizardTabInit::Create {
                        parent_path: Vec::new(),
                    },
                },
                &TabsCtx { shell_name: "sh" },
            );
            app
        };
        let error_tab = {
            let mut app = app_with_start_card();
            let _ = open_error_tab(&mut app);
            app
        };
        let searching = {
            let mut app = app_in(Mode::Containers);
            app.state.window_size = SMALL_WINDOW;
            let _ = app.widgets.containers.reduce(ContainersIntent::Enter);
            app
        };
        let missing_error = {
            let mut app = app_with_start_card();
            let _ = app.widgets.tabs.reduce(
                TabsIntent::OpenErrorTab {
                    title: String::from("Failed"),
                    message: String::from("Why"),
                },
                &TabsCtx { shell_name: "sh" },
            );
            app
        };
        let missing_terminal = {
            let mut app = app_with_start_card();
            let _ = app.widgets.tabs.reduce(
                TabsIntent::OpenTerminalTab,
                &TabsCtx { shell_name: "sh" },
            );
            app
        };
        let no_tiles = app_with_card(fixtures::discovery_with_cli(
            &[],
            Some("/usr/bin/docker"),
            Some("/usr/bin/podman"),
        ));

        for (name, app) in [
            ("start card", app_with_start_card()),
            ("settings", settings),
            ("launch editor", launch_editor),
            ("missing editor", missing_editor),
            ("error tab", error_tab),
            ("missing error", missing_error),
            ("missing terminal", missing_terminal),
            ("searching card", searching),
            ("no-engine card", no_tiles),
            ("run form", app_with_run_form()),
        ] {
            assert_eq!(screen_scrollers(&app), 1, "{name}");
        }
    }

    pub(crate) fn containers_app_with_lists() -> App {
        use otty_containers::{EngineKind, fixtures};

        use crate::widgets::containers::event::ContainersIntent;
        use crate::widgets::containers::types::LoadedLists;

        let mut app = app_in(Mode::Containers);
        // The simulator draws the window at its default size.
        app.state.window_size = iced::Size::new(1024.0, 768.0);
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

        app
    }

    #[test]
    fn a_hidden_left_panel_hides_the_switcher() {
        let mut app = app_in(Mode::Terminal);
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleLeft);
        let mut ui = simulator(view(&app));

        assert!(!app.widgets.sidebar.vm().is_left_open);
        assert!(ui.find(i18n::t(Key::ModeTerminal)).is_err());
    }

    #[test]
    fn clicking_tabs_and_more_in_the_window_switches_modes_and_opens_the_menu()
    {
        // Centers of the first, second and third switcher tabs: under the
        // title bar and its line, inside the row padding and the group.
        let tab = |index: f32| {
            Point::new(
                8.0 + 3.0 + 15.0 + index * 32.0,
                size::TITLEBAR + size::LINE + 8.0 + 3.0 + 13.0,
            )
        };
        let mut app = app_in(Mode::Terminal);

        for (point, expected) in
            [(tab(1.0), Mode::Containers), (tab(0.0), Mode::Terminal)]
        {
            let events = click_in_window(&app, point);
            for event in events {
                let _ = app.update(crate::app::Message::Input(event));
            }
            assert_eq!(app.widgets.modes.active(), expected);
        }

        for event in click_in_window(&app, tab(2.0)) {
            let _ = app.update(crate::app::Message::Input(event));
        }
        assert!(app.widgets.modes.is_menu_open());
    }

    fn click_in_window(app: &App, point: Point) -> Vec<AppEvent> {
        let mut ui = simulator(view(app));
        ui.point_at(point);
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages()
            .filter(|event| matches!(event, AppEvent::Modes(_)))
            .collect()
    }

    #[test]
    fn an_open_modes_menu_is_drawn_over_the_window() {
        let mut app = app_in(Mode::Terminal);
        let _ = app.widgets.modes.reduce(ModesIntent::ToggleMenu);
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::ModesPinnedSection))
            .expect("all modes menu should be visible");
    }

    #[test]
    fn clicking_a_mode_name_in_the_open_menu_selects_it() {
        let mut app = app_in(Mode::Terminal);
        let _ = app.widgets.modes.reduce(ModesIntent::ToggleMenu);
        let mut ui = simulator(view(&app));

        ui.click(i18n::t(Key::ModeContainers))
            .expect("containers row should be visible");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Containers
            )))]
        ));
    }

    /// Containers mode with the Pull image dialog open over the images.
    fn containers_app_with_a_dialog() -> App {
        let mut app = containers_app_with_lists();
        let engine = otty_containers::fixtures::engine(
            otty_containers::EngineKind::Docker,
        );
        let _ = app.widgets.containers.reduce(ContainersIntent::Open(
            crate::widgets::containers::types::Place::new(
                engine.address().clone(),
                crate::widgets::containers::types::ResourceKind::Images,
            ),
        ));
        let _ = app.widgets.containers.reduce(ContainersIntent::OpenCreate);

        app
    }

    /// Opens the password prompt of an SSH launch, as the launch does when
    /// the server asks for one.
    fn open_secret_prompt(app: &mut App) {
        use crate::domain::exit_behavior::ExitBehavior;
        use crate::domain::quick_launch::{
            CommandSpec, QuickLaunch, SshCommand,
        };
        use crate::widgets::quick_launch::types::{
            QuickLaunchSetupOutcome, SshSecretKind, SshSecretRequest,
        };

        let launch = QuickLaunch::new(
            String::from("laptop"),
            CommandSpec::Ssh {
                ssh: SshCommand::new(
                    String::from("192.168.1.20"),
                    22,
                    Some(String::from("ubuntu")),
                    None,
                    Vec::new(),
                ),
            },
            ExitBehavior::default(),
        );

        reduce_quick_launch(
            app,
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::SecretRequired {
                    path: vec![String::from("laptop")],
                    launch_id: 0,
                    command: Box::new(launch),
                    request: SshSecretRequest::new(
                        SshSecretKind::Password,
                        String::from("ubuntu@192.168.1.20:22"),
                        false,
                    ),
                },
            ),
        );
    }

    /// An SSH launch asks for its password while a Containers dialog is open:
    /// the prompt is drawn on top and takes the click.
    #[test]
    fn the_secret_prompt_is_drawn_over_a_containers_dialog() {
        let mut app = containers_app_with_a_dialog();
        open_secret_prompt(&mut app);
        assert!(app.widgets.containers.has_overlay());

        let mut ui = simulator(view(&app));
        ui.click(i18n::t(Key::ButtonConnect))
            .expect("the prompt should show");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::SecretPromptSubmit
            ))
        )));
    }

    /// An SSH launch asks for its password while the New session menu is
    /// open: the prompt is drawn over the Terminal overlays and takes the
    /// click.
    #[test]
    fn the_secret_prompt_is_drawn_over_the_new_session_menu() {
        let mut app = app_in(Mode::Terminal);
        let _ = app
            .widgets
            .sidebar
            .reduce(SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions));
        open_secret_prompt(&mut app);
        assert!(app.widgets.sidebar.has_header_menu_open());

        let mut ui = simulator(view(&app));
        ui.find(i18n::t(Key::MenuSplitCurrentPane))
            .expect("the New session menu should show");
        ui.click(i18n::t(Key::ButtonConnect))
            .expect("the prompt should show");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::SecretPromptSubmit
            ))
        )));
    }

    #[test]
    fn the_palette_is_drawn_over_a_containers_dialog() {
        use otty_ui_kit::components::command_palette::PaletteAction;

        use crate::widgets::palette::event::PaletteIntent;

        let mut app = containers_app_with_a_dialog();
        let _ = app.update(crate::app::Message::Input(AppEvent::Palette(
            PaletteEvent::Intent(PaletteIntent::Open),
        )));
        assert!(app.widgets.palette.is_open());

        let mut ui = simulator(view(&app));
        ui.click("Esc")
            .expect("the palette should show its Esc key");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Action(
                PaletteAction::Close
            )))
        )));
    }

    #[test]
    fn the_modes_menu_is_drawn_over_the_containers_row_menu() {
        let mut app = containers_app_with_lists();
        let _ = app
            .widgets
            .containers
            .reduce(ContainersIntent::OpenRowMenu {
                index: 5,
                position: Point::new(400.0, 200.0),
            });
        let _ = app.widgets.modes.reduce(ModesIntent::ToggleMenu);

        let mut ui = simulator(view(&app));
        ui.click(i18n::t(Key::ModeTerminal))
            .expect("the modes menu should show");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Terminal
            )))
        )));
    }
}
