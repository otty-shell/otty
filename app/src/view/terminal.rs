//! The screen of Terminal mode: the Sessions and Launches sections, the active
//! tab, the Files section, the status bar and the context menus.

use iced::widget::{container, text};
use iced::{Element, Length, Theme, alignment};

use super::{MainContent, ScreenId};
use crate::app::App;
use crate::i18n::{self, Key};
use crate::router::AppEvent;
use crate::theme::ThemeProps;
use crate::widgets::chrome::view::status_bar;
use crate::widgets::chrome::{StatusBarInput, StatusBarViewModel};
use crate::widgets::explorer::event::ExplorerEvent;
use crate::widgets::explorer::view::sidebar_tree;
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::quick_launch::view::{
    context_menu as ql_context_menu, error_tab, sidebar_panel, wizard_form,
};
use crate::widgets::settings::event::SettingsEvent;
use crate::widgets::settings::view::settings_form;
use crate::widgets::sidebar::SidebarViewModel;
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::sidebar::types::HeaderMenu;
use crate::widgets::sidebar::view::left_panel;
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::tabs::types::{TabContent, TabMachine};
use crate::widgets::tabs::view::{empty_state, session_list};
use crate::widgets::terminal_workspace::event::TerminalWorkspaceEvent;
use crate::widgets::terminal_workspace::view::{
    pane_context_menu as terminal_pane_context_menu,
    pane_grid as terminal_pane_grid,
};

/// Left panel of Terminal mode: the Sessions section over the Launches
/// section, sharing the panel height (see [`left_panel::view`]).
pub(super) fn left_panel<'a>(
    app: &'a App,
    sidebar_vm: SidebarViewModel,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let host_name = app.host_name.as_deref();

    left_panel::view(left_panel::LeftPanelProps {
        vm: sidebar_vm,
        theme: theme_props,
        sections: move || {
            let sessions_vm = app.widgets.tabs.vm(host_name);
            let sessions_content_height =
                session_list::content_height(&sessions_vm);
            let sessions = session_list::view(session_list::SessionListProps {
                vm: sessions_vm,
                theme: theme_props,
                is_open: sidebar_vm.is_sessions_open,
                is_menu_open: sidebar_vm
                    .open_header_menu
                    .is_some_and(|open| open.menu() == HeaderMenu::Sessions),
                on_toggle: AppEvent::Sidebar(SidebarEvent::Intent(
                    SidebarIntent::ToggleSessions,
                )),
                on_menu: AppEvent::Sidebar(SidebarEvent::Intent(
                    SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
                )),
                on_intent: |intent| AppEvent::Tabs(TabsEvent::Intent(intent)),
            });

            let launches_vm = app.widgets.quick_launch.vm();
            let launches_content_height =
                sidebar_panel::content_height(&launches_vm);
            let launches =
                sidebar_panel::view(sidebar_panel::SidebarPanelProps {
                    vm: launches_vm,
                    theme: theme_props,
                    is_open: sidebar_vm.is_launches_open,
                    is_menu_open: sidebar_vm.open_header_menu.is_some_and(
                        |open| open.menu() == HeaderMenu::Launches,
                    ),
                    on_toggle: AppEvent::Sidebar(SidebarEvent::Intent(
                        SidebarIntent::ToggleLaunches,
                    )),
                    on_menu: AppEvent::Sidebar(SidebarEvent::Intent(
                        SidebarIntent::HeaderMenuOpen(HeaderMenu::Launches),
                    )),
                    on_intent: |intent| {
                        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent))
                    },
                });

            left_panel::LeftPanelSections {
                sessions,
                sessions_content_height,
                launches,
                launches_content_height,
            }
        },
    })
}

