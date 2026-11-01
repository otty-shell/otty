use iced::widget::pane_grid::PaneGrid;
use iced::widget::{Space, column, container, mouse_area, text};
use iced::{Element, Length, Size, Theme, alignment};
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::components::surface::{Surface, surface};
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::components::{divider, split};
use otty_ui_kit::{size, spacing};

use super::{App, AppEvent};
#[cfg(not(target_os = "macos"))]
use crate::components::primitive::resize_grips;
use crate::geometry::{anchor_position, menu_height_for_items};
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;
use crate::widgets::chrome::ChromeEvent;
use crate::widgets::chrome::model::{StatusBarInput, StatusBarViewModel};
use crate::widgets::chrome::view::{action_bar, focus_note, status_bar};
use crate::widgets::explorer::ExplorerEvent;
use crate::widgets::explorer::view::sidebar_tree;
use crate::widgets::quick_launch::view::{
    context_menu as ql_context_menu, error_tab, secret_prompt, sidebar_panel,
    wizard_form,
};
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::settings::SettingsEvent;
use crate::widgets::settings::view::settings_form;
use crate::widgets::sidebar::types::SidebarPane;
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent, SidebarViewModel};
use crate::widgets::tabs::types::TabContent;
use crate::widgets::tabs::view::{empty_state, session_list};
use crate::widgets::tabs::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::TerminalWorkspaceEvent;
use crate::widgets::terminal_workspace::model::followed_program;
use crate::widgets::terminal_workspace::view::{
    pane_context_menu as terminal_pane_context_menu,
    pane_grid as terminal_pane_grid,
};

/// Render the root application view.
pub(super) fn view(app: &App) -> Element<'_, AppEvent, Theme, iced::Renderer> {
    let theme = app.theme_manager.current();
    let theme_props: ThemeProps<'_> = ThemeProps::new(theme);

    let sidebar_vm = app.widgets.sidebar.vm();

    let content_row = view_panels_layout(app, sidebar_vm, theme_props);

    let content_row = mouse_area(content_row).on_move(|position| {
        AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::WorkspaceCursorMoved { position },
        ))
    });

    let mut layers: Vec<Element<'_, AppEvent, Theme, iced::Renderer>> =
        vec![content_row.into()];

    // Focus mode note
    if let Some(note) = focus_note::view(sidebar_vm.is_focus_mode, theme_props)
    {
        layers.push(note);
    }

    // Add menu overlay
    if sidebar_vm.has_add_menu_open
        && let Some(cursor) = sidebar_vm.add_menu_cursor
    {
        layers.push(
            view_add_menu_overlay(cursor, app.state.screen_size, theme_props)
                .map(|event| AppEvent::Sidebar(SidebarEvent::Intent(event))),
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
    let resize_grips_layer = if app.widgets.sidebar.has_add_menu_open()
        || app.widgets.quick_launch.context_menu().is_some()
        || app.widgets.quick_launch.secret_prompt().is_some()
        || app.widgets.terminal_workspace.has_any_context_menu()
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

    let status_bar = view_status_bar(app, theme_props);

    let root_layers: Vec<Element<'_, AppEvent, Theme, iced::Renderer>> = vec![
        column![header, content_stack, status_bar]
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        resize_grips_layer,
    ];

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
        title: app.widgets.tabs.active_tab_title().unwrap_or("OTTY"),
        theme: theme_props,
        left_open: sidebar_vm.is_left_open,
        right_open: sidebar_vm.is_right_open,
        focus_mode: sidebar_vm.is_focus_mode,
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
    let (machine_name, machine_target) =
        match app.widgets.tabs.active_tab_machine() {
            Some(machine) => (machine.name(host_name), machine.target_kind()),
            None => (
                host_name.unwrap_or(i18n::t(Key::ThisComputer)),
                TargetKind::Local,
            ),
        };
    let focused_pane = app
        .widgets
        .terminal_workspace
        .focused_pane_header(tab_id, machine_name);

    let vm = StatusBarViewModel::new(StatusBarInput {
        machine_name,
        machine_target,
        focused_pane: focused_pane.as_ref(),
        pane_count: app.widgets.terminal_workspace.pane_count(tab_id),
        session_count: app.widgets.tabs.len(),
    });

    status_bar::view(&vm, env!("CARGO_PKG_VERSION"), theme_props)
}

