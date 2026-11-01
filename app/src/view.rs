use iced::widget::{Space, column, container, mouse_area, pane_grid, text};
use iced::{Element, Length, Theme, alignment};
use otty_ui_kit::components::divider;
use otty_ui_kit::spacing;

use super::{App, AppEvent};
#[cfg(not(target_os = "macos"))]
use crate::components::primitive::resize_grips;
use crate::geometry::anchor_position;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;
use crate::widgets::chrome::ChromeEvent;
use crate::widgets::chrome::model::{StatusBarInput, StatusBarViewModel};
use crate::widgets::chrome::view::{action_bar, status_bar};
use crate::widgets::docker::DockerEvent;
use crate::widgets::docker::view::{
    confirm_dialog as docker_confirm, engine_tree, form_dialog as docker_form,
    inspector as docker_inspector, main_area as docker_main,
    row_menu as docker_row_menu, run_form as docker_run_form,
    run_panel as docker_run_panel, status_bar as docker_status_bar,
};
use crate::widgets::explorer::ExplorerEvent;
use crate::widgets::explorer::view::sidebar_tree;
use crate::widgets::modes::types::Mode;
use crate::widgets::modes::view::{mode_menu, mode_switcher};
use crate::widgets::modes::{ModesEvent, ModesIntent};
use crate::widgets::palette::{PaletteEvent, view as palette_view};
use crate::widgets::quick_launch::view::{
    context_menu as ql_context_menu, error_tab, new_launch_menu, secret_prompt,
    sidebar_panel, wizard_form,
};
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::settings::SettingsEvent;
use crate::widgets::settings::view::settings_form;
use crate::widgets::sidebar::types::{HeaderMenu, OpenHeaderMenu, SidebarPane};
use crate::widgets::sidebar::view::{focus_note, left_panel, panels};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent, SidebarViewModel};
use crate::widgets::tabs::types::{TabContent, TabMachine};
use crate::widgets::tabs::view::{empty_state, new_session_menu, session_list};
use crate::widgets::tabs::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::view::{
    pane_context_menu as terminal_pane_context_menu,
    pane_grid as terminal_pane_grid,
};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Render the root application view.
pub(super) fn view(app: &App) -> Element<'_, AppEvent, Theme, iced::Renderer> {
    let theme = app.theme_manager.current();
    let theme_props: ThemeProps<'_> = ThemeProps::new(theme);

    let sidebar_vm = app.widgets.sidebar.vm();
    let mode = app.widgets.modes.active();

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
                view_tab_content(app, theme_props)
            },
            (SidebarPane::Main, Mode::Docker) => {
                view_docker_main(app, theme_props)
            },
            (SidebarPane::Right, Mode::Terminal) => {
                view_right_panel(app, theme_props)
            },
            (SidebarPane::Right, Mode::Docker) => {
                view_docker_inspector(app, theme_props)
            },
        },
        on_intent: |intent| AppEvent::Sidebar(SidebarEvent::Intent(intent)),
    });

    let content_row = mouse_area(content_row).on_move(|position| {
        AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::WorkspaceCursorMoved { position },
        ))
    });

    let mut layers: Vec<Element<'_, AppEvent, Theme, iced::Renderer>> =
        vec![content_row.into()];

    // Focus mode note
    if let Some(note) =
        focus_note::view(sidebar_vm.focus_note_opacity, theme_props)
    {
        layers.push(note);
    }

    // Header menu overlay (New session or Launches)
    if let Some(open) = sidebar_vm.open_header_menu {
        layers.push(view_header_menu_overlay(app, open, theme_props));
    }

    // All modes menu under the switcher
    let modes_vm = app.widgets.modes.vm();
    if modes_vm.is_menu_open {
        layers.push(
            mode_menu::view(mode_menu::ModeMenuProps {
                vm: modes_vm,
                theme: theme_props,
            })
            .map(|intent| AppEvent::Modes(ModesEvent::Intent(intent))),
        );
    }

    // Quick launch context menu overlay
    if let Some(menu) = app.widgets.quick_launch.context_menu() {
        layers.push(
            ql_context_menu::view(ql_context_menu::ContextMenuProps {
                menu,
                theme: theme_props,
                area_size: app.state.screen_size,
                launching: app.widgets.quick_launch.launching(),
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

    // Command palette
    if let Some(vm) = app.widgets.palette.vm() {
        layers.push(
            palette_view::view(palette_view::PaletteProps {
                vm,
                theme: theme_props,
            })
            .map(|intent| AppEvent::Palette(PaletteEvent::Intent(intent))),
        );
    }

    // SSH secret dialog stays on top of every other overlay.
    if let Some(prompt) = app.widgets.quick_launch.secret_prompt() {
        layers.push(
            secret_prompt::view(secret_prompt::SecretPromptProps {
                prompt,
                theme: theme_props,
            })
            .map(|event| {
                AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event))
            }),
        );
    }

    let content_stack = iced::widget::Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill);

    #[cfg(target_os = "macos")]
    let resize_grips_layer = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    #[cfg(not(target_os = "macos"))]
    let resize_grips_layer = if app.widgets.sidebar.has_header_menu_open()
        || app.widgets.modes.is_menu_open()
        || app.widgets.quick_launch.context_menu().is_some()
        || app.widgets.quick_launch.secret_prompt().is_some()
        || app.widgets.terminal_workspace.has_any_context_menu()
        || (mode == Mode::Docker && app.widgets.docker.has_overlay())
    {
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        resize_grips::view().map(|event| match event {
            resize_grips::ResizeGripEvent::Resize(dir) => {
                AppEvent::ResizeWindow(dir)
            },
        })
    };

    let header = view_header(app, sidebar_vm, theme_props);

    let status_bar = match mode {
        Mode::Terminal => view_status_bar(app, theme_props),
        Mode::Docker => docker_status_bar::view(
            &app.widgets.docker.status_vm(),
            app.widgets.tabs.len(),
            env!("CARGO_PKG_VERSION"),
            AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Terminal,
            ))),
            theme_props,
        ),
    };

    let mut root_layers: Vec<Element<'_, AppEvent, Theme, iced::Renderer>> = vec![
        column![header, content_stack, status_bar]
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    ];
    // The Docker row menu opens at the pointer in window coordinates, so it
    // and the dialogs lie over the whole window.
    if mode == Mode::Docker {
        root_layers.extend(view_docker_overlays(app, theme_props));
    }
    root_layers.push(resize_grips_layer);

    iced::widget::Stack::with_children(root_layers)
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