/// Main area of Terminal mode: the content of the active tab, or the start
/// card while no tab is open.
pub(super) fn main<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> MainContent<'a> {
    let active_tab_id = app.widgets.tabs.active_tab_id();
    let active_tab_content = app.widgets.tabs.active_tab_content();

    let (id, content) = match (active_tab_id, active_tab_content) {
        (Some(tab_id), Some(TabContent::Terminal)) => {
            return view_terminal_tab(app, tab_id, theme_props);
        },
        (Some(tab_id), Some(TabContent::Settings)) => (
            ScreenId::Tab(tab_id),
            settings_form::view(settings_form::SettingsFormProps {
                vm: app.widgets.settings.vm(),
                theme: theme_props,
            })
            .map(|event| AppEvent::Settings(SettingsEvent::Intent(event))),
        ),
        (Some(tab_id), Some(TabContent::QuickLaunchWizard)) => (
            ScreenId::Tab(tab_id),
            match app.widgets.quick_launch.wizard_vm(tab_id) {
                Some(vm) => wizard_form::view(wizard_form::WizardFormProps {
                    tab_id,
                    vm,
                    theme: theme_props,
                })
                .map(|event| {
                    AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event))
                }),
                None => missing_tab_state(i18n::t(
                    Key::QuickLaunchEditorNotInitialized,
                )),
            },
        ),
        (Some(tab_id), Some(TabContent::QuickLaunchError)) => (
            ScreenId::Tab(tab_id),
            match app.widgets.quick_launch.error_tab_vm(tab_id) {
                Some(vm) => error_tab::view(error_tab::ErrorTabProps {
                    vm,
                    theme: theme_props,
                })
                .map(|event| {
                    AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event))
                }),
                None => missing_tab_state(i18n::t(
                    Key::QuickLaunchErrorPayloadMissing,
                )),
            },
        ),
        _ => (
            ScreenId::StartCard,
            empty_state::view(empty_state::EmptyStateProps {
                theme: theme_props,
                on_new_terminal: AppEvent::Tabs(TabsEvent::Intent(
                    TabsIntent::OpenTerminalTab,
                )),
                on_new_launch: AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                    QuickLaunchIntent::HeaderCreateCommand,
                )),
            }),
        ),
    };

    MainContent::Screen { id, content }
}

/// Right panel of Terminal mode: the Files section.
pub(super) fn right_panel<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    sidebar_tree::view(sidebar_tree::SidebarTreeProps {
        vm: app.widgets.explorer.vm(),
        theme: theme_props,
    })
    .map(|event| AppEvent::Explorer(ExplorerEvent::Intent(event)))
}

/// Status bar of Terminal mode: machine and focused pane on the left, pane
/// and session counts on the right. It stays in focus mode.
pub(super) fn status_bar<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let tab_id = app.widgets.tabs.active_tab_id();
    let host_name = app.host_name.as_deref();
    // With no tabs the status bar names this computer.
    let machine = match app.widgets.tabs.active_tab_machine() {
        Some(machine) => machine,
        None => &TabMachine::ThisComputer,
    };
    let folder = app
        .widgets
        .terminal_workspace
        .focused_pane_label(tab_id)
        .and_then(|label| label.folder_path());

    let vm = StatusBarViewModel::new(StatusBarInput {
        machine_name: machine.name(host_name),
        machine_target: machine.target_kind(),
        folder,
        pane_count: app.widgets.terminal_workspace.pane_count(tab_id),
        session_count: app.widgets.tabs.len(),
    });

    status_bar::view(&vm, env!("CARGO_PKG_VERSION"), theme_props)
}

/// Overlays of Terminal mode, bottom to top: the context menu of the
/// Launches tree and the context menu of a terminal pane. Both lie over the
/// main area.
pub(super) fn overlays<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent, Theme, iced::Renderer>> {
    let mut layers = Vec::new();

    if let Some(vm) = app.widgets.quick_launch.context_menu_vm() {
        layers.push(
            ql_context_menu::view(ql_context_menu::ContextMenuProps {
                vm,
                theme: theme_props,
                area_size: app.state.screen_size,
            })
            .map(|event| {
                AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event))
            }),
        );
    }
    if let Some(overlay) = view_terminal_context_menu_overlay(app, theme_props)
    {
        layers.push(overlay);
    }

    layers
}

/// Render the pane grid of a terminal tab. Every pane header names the
/// machine the tab runs on. A tab whose state is missing has no terminal, so
/// the line about that is a screen like any other.
fn view_terminal_tab<'a>(
    app: &'a App,
    tab_id: u64,
    theme_props: ThemeProps<'a>,
) -> MainContent<'a> {
    let missing = || MainContent::Screen {
        id: ScreenId::Tab(tab_id),
        content: missing_tab_state(i18n::t(Key::TerminalTabNotInitialized)),
    };
    let Some(machine) = app.widgets.tabs.active_tab_machine() else {
        return missing();
    };

    let vm = app.widgets.terminal_workspace.vm(
        Some(tab_id),
        machine.name(app.host_name.as_deref()),
        machine.target_kind(),
    );

    match vm.tab {
        Some(tab_vm) => MainContent::ScrollsOnItsOwn(
            terminal_pane_grid::view(tab_vm, theme_props.theme.ui()).map(
                |event| {
                    AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                        event,
                    ))
                },
            ),
        ),
        None => missing(),
    }
}

fn view_terminal_context_menu_overlay<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Option<Element<'a, AppEvent, Theme, iced::Renderer>> {
    for (&tab_id, terminal_tab) in app.widgets.terminal_workspace.tabs() {
        let Some(menu) = terminal_tab.context_menu() else {
            continue;
        };
        let has_block_selection =
            terminal_tab.selected_block_terminal() == Some(menu.terminal_id());
        return Some(
            terminal_pane_context_menu::view(
                terminal_pane_context_menu::PaneContextMenuProps {
                    tab_id,
                    pane: menu.pane(),
                    cursor: menu.cursor(),
                    grid_size: menu.grid_size(),
                    terminal_id: menu.terminal_id(),
                    focus_target: menu.focus_target().clone(),
                    has_block_selection,
                    theme: theme_props,
                },
            )
            .map(|event| {
                AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                    event,
                ))
            }),
        );
    }

    None
}

