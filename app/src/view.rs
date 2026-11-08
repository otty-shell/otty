use iced::widget::{Space, column, container, mouse_area};
use iced::{Element, Length};
use otty_ui_kit::components::divider;

use super::{App, AppEvent};
#[cfg(not(target_os = "macos"))]
use crate::components::primitive::resize_grips;
use crate::domain::mode::Mode;
use crate::screens::{containers, over_main_area, terminal};
use crate::theme::ThemeProps;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::chrome::view::action_bar;
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

/// Render the root application view: the window content and the overlays
/// over it, in one order for every mode.
pub(crate) fn view(app: &App) -> Element<'_, AppEvent> {
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
    let mode_overlays: Element<'_, AppEvent> =
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
) -> Element<'a, AppEvent> {
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

    let mut main_area: Vec<Element<'_, AppEvent>> = vec![content_row.into()];
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
) -> Element<'a, AppEvent> {
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
) -> Element<'a, AppEvent> {
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
) -> Option<Element<'a, AppEvent>> {
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
) -> Option<Element<'a, AppEvent>> {
    let vm = app.widgets.palette.vm()?;

    Some(
        palette_view::view(palette_view::CommandPaletteProps {
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
) -> Option<Element<'a, AppEvent>> {
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
fn view_resize_grips(app: &App, mode: Mode) -> Element<'_, AppEvent> {
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
fn view_resize_grips(_app: &App, _mode: Mode) -> Element<'_, AppEvent> {
    container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;
    use otty_ui_kit::size;

    use super::*;
    use crate::i18n::{self, Key};
    use crate::testing::overlays::open_secret_prompt;
    use crate::testing::window::{
        Frames, SMALL_WINDOW, app_in, app_with_card, app_with_run_form,
        app_with_start_card, assert_each_screen_starts_at_its_top,
        containers_app_with_lists, denied_podman_search, open_error_tab,
        open_launch_editor, open_settings, screen_scrollers, scroll_main_area,
    };
    use crate::widgets::containers::event::ContainersIntent;
    use crate::widgets::modes::event::ModesIntent;
    use crate::widgets::quick_launch::event::QuickLaunchIntent;
    use crate::widgets::sidebar::types::HeaderMenu;
    use crate::widgets::tabs::TabsCtx;
    use crate::widgets::tabs::event::TabsIntent;

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

        // A dialog ignores the palette shortcut, so the palette opens here
        // the way a command opens it, past the input checks.
        let mut app = containers_app_with_a_dialog();
        let _ = app.update(crate::app::Message::Internal(AppEvent::Palette(
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
