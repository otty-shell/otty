use iced::widget::{Space, column, container, mouse_area, pane_grid, row};
use iced::{Element, Length, Size, Theme};

use super::{App, Event};
use crate::features::sidebar::{SIDEBAR_MENU_WIDTH, SidebarItem, SidebarPane};
use crate::theme::ThemeProps;
use crate::ui::components::{resize_grips, sidebar_workspace_panel};
use crate::ui::widgets::{
    action_bar, quick_launches_context_menu, sidebar_menu, sidebar_workspace,
    sidebar_workspace_add_menu, tab_bar, tab_content,
    terminal_pane_context_menu,
};

const HEADER_SEPARATOR_HEIGHT: f32 = 1.0;
const SIDEBAR_SEPARATOR_WIDTH: f32 = 0.3;
const SEPARATOR_ALPHA: f32 = 0.3;

pub(super) fn view(app: &App) -> Element<'_, Event, Theme, iced::Renderer> {
    let theme = app.theme_manager.current();
    let theme_props: ThemeProps<'_> = ThemeProps::new(theme);

    let tab_summaries = app.features.tab().tab_summaries();
    let active_tab_id = app.features.tab().active_tab_id().unwrap_or(0);

    let header = view_header(app, theme_props);
    let sidebar = app.features.sidebar();

    let content_row: Element<'_, Event, Theme, iced::Renderer> =
        if sidebar.is_hidden() {
            view_content_only(app, theme_props, &tab_summaries, active_tab_id)
        } else {
            view_sidebar_layout(app, theme_props, &tab_summaries, active_tab_id)
        };

    let content_row = mouse_area(content_row).on_move(|position| {
        Event::SidebarWorkspace(
            sidebar_workspace::SidebarWorkspaceEvent::WorkspaceCursorMoved {
                position,
            },
        )
    });

    let mut content_layers: Vec<Element<'_, Event, Theme, iced::Renderer>> =
        vec![content_row.into()];

    if let Some(overlay) = view_context_menu_overlay(app, theme_props) {
        content_layers.push(overlay);
    }

    let content_stack = iced::widget::Stack::with_children(content_layers)
        .width(Length::Fill)
        .height(Length::Fill);

    let resize_grips_layer = if super::update::any_context_menu_open(app) {
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        resize_grips::view()
    };

    let root_layers: Vec<Element<'_, Event, Theme, iced::Renderer>> = vec![
        column![header, content_stack]
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

/// Render the action bar and its bottom separator line.
fn view_header<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, Event, Theme, iced::Renderer> {
    let header = action_bar::view(action_bar::ActionBarProps {
        title: app.features.tab().active_tab_title().unwrap_or("OTTY"),
        theme: theme_props,
        fonts: &app.fonts,
    })
    .map(Event::ActionBar);

    let palette = theme_props.theme.iced_palette();
    let separator = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(HEADER_SEPARATOR_HEIGHT))
        .style(move |_| {
            let mut background = palette.dim_white;
            background.a = SEPARATOR_ALPHA;
            iced::widget::container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        });

    column![header, separator]
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
}