/// Render the status bar: machine and focused pane on the left, pane and
/// session counts on the right. It stays in focus mode.
fn view_status_bar<'a>(
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
        Mode::Terminal => {
            view_terminal_left_panel(app, sidebar_vm, theme_props)
        },
        Mode::Docker => view_docker_tree(app, theme_props),
    };

    column![switcher, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Terminal mode left panel: the Sessions section over the Launches
/// section, sharing the panel height (see [`left_panel::view`]).
fn view_terminal_left_panel<'a>(
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

/// Docker mode left panel content: the engine tree.
fn view_docker_tree<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    engine_tree::view(engine_tree::EngineTreeProps {
        rows: app.widgets.docker.tree_vm(),
        note: app.widgets.docker.tree_note(),
        theme: theme_props,
    })
    .map(|intent| AppEvent::Docker(DockerEvent::Intent(intent)))
}

/// Docker mode main area: the Run container form while it is open, else
/// the open list of an engine.
fn view_docker_main<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let main = match app.widgets.docker.run_form_vm() {
        Some(vm) => docker_run_form::view(docker_run_form::RunFormProps {
            vm,
            theme: theme_props,
        }),
        None => docker_main::view(docker_main::MainAreaProps {
            vm: app.widgets.docker.main_vm(unix_now()),
            theme: theme_props,
        }),
    };

    main.map(|intent| AppEvent::Docker(DockerEvent::Intent(intent)))
}

/// Docker mode right panel content: what the image sets while the Run
/// form is open, else the inspector of the selected row.
fn view_docker_inspector<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    if let Some(vm) = app.widgets.docker.image_panel_vm() {
        return docker_run_panel::view(docker_run_panel::RunPanelProps {
            vm,
            theme: theme_props,
        })
        .map(|intent| AppEvent::Docker(DockerEvent::Intent(intent)));
    }

    docker_inspector::view(docker_inspector::InspectorProps {
        vm: app.widgets.docker.inspector_vm(),
        tabs: app.widgets.docker.inspector_tabs_vm(),
        actions: app.widgets.docker.actions_vm(),
        theme: theme_props,
    })
    .map(|intent| AppEvent::Docker(DockerEvent::Intent(intent)))
}

