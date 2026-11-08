use iced::widget::{Space, column, container, mouse_area, pane_grid, text};
use iced::{Element, Length, Theme, alignment};
use otty_ui_kit::components::divider;
use otty_ui_kit::components::screen_area::ScreenArea;
use otty_ui_kit::spacing;

use super::{App, AppEvent};
#[cfg(not(target_os = "macos"))]
use crate::components::primitive::resize_grips;
use crate::domain::mode::Mode;
use crate::geometry::anchor_position;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::chrome::view::{action_bar, status_bar};
use crate::widgets::chrome::{StatusBarInput, StatusBarViewModel};
use crate::widgets::containers::actions::view::{
    confirm_dialog as containers_confirm, row_menu as containers_row_menu,
};
use crate::widgets::containers::create::view::form_dialog as containers_form;
use crate::widgets::containers::event::{ContainersEvent, ContainersIntent};
use crate::widgets::containers::no_engine::NoEngineCase;
use crate::widgets::containers::run::view::{
    run_form as containers_run_form, run_panel as containers_run_panel,
};
use crate::widgets::containers::view::{
    engine_tree, inspector as containers_inspector,
    main_area as containers_main, status_bar as containers_status_bar,
};
use crate::widgets::containers::{MainViewModel, format as containers_format};
use crate::widgets::explorer::event::ExplorerEvent;
use crate::widgets::explorer::view::sidebar_tree;
use crate::widgets::modes::event::{ModesEvent, ModesIntent};
use crate::widgets::modes::view::{mode_menu, mode_switcher};
use crate::widgets::palette::event::PaletteEvent;
use crate::widgets::palette::view::command_palette as palette_view;
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::quick_launch::view::{
    context_menu as ql_context_menu, error_tab, new_launch_menu, secret_prompt,
    sidebar_panel, wizard_form,
};
use crate::widgets::settings::event::SettingsEvent;
use crate::widgets::settings::view::settings_form;
use crate::widgets::sidebar::SidebarViewModel;
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::sidebar::types::{HeaderMenu, OpenHeaderMenu, SidebarPane};
use crate::widgets::sidebar::view::{focus_note, left_panel, panels};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::tabs::types::{TabContent, TabMachine};
use crate::widgets::tabs::view::{empty_state, new_session_menu, session_list};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};
use crate::widgets::terminal_workspace::view::{
    pane_context_menu as terminal_pane_context_menu,
    pane_grid as terminal_pane_grid,
};