/// Render the tab bar + content area when the sidebar is hidden.
fn view_content_only<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
    tab_summaries: &[(u64, &'a str)],
    active_tab_id: u64,
) -> Element<'a, Event, Theme, iced::Renderer> {
    let tab_bar = tab_bar::view(tab_bar::TabBarProps {
        tabs: tab_summaries.to_vec(),
        active_tab_id,
        theme: theme_props,
    })
    .map(|e| match e {
        tab_bar::TabBarEvent::ActivateTab { tab_id } => {
            Event::ActivateTab { tab_id }
        },
        tab_bar::TabBarEvent::CloseTab { tab_id } => {
            Event::CloseTabRequested { tab_id }
        },
    });

    let content = tab_content::view(tab_content::TabContentProps {
        active_tab: app.features.tab().active_tab(),
        terminal: app.features.terminal().state(),
        quick_launch_wizard: app.features.quick_launch_wizard().state(),
        quick_launches: app.features.quick_launch().state(),
        settings: app.features.settings().state(),
        theme: theme_props,
    })
    .map(map_tab_content_event);

    container(
        column![tab_bar, content]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Render the sidebar menu + workspace split + content area.
fn view_sidebar_layout<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
    tab_summaries: &[(u64, &'a str)],
    active_tab_id: u64,
) -> Element<'a, Event, Theme, iced::Renderer> {
    let sidebar_menu = sidebar_menu::view(sidebar_menu::SidebarMenuProps {
        active_item: match app.features.sidebar().active_item() {
            SidebarItem::Terminal => sidebar_menu::SidebarMenuItem::Terminal,
            SidebarItem::Explorer => sidebar_menu::SidebarMenuItem::Explorer,
        },
        workspace_open: app.features.sidebar().is_workspace_open(),
        menu_width: SIDEBAR_MENU_WIDTH,
        theme: theme_props,
    })
    .map(Event::Sidebar);

    let palette = theme_props.theme.iced_palette();
    let sidebar_separator = container(Space::new())
        .width(Length::Fixed(SIDEBAR_SEPARATOR_WIDTH))
        .height(Length::Fill)
        .style(move |_| {
            let mut background = palette.dim_white;
            background.a = SEPARATOR_ALPHA;
            iced::widget::container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        });

    let sidebar = app.features.sidebar();
    let explorer_feature = app.features.explorer();
    let terminal_state = app.features.terminal().state();
    let quick_launches_state = app.features.quick_launch().state();
    let wizard_state = app.features.quick_launch_wizard().state();
    let settings_state = app.features.settings().state();
    let active_tab = app.features.tab().active_tab();
    let workspace_open = sidebar.is_workspace_open();
    let active_item = sidebar.active_item();

    let sidebar_split = pane_grid::PaneGrid::new(
        sidebar.panes(),
        move |_, pane, _| match pane {
            SidebarPane::Workspace => {
                let workspace_content = sidebar_workspace::view(
                    sidebar_workspace::SidebarWorkspaceProps {
                        active_item: match active_item {
                            SidebarItem::Terminal => {
                                sidebar_workspace::SidebarWorkspaceItem::Terminal
                            },
                            SidebarItem::Explorer => {
                                sidebar_workspace::SidebarWorkspaceItem::Explorer
                            },
                        },
                        quick_launches: quick_launches_state,
                        explorer: explorer_feature,
                        theme: theme_props,
                    },
                )
                .map(Event::SidebarWorkspace);
                let workspace = sidebar_workspace_panel::view(
                    sidebar_workspace_panel::SidebarWorkspacePanelProps {
                        content: workspace_content,
                        visible: workspace_open,
                        theme: theme_props,
                    },
                );
                pane_grid::Content::new(workspace)
            },
            SidebarPane::Content => {
                let tab_bar = tab_bar::view(tab_bar::TabBarProps {
                    tabs: tab_summaries.to_vec(),
                    active_tab_id,
                    theme: theme_props,
                })
                .map(|e| match e {
                    tab_bar::TabBarEvent::ActivateTab { tab_id } => {
                        Event::ActivateTab { tab_id }
                    },
                    tab_bar::TabBarEvent::CloseTab { tab_id } => {
                        Event::CloseTabRequested { tab_id }
                    },
                });

                let content = tab_content::view(tab_content::TabContentProps {
                    active_tab,
                    terminal: terminal_state,
                    quick_launch_wizard: wizard_state,
                    quick_launches: quick_launches_state,
                    settings: settings_state,
                    theme: theme_props,
                })
                .map(map_tab_content_event);

                pane_grid::Content::new(
                    container(
                        column![tab_bar, content]
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill),
                )
            },
        },
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(0)
    .min_size(0)
    .on_resize(10.0, |event| {
        Event::Sidebar(sidebar_menu::SidebarMenuEvent::Resized(event))
    });

    row![sidebar_menu, sidebar_separator, sidebar_split]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render the context menu overlay layer, if any menu is open.
fn view_context_menu_overlay<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Option<Element<'a, Event, Theme, iced::Renderer>> {
    context_menu_layer(app, theme_props, app.state.screen_size)
}

fn context_menu_layer<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
    area_size: Size,
) -> Option<Element<'a, Event, Theme, iced::Renderer>> {
    if let Some(cursor) = app.features.sidebar().add_menu_cursor() {
        return Some(
            sidebar_workspace_add_menu::view(
                sidebar_workspace_add_menu::SidebarWorkspaceAddMenuProps {
                    cursor,
                    theme,
                    area_size,
                },
            )
            .map(Event::SidebarWorkspace),
        );
    }

    if let Some(menu) = app.features.quick_launch().context_menu() {
        return Some(
            quick_launches_context_menu::view(
                quick_launches_context_menu::QuickLaunchesContextMenuProps {
                    menu,
                    theme,
                    area_size,
                    launching: app.features.quick_launch().launching(),
                },
            )
            .map(|event| {
                Event::SidebarWorkspace(
                    sidebar_workspace::SidebarWorkspaceEvent::QuickLaunch(
                        event,
                    ),
                )
            }),
        );
    }

    for (&tab_id, terminal) in app.features.terminal().tabs() {
        if let Some(menu) = terminal.context_menu() {
            let has_block_selection =
                terminal.selected_block_terminal() == Some(menu.terminal_id());
            return Some(
                terminal_pane_context_menu::view(
                    terminal_pane_context_menu::TerminalPaneContextMenuProps {
                        tab_id,
                        pane: menu.pane(),
                        cursor: menu.cursor(),
                        grid_size: menu.grid_size(),
                        terminal_id: menu.terminal_id(),
                        focus_target: menu.focus_target().clone(),
                        has_block_selection,
                        theme,
                    },
                )
                .map(Event::Terminal),
            );
        }
    }

    None
}

fn map_tab_content_event(event: tab_content::TabContentEvent) -> Event {
    match event {
        tab_content::TabContentEvent::Terminal(event) => Event::Terminal(event),
        tab_content::TabContentEvent::Settings(event) => Event::Settings(event),
        tab_content::TabContentEvent::QuickLaunchWizard { tab_id, event } => {
            Event::QuickLaunchWizard { tab_id, event }
        },
        tab_content::TabContentEvent::QuickLaunchError(event) => match event {},
    }
}

pub(super) fn screen_size_from_window(window_size: Size) -> Size {
    let action_bar_height = action_bar::ACTION_BAR_HEIGHT;
    let height =
        (window_size.height - action_bar_height - SIDEBAR_SEPARATOR_WIDTH)
            .max(0.0);
    Size::new(window_size.width, height)
}