/// Render `left panel | main | right panel` with a pane grid. A closed panel
/// is a zero-width pane, so its edge can still be dragged open.
///
/// Focus mode keeps the same pane grid with both panels at zero width, so
/// the main area keeps its widget state (terminal focus, scroll); only the
/// drag handles are removed.
fn view_panels_layout<'a>(
    app: &'a App,
    sidebar_vm: SidebarViewModel,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let panels = PaneGrid::new(
        app.widgets.sidebar.panes(),
        move |_pane, pane_kind, _| {
            let content: Element<'_, AppEvent, Theme, iced::Renderer> =
                match *pane_kind {
                    SidebarPane::Left if sidebar_vm.is_left_open => side_panel(
                        view_left_panel(app, theme_props),
                        theme_props,
                    ),
                    SidebarPane::Right if sidebar_vm.is_right_open => {
                        side_panel(
                            view_right_panel(app, theme_props),
                            theme_props,
                        )
                    },
                    SidebarPane::Main => view_tab_content(app, theme_props),
                    _ => Space::new()
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into(),
                };
            iced::widget::pane_grid::Content::new(content)
        },
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(size::LINE)
    .min_size(0)
    .style(split::style(theme_props.theme.ui()));

    if sidebar_vm.is_focus_mode {
        return panels.into();
    }

    panels
        .on_resize(size::SPLIT_GRAB, |event| {
            AppEvent::Sidebar(SidebarEvent::Intent(SidebarIntent::Resized(
                event,
            )))
        })
        .into()
}

/// Put panel content on the panel surface.
fn side_panel<'a>(
    content: Element<'a, AppEvent, Theme, iced::Renderer>,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    surface(content, Surface::Panel, theme_props.theme.ui()).into()
}

/// Left panel content: the Sessions section over the Launches section.
/// Launches is the only growing section, so it takes the height left under
/// Sessions.
fn view_left_panel<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let sidebar_vm = app.widgets.sidebar.vm();

    let sessions = session_list::view(session_list::SessionListProps {
        vm: app.widgets.tabs.vm(app.host_name.as_deref()),
        theme: theme_props,
        is_open: sidebar_vm.is_sessions_open,
        new_session_title: app.shell_session.name(),
        on_toggle: AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::ToggleSessions,
        )),
        on_intent: |intent| AppEvent::Tabs(TabsEvent::Intent(intent)),
    });

    let launches = sidebar_panel::view(sidebar_panel::SidebarPanelProps {
        vm: app.widgets.quick_launch.vm(),
        theme: theme_props,
        is_open: sidebar_vm.is_launches_open,
        on_toggle: AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::ToggleLaunches,
        )),
        on_intent: |intent| {
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent))
        },
    });

    column![
        sessions,
        divider::horizontal(theme_props.theme.ui()),
        launches
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Right panel content: the Files section.
fn view_right_panel<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let focused_program = app
        .widgets
        .terminal_workspace
        .focused_program(app.widgets.tabs.active_tab_id());

    sidebar_tree::view(sidebar_tree::SidebarTreeProps {
        vm: app.widgets.explorer.vm(),
        shell_name: followed_program(focused_program, app.shell_session.name()),
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
                TabsIntent::OpenTerminalTab {
                    title: app.shell_session.name().to_string(),
                },
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

/// Render the add menu overlay with dismiss layer.
fn view_add_menu_overlay<'a>(
    cursor: iced::Point,
    area_size: Size,
    theme_props: ThemeProps<'a>,
) -> Element<'a, SidebarIntent, Theme, iced::Renderer> {
    let menu = Menu::new(theme_props.theme.ui())
        .item(
            MenuItem::new(i18n::t(Key::MenuCreateTab))
                .on_press(SidebarIntent::AddMenuCreateTab),
        )
        .item(
            MenuItem::new(i18n::t(Key::MenuCreateFolder))
                .on_press(SidebarIntent::AddMenuCreateFolder),
        )
        .item(
            MenuItem::new(i18n::t(Key::MenuCreateQuickLaunch))
                .on_press(SidebarIntent::AddMenuCreateCommand),
        );

    let anchor = anchor_position(
        cursor,
        area_size,
        menu::MENU_WIDTH,
        menu_height_for_items(3),
        spacing::SPACE_2,
    );

    let padding = iced::Padding {
        top: anchor.y,
        left: anchor.x,
        ..iced::Padding::ZERO
    };

    let positioned_menu = container(Element::from(menu))
        .padding(padding)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Top);

    let dismiss_layer = mouse_area(
        container(text("")).width(Length::Fill).height(Length::Fill),
    )
    .on_press(SidebarIntent::DismissAddMenu)
    .on_right_press(SidebarIntent::DismissAddMenu)
    .on_move(|position| SidebarIntent::WorkspaceCursorMoved { position });

    iced::widget::stack!(dismiss_layer, positioned_menu)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