/// Docker mode overlays: the row menu and the confirmation dialog.
fn view_docker_overlays<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent, Theme, iced::Renderer>> {
    let mut layers = Vec::new();

    if let Some(vm) = app.widgets.docker.row_menu_vm() {
        layers.push(docker_row_menu::view(docker_row_menu::RowMenuProps {
            vm,
            area: app.state.window_size,
            theme: theme_props,
        }));
    }
    if let Some(vm) = app.widgets.docker.confirm_vm() {
        layers.push(docker_confirm::view(docker_confirm::ConfirmDialogProps {
            vm,
            theme: theme_props,
        }));
    }
    if let Some(vm) = app.widgets.docker.form_vm() {
        layers.push(docker_form::view(docker_form::FormDialogProps {
            vm,
            theme: theme_props,
        }));
    }

    layers
        .into_iter()
        .map(|layer| {
            layer.map(|intent| AppEvent::Docker(DockerEvent::Intent(intent)))
        })
        .collect()
}

/// The current time in Unix seconds, for the age column of the Docker
/// tables; 0 when the clock is before 1970.
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// Right panel content: the Files section.
fn view_right_panel<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    sidebar_tree::view(sidebar_tree::SidebarTreeProps {
        vm: app.widgets.explorer.vm(),
        theme: theme_props,
    })
    .map(|event| AppEvent::Explorer(ExplorerEvent::Intent(event)))
}

/// Render the active tab content based on its type.
fn view_tab_content<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let active_tab_id = app.widgets.tabs.active_tab_id();
    let active_tab_content = app.widgets.tabs.active_tab_content();

    match (active_tab_id, active_tab_content) {
        (Some(tab_id), Some(TabContent::Terminal)) => {
            view_terminal_tab(app, tab_id, theme_props)
        },
        (Some(_tab_id), Some(TabContent::Settings)) => {
            settings_form::view(settings_form::SettingsFormProps {
                vm: app.widgets.settings.vm(),
                theme: theme_props,
            })
            .map(|event| AppEvent::Settings(SettingsEvent::Intent(event)))
        },
        (Some(tab_id), Some(TabContent::QuickLaunchWizard)) => {
            match app.widgets.quick_launch.wizard_editor(tab_id) {
                Some(editor) => {
                    wizard_form::view(wizard_form::WizardFormProps {
                        tab_id,
                        editor,
                        theme: theme_props,
                    })
                    .map(|event| {
                        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event))
                    })
                },
                None => missing_tab_state(i18n::t(
                    Key::QuickLaunchEditorNotInitialized,
                )),
            }
        },
        (Some(tab_id), Some(TabContent::QuickLaunchError)) => {
            match app.widgets.quick_launch.error_tab(tab_id) {
                Some(error) => error_tab::view(error_tab::ErrorTabProps {
                    error,
                    theme: theme_props,
                })
                .map(|event| {
                    AppEvent::QuickLaunch(QuickLaunchEvent::Intent(event))
                }),
                None => missing_tab_state(i18n::t(
                    Key::QuickLaunchErrorPayloadMissing,
                )),
            }
        },
        _ => empty_state::view(empty_state::EmptyStateProps {
            theme: theme_props,
            on_new_terminal: AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenTerminalTab,
            )),
            on_new_launch: AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::HeaderCreateCommand,
            )),
        }),
    }
}

/// Render the pane grid of a terminal tab. Every pane header names the
/// machine the tab runs on.
fn view_terminal_tab<'a>(
    app: &'a App,
    tab_id: u64,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let Some(machine) = app.widgets.tabs.active_tab_machine() else {
        return missing_tab_state(i18n::t(Key::TerminalTabNotInitialized));
    };

    let vm = app.widgets.terminal_workspace.vm(
        Some(tab_id),
        machine.name(app.host_name.as_deref()),
        machine.target_kind(),
    );

    match vm.tab {
        Some(tab_vm) => terminal_pane_grid::view(
            tab_vm,
            theme_props.theme.ui(),
        )
        .map(|event| {
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(event))
        }),
        None => missing_tab_state(i18n::t(Key::TerminalTabNotInitialized)),
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

/// Render the open header menu with its dismiss layer, anchored at the point
/// where it was opened.
fn view_header_menu_overlay<'a>(
    app: &'a App,
    open: OpenHeaderMenu,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let (menu, menu_width, menu_height) = match open.menu() {
        HeaderMenu::Sessions => view_new_session_menu(app, theme_props),
        HeaderMenu::Launches => view_new_launch_menu(theme_props),
    };

    let anchor = anchor_position(
        open.cursor(),
        app.state.screen_size,
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

    let dismiss = || {
        AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::HeaderMenuDismiss,
        ))
    };
    let dismiss_layer = mouse_area(
        container(text("")).width(Length::Fill).height(Length::Fill),
    )
    .on_press(dismiss())
    .on_right_press(dismiss())
    .on_move(|position| {
        AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::WorkspaceCursorMoved { position },
        ))
    });

    iced::widget::stack!(dismiss_layer, positioned_menu)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The New session menu with its width and height.