/// Which screen fills the main area. Each value is one screen to the user: a
/// change of value starts at the top left, and the same value built again keeps
/// its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenId {
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
enum MainContent<'a> {
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
                view_tab_content(app, theme_props).into_element(theme_props)
            },
            (SidebarPane::Main, Mode::Containers) => {
                view_containers_main(app, theme_props).into_element(theme_props)
            },
            (SidebarPane::Right, Mode::Terminal) => {
                view_right_panel(app, theme_props)
            },
            (SidebarPane::Right, Mode::Containers) => {
                view_containers_inspector(app, theme_props)
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
        || (mode == Mode::Containers && app.widgets.containers.has_overlay())
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
        Mode::Containers => containers_status_bar::view(
            &app.widgets.containers.status_vm(),
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
    // The Containers row menu opens at the pointer in window coordinates, so it
    // and the dialogs lie over the whole window.
    if mode == Mode::Containers {
        root_layers.extend(view_containers_overlays(app, theme_props));
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
        Mode::Containers => view_containers_tree(app, theme_props),
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

/// Containers mode left panel content: the engine tree.
fn view_containers_tree<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    engine_tree::view(engine_tree::EngineTreeProps {
        rows: app.widgets.containers.tree_vm(),
        note: app.widgets.containers.tree_note(),
        theme: theme_props,
    })
    .map(|intent| AppEvent::Containers(ContainersEvent::Intent(intent)))
}

/// Containers mode main area: the Run container form while it is open, else
/// the open list of an engine or a card while there is none.
fn view_containers_main<'a>(
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

    let vm = app
        .widgets
        .containers
        .main_vm(containers_format::unix_now());
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

/// Containers mode right panel content: what the image sets while the Run
/// form is open, else the inspector of the selected row.
fn view_containers_inspector<'a>(
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

/// Containers mode overlays: the row menu, the confirmation dialog and the
/// create dialog.
fn view_containers_overlays<'a>(
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
            },
        ),
        (Some(tab_id), Some(TabContent::QuickLaunchError)) => (
            ScreenId::Tab(tab_id),
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
    use crate::widgets::containers::event::ContainersIntent;
    use crate::widgets::containers::run::event::RunIntent;
    use crate::widgets::containers::run::types::RunEdit;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::tabs::TabsCtx;

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
        assert!(ui.find(i18n::t(Key::ContainersSearching)).is_err());
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

    /// A window where the main area between the panels at their smallest is
    /// too narrow and too low for the no-engine card. Panels dragged wide do
    /// the same in a larger window.
    const SMALL_WINDOW: iced::Size = iced::Size::new(400.0, 250.0);

    /// A no-engine card with these Docker and Podman commands found.
    fn app_with_card(discovery: otty_containers::Discovery) -> App {
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
    fn denied_podman_search() -> otty_containers::Discovery {
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
    fn card_area_scroller(
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

    /// Presses at the first point, moves along the others and stays pressed.
    fn press_and_move(
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

    fn sent_a_panel_resize(ui: iced_test::Simulator<'_, AppEvent>) -> bool {
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
    fn right_panel_split(app: &App, window: iced::Size) -> f32 {
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

    /// One frame of the app after another, as the running app builds them: the
    /// widget state carries over from one to the next.
    struct Frames {
        renderer: iced::Renderer,
        cache: Option<iced_test::runtime::user_interface::Cache>,
    }

    impl Frames {
        fn new() -> Self {
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
        fn frame(
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

    /// Terminal mode with no tab open: the start card in a [`SMALL_WINDOW`].
    fn app_with_start_card() -> App {
        let mut app = app_in(Mode::Terminal);
        app.state.window_size = SMALL_WINDOW;

        app
    }

    /// A wheel in the main area of a [`SMALL_WINDOW`] that moves a page 30 px
    /// right and 50 px down.
    fn scroll_main_area() -> Option<(Point, iced::Vector)> {
        Some((Point::new(200.0, 150.0), iced::Vector::new(-30.0, -50.0)))
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

    /// Changes the quick launch widget as the running app does.
    fn reduce_quick_launch(app: &mut App, intent: QuickLaunchIntent) {
        let settings = app.terminal_settings.clone();
        let ctx = crate::widgets::quick_launch::QuickLaunchCtx {
            terminal_settings: &settings,
            sidebar_cursor: Point::ORIGIN,
            sidebar_is_resizing: false,
        };

        let _ = app.widgets.quick_launch.reduce(intent, &ctx);
    }

    /// Opens Settings as a tab and returns its id.
    fn open_settings(app: &mut App) -> u64 {
        let _ = app
            .widgets
            .tabs
            .reduce(TabsIntent::OpenSettingsTab, &TabsCtx { shell_name: "sh" });

        app.widgets.tabs.active_tab_id().expect("the form's tab")
    }

    /// Opens a launch editor as a tab, its state in place, and returns its id.
    fn open_launch_editor(app: &mut App) -> u64 {
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
    fn open_error_tab(app: &mut App) -> u64 {
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

    fn activate_tab(app: &mut App, tab_id: u64) {
        let _ = app.widgets.tabs.reduce(
            TabsIntent::ActivateTab { tab_id },
            &TabsCtx { shell_name: "sh" },
        );
    }

    /// The Run form over the lists of the fixture Docker engine, in a window
    /// too small for it.
    fn app_with_run_form() -> App {
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
    type AppChange = Box<dyn Fn(&mut App)>;

    /// The offset of a page that was just scrolled once and is built again
    /// after each of `changes`, which must all leave the same screen on show.
    fn offsets_through_rebuilds(
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
    fn assert_each_screen_starts_at_its_top(
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

    /// How many scrollers in `app` have content of the minimum width of a
    /// screen.
    fn screen_scrollers(app: &App) -> usize {
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

        let content = view_tab_content(
            &app,
            ThemeProps::new(app.theme_manager.current()),
        );

        assert!(matches!(content, MainContent::ScrollsOnItsOwn(_)));
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

    fn containers_app_with_lists() -> App {
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