/// Render an error placeholder for missing tab state.
fn missing_tab_state<'a>(
    message: &'static str,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    container(text(message))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;
    use otty_ui_kit::size;

    use super::super::tests::{
        Frames, SMALL_WINDOW, app_in, app_with_start_card,
        assert_each_screen_starts_at_its_top, card_area_scroller,
        offsets_through_rebuilds, open_error_tab, open_launch_editor,
        open_settings, press_and_move, reduce_quick_launch, right_panel_split,
        screen_scrollers, scroll_main_area, sent_a_panel_resize,
    };
    use super::super::view;
    use super::*;
    use crate::domain::mode::Mode;
    use crate::widgets::tabs::TabsCtx;

    #[test]
    fn terminal_mode_shows_the_switcher_over_sessions_and_launches() {
        let app = app_in(Mode::Terminal);
        let mut ui = simulator(view(&app));

        assert!(
            ui.find(i18n::t(Key::ModeTerminal)).is_err(),
            "the switcher row has no mode name"
        );
        ui.find(i18n::t(Key::SessionsPanelTitle))
            .expect("sessions should be visible");
        ui.find(i18n::t(Key::LaunchesPanelTitle))
            .expect("launches should be visible");
        ui.find(i18n::t(Key::EmptyTitle))
            .expect("empty state should be visible");
        assert!(ui.find(i18n::t(Key::ContainersSearching)).is_err());
    }

    /// Both panel splits, dragged over the start card and over its bars, move
    /// the panel and scroll nothing (ticket 40).
    #[test]
    fn dragging_a_panel_split_across_the_start_card_does_not_scroll_it() {
        let app = app_with_start_card();
        let right = right_panel_split(&app, SMALL_WINDOW);
        let left = app
            .widgets
            .sidebar
            .panes()
            .layout()
            .split_regions(size::LINE, 0.0, SMALL_WINDOW)
            .values()
            .map(|(axis, region, ratio)| {
                axis.split_line_bounds(*region, *ratio, size::LINE).x
            })
            .fold(f32::MAX, f32::min);

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
        let app = app_with_start_card();
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

    /// In the real window, between the panels, each tile comes into view inside
    /// the main area and does what it says.
    #[test]
    fn in_a_small_window_each_tile_of_the_start_card_can_be_pressed() {
        let app = app_with_start_card();

        for (key, is_expected) in [
            (
                Key::EmptyNewTerminal,
                (|event: &AppEvent| {
                    matches!(
                        event,
                        AppEvent::Tabs(TabsEvent::Intent(
                            TabsIntent::OpenTerminalTab
                        ))
                    )
                }) as fn(&AppEvent) -> bool,
            ),
            (Key::EmptyNewLaunch, |event: &AppEvent| {
                matches!(
                    event,
                    AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                        QuickLaunchIntent::HeaderCreateCommand
                    ))
                )
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

    /// iced keeps the state of a scroller for the next page of the same shape,
    /// and a form page has the shape of a card's. A form opened from a scrolled
    /// card must start at its top, and so must the card when the form closes.
    #[test]
    fn the_start_card_and_a_form_do_not_share_their_scrolling() {
        let mut app = app_with_start_card();
        let mut frames = Frames::new();
        let tabs_ctx = TabsCtx { shell_name: "sh" };

        let _ = frames.frame(&app, None);
        let card_scrolled = frames.frame(&app, scroll_main_area());

        let _ = app
            .widgets
            .tabs
            .reduce(TabsIntent::OpenSettingsTab, &tabs_ctx);
        let form_opened = frames.frame(&app, None);
        let form_scrolled = frames.frame(&app, scroll_main_area());

        let tab_id = app.widgets.tabs.active_tab_id().expect("the form's tab");
        let _ = app
            .widgets
            .tabs
            .reduce(TabsIntent::CloseTab { tab_id }, &tabs_ctx);
        let card_again = frames.frame(&app, None);

        assert!(card_scrolled.x > 0.0 && card_scrolled.y > 0.0);
        assert_eq!(form_opened, iced::Vector::new(0.0, 0.0));
        assert_eq!(form_scrolled, iced::Vector::new(30.0, 50.0));
        assert_eq!(card_again, iced::Vector::new(0.0, 0.0));
    }

    fn activate_tab(app: &mut App, tab_id: u64) {
        let _ = app.widgets.tabs.reduce(
            TabsIntent::ActivateTab { tab_id },
            &TabsCtx { shell_name: "sh" },
        );
    }

    #[test]
    fn two_launch_editors_do_not_share_their_scrolling() {
        let mut app = app_with_start_card();
        let first = open_launch_editor(&mut app);

        assert_each_screen_starts_at_its_top(
            app,
            |app| {
                let _ = open_launch_editor(app);
            },
            move |app| activate_tab(app, first),
        );
    }

    #[test]
    fn settings_and_a_launch_editor_do_not_share_their_scrolling() {
        let mut app = app_with_start_card();
        let settings = open_settings(&mut app);

        assert_each_screen_starts_at_its_top(
            app,
            |app| {
                let _ = open_launch_editor(app);
            },
            move |app| activate_tab(app, settings),
        );
    }

    #[test]
    fn a_launch_editor_and_the_error_tab_do_not_share_their_scrolling() {
        let mut app = app_with_start_card();
        let editor = open_launch_editor(&mut app);
        let error = open_error_tab(&mut app);
        activate_tab(&mut app, editor);
        let mut frames = Frames::new();

        let _ = frames.frame(&app, None);
        let scrolled = frames.frame(&app, scroll_main_area());
        activate_tab(&mut app, error);
        let error_opened = frames.frame(&app, None);

        assert_eq!(scrolled, iced::Vector::new(30.0, 50.0));
        assert_eq!(error_opened, iced::Vector::new(0.0, 0.0));
    }

    #[test]
    fn settings_built_again_keep_their_scrolling() {
        use crate::widgets::settings::event::SettingsIntent;

        let mut app = app_with_start_card();
        let _ = open_settings(&mut app);

        let offsets = offsets_through_rebuilds(
            &mut app,
            vec![
                Box::new(|app| {
                    let _ = app.widgets.settings.reduce(
                        SettingsIntent::ShellChanged(String::from("/bin/z")),
                    );
                }),
                Box::new(|app| {
                    let _ = app
                        .widgets
                        .settings
                        .reduce(SettingsIntent::EqualizePanesToggled(false));
                }),
            ],
        );

        assert_eq!(offsets[0], iced::Vector::new(30.0, 50.0));
        assert!(offsets.iter().all(|offset| *offset == offsets[0]));
    }

    #[test]
    fn a_launch_editor_built_again_keeps_its_scrolling() {
        let mut app = app_with_start_card();
        let tab_id = open_launch_editor(&mut app);

        let offsets = offsets_through_rebuilds(
            &mut app,
            vec![
                Box::new(move |app| {
                    reduce_quick_launch(
                        app,
                        QuickLaunchIntent::WizardUpdateTitle {
                            tab_id,
                            value: String::from("t"),
                        },
                    );
                }),
                // Saving an empty launch brings the validation messages.
                Box::new(move |app| {
                    reduce_quick_launch(
                        app,
                        QuickLaunchIntent::WizardSave { tab_id },
                    );
                }),
            ],
        );

        assert_eq!(offsets[0], iced::Vector::new(30.0, 50.0));
        assert!(offsets.iter().all(|offset| *offset == offsets[0]));
    }

    /// The panes of a terminal fill the height they get and scroll on their
    /// own. This opens a real shell, as the tests of the terminal workspace do.
    #[test]
    fn a_terminal_tab_stays_out_of_the_scrolling_of_a_screen() {
        use otty_ui_term::settings::{
            LocalSessionOptions, SessionKind, Settings,
        };

        use crate::widgets::terminal_workspace::TerminalWorkspaceCtx;
        use crate::widgets::terminal_workspace::event::TerminalWorkspaceIntent;
        use crate::widgets::terminal_workspace::types::TerminalKind;

        let mut app = app_with_start_card();
        let _ = app
            .widgets
            .tabs
            .reduce(TabsIntent::OpenTerminalTab, &TabsCtx { shell_name: "sh" });
        let tab_id = app.widgets.tabs.active_tab_id().expect("the tab");
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program("/bin/sh"),
            ),
        );
        let _ = app.widgets.terminal_workspace.reduce(
            TerminalWorkspaceIntent::OpenTab {
                tab_id,
                default_title: String::from("sh"),
                settings: Box::new(settings),
                kind: TerminalKind::Shell,
                sync_explorer: false,
            },
            &TerminalWorkspaceCtx {
                active_tab_id: None,
                pane_grid_size: SMALL_WINDOW,
                screen_size: SMALL_WINDOW,
                sidebar_cursor: Point::ORIGIN,
                equalize_panes: true,
            },
        );

        let content = main(&app, ThemeProps::new(app.theme_manager.current()));

        assert!(matches!(content, MainContent::ScrollsOnItsOwn(_)));
        assert_eq!(screen_scrollers(&app), 0);
    }
}