fn view_new_session_menu<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> (Element<'a, AppEvent, Theme, iced::Renderer>, f32, f32) {
    let active_tab_id = app.widgets.tabs.active_tab_id();
    let on_split = app
        .widgets
        .terminal_workspace
        .focused_pane(active_tab_id)
        .map(|(tab_id, pane)| {
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitPane {
                    tab_id,
                    pane,
                    axis: pane_grid::Axis::Vertical,
                },
            ))
        });
    let launches: Vec<_> = app
        .widgets
        .quick_launch
        .top_level_launches()
        .into_iter()
        .map(|launch| {
            let run = QuickLaunchIntent::LaunchNode { path: launch.path };
            (
                launch.title,
                AppEvent::QuickLaunch(QuickLaunchEvent::Intent(run)),
            )
        })
        .collect();
    let menu_height = new_session_menu::menu_height(launches.len());

    let menu = new_session_menu::view(new_session_menu::NewSessionMenuProps {
        theme: theme_props,
        shell_name: app.shell_session.name(),
        on_new_terminal: AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::OpenTerminalTab,
        )),
        on_split,
        launches,
        on_new_launch: AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::HeaderCreateCommand,
        )),
    });

    (menu, new_session_menu::NEW_SESSION_MENU_WIDTH, menu_height)
}

/// The Launches menu with its width and height.
fn view_new_launch_menu<'a>(
    theme_props: ThemeProps<'a>,
) -> (Element<'a, AppEvent, Theme, iced::Renderer>, f32, f32) {
    let menu = new_launch_menu::view(new_launch_menu::NewLaunchMenuProps {
        theme: theme_props,
        on_intent: |intent| {
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent))
        },
    });

    (
        menu,
        otty_ui_kit::components::menu::MENU_WIDTH,
        new_launch_menu::menu_height(),
    )
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;
    use otty_ui_kit::components::menu;
    use otty_ui_kit::size;

    use super::*;
    use crate::widgets::docker::DockerIntent;
    use crate::widgets::modes::ModesWidget;

    fn app_in(mode: Mode) -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        let _ = app.widgets.modes.reduce(ModesIntent::Select(mode));
        app
    }

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
        assert!(ui.find(i18n::t(Key::DockerSearching)).is_err());
    }

    #[test]
    fn docker_mode_swaps_the_left_panel_main_area_and_status_bar() {
        let app = app_in(Mode::Docker);
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::DockerSearching))
            .expect("docker main area should be visible");
        assert!(ui.find(i18n::t(Key::SessionsPanelTitle)).is_err());
        assert!(ui.find(i18n::t(Key::LaunchesPanelTitle)).is_err());
        assert!(ui.find(i18n::t(Key::EmptyTitle)).is_err());
    }

    #[test]
    fn docker_mode_shows_the_engine_tree_the_table_and_the_inspector() {
        use otty_docker::{EngineKind, fixtures};

        use crate::widgets::docker::DockerIntent;
        use crate::widgets::docker::types::LoadedLists;

        let mut app = app_in(Mode::Docker);
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = app.widgets.docker.reduce(DockerIntent::Enter);
        let _ = app.widgets.docker.reduce(DockerIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = app.widgets.docker.reduce(DockerIntent::ListsLoaded {
            engine: docker.address().clone(),
            lists: LoadedLists {
                containers: Ok(resources.containers().to_vec()),
                images: Ok(resources.images().to_vec()),
                volumes: Ok(resources.volumes().to_vec()),
                networks: Ok(resources.networks().to_vec()),
            },
        });
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::DockerImages))
            .expect("tree should list the images");
        ui.find("otty-fx-exit3")
            .expect("table should list containers");
        ui.click("otty-fx-exit3").expect("row should be clickable");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Docker(DockerEvent::Intent(DockerIntent::SelectRow(5)))
        )));
    }

    #[test]
    fn the_run_form_replaces_the_table_and_the_inspector() {
        use otty_docker::{EngineKind, fixtures};

        use crate::widgets::docker::DockerIntent;
        use crate::widgets::docker::types::LoadedLists;

        let mut app = app_in(Mode::Docker);
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = app.widgets.docker.reduce(DockerIntent::Enter);
        let _ = app.widgets.docker.reduce(DockerIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = app.widgets.docker.reduce(DockerIntent::ListsLoaded {
            engine: docker.address().clone(),
            lists: LoadedLists {
                containers: Ok(resources.containers().to_vec()),
                images: Ok(resources.images().to_vec()),
                volumes: Ok(resources.volumes().to_vec()),
                networks: Ok(resources.networks().to_vec()),
            },
        });
        let _ = app.widgets.docker.reduce(DockerIntent::OpenCreate);
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(Key::DockerRunSubtitle))
            .expect("main area should show the run form");
        ui.find(i18n::t(Key::DockerRunNoImage))
            .expect("right panel should show the image panel");
        assert!(ui.find("otty-fx-exit3").is_err());
    }

    #[test]
    fn docker_mode_shows_the_row_menu_and_the_remove_dialog_over_the_table() {
        use otty_docker::{EngineKind, fixtures};

        use crate::widgets::docker::DockerIntent;
        use crate::widgets::docker::actions::RowAction;
        use crate::widgets::docker::types::LoadedLists;

        let mut app = app_in(Mode::Docker);
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = app.widgets.docker.reduce(DockerIntent::Enter);
        let _ = app.widgets.docker.reduce(DockerIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = app.widgets.docker.reduce(DockerIntent::ListsLoaded {
            engine: docker.address().clone(),
            lists: LoadedLists {
                containers: Ok(resources.containers().to_vec()),
                images: Ok(resources.images().to_vec()),
                volumes: Ok(resources.volumes().to_vec()),
                networks: Ok(resources.networks().to_vec()),
            },
        });
        let _ = app.widgets.docker.reduce(DockerIntent::OpenRowMenu {
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
            .find(i18n::t(Key::DockerActionStart))
            .expect("menu item should show");
        menu_ui.point_at(first_item);
        let _ = menu_ui.simulate(iced_test::simulator::click());
        let events: Vec<AppEvent> = menu_ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Docker(DockerEvent::Intent(DockerIntent::Act(
                RowAction::Start
            )))
        )));

        let _ = app
            .widgets
            .docker
            .reduce(DockerIntent::Act(RowAction::Remove));
        let mut dialog_ui = simulator(view(&app));
        dialog_ui
            .find(i18n::t(Key::DockerRemoveContainerText))
            .expect("the remove dialog should show");
        drop(dialog_ui);
        assert!(app.widgets.docker.has_overlay());

        let _ = app.widgets.docker.reduce(DockerIntent::ConfirmCancelled);
        let _ = app.widgets.docker.reduce(DockerIntent::Open(
            crate::widgets::docker::types::Place::new(
                docker.address().clone(),
                crate::widgets::docker::types::ResourceKind::Images,
            ),
        ));
        let _ = app.widgets.docker.reduce(DockerIntent::OpenCreate);
        let mut pull_ui = simulator(view(&app));
        pull_ui
            .find(i18n::t(Key::DockerFieldImage))
            .expect("the pull dialog should show");
        assert!(app.widgets.docker.has_overlay());
    }

    fn docker_app_with_lists() -> App {
        use otty_docker::{EngineKind, fixtures};

        use crate::widgets::docker::DockerIntent;
        use crate::widgets::docker::types::LoadedLists;

        let mut app = app_in(Mode::Docker);
        // The simulator draws the window at its default size.
        app.state.window_size = iced::Size::new(1024.0, 768.0);
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = app.widgets.docker.reduce(DockerIntent::Enter);
        let _ = app.widgets.docker.reduce(DockerIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = app.widgets.docker.reduce(DockerIntent::ListsLoaded {
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
                        event,
                        AppEvent::Docker(DockerEvent::Intent(
                            DockerIntent::RowMenuPressed { .. }
                        ))
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
        let app = docker_app_with_lists();

        let (row_center, clicked_x, events) = press_first_row_menu(&app);

        let [
            AppEvent::Docker(DockerEvent::Intent(
                DockerIntent::RowMenuPressed { position, .. },
            )),
        ] = events.as_slice()
        else {
            panic!("one row menu intent expected");
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
        let mut app = docker_app_with_lists();
        let (row_center, clicked_x, events) = press_first_row_menu(&app);
        for event in events {
            let _ = app.update(event);
        }

        let mut ui = simulator(view(&app));
        let item = ui
            .find(i18n::t(Key::DockerActionStop))
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
            [(tab(1.0), Mode::Docker), (tab(0.0), Mode::Terminal)]
        {
            let events = click_in_window(&app, point);
            for event in events {
                let _ = app.update(event);
            }
            assert_eq!(app.widgets.modes.active(), expected);
        }

        for event in click_in_window(&app, tab(2.0)) {
            let _ = app.update(event);
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

        ui.click(i18n::t(Key::ModeDocker))
            .expect("docker row should be visible");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Docker
            )))]
        ));
    }

    #[test]
    fn the_docker_sessions_button_switches_to_terminal_mode() {
        let app = app_in(Mode::Docker);
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
