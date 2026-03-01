use iced::widget::pane_grid::PaneGrid;
use iced::widget::{Space, column, container, mouse_area, row, text};
use iced::{Element, Length, Size, Theme, alignment};

use super::{App, AppEvent};
use crate::components::primitive::{
    menu_item, resize_grips, sidebar_workspace_panel,
};
use crate::shared::ui::theme::ThemeProps;
use crate::shared::ui::{menu_geometry, menu_style};
use crate::widgets::chrome::ChromeEvent;
use crate::widgets::chrome::view::action_bar::{self, ACTION_BAR_HEIGHT};
use crate::widgets::explorer::view::sidebar_tree;
use crate::widgets::quick_launch::QuickLaunchEvent;
use crate::widgets::quick_launch::view::{
    context_menu as ql_context_menu, error_tab, sidebar_panel, wizard_form,
};
use crate::widgets::settings::view::settings_form;
use crate::widgets::sidebar;
use crate::widgets::sidebar::{SidebarItem, SidebarPane};
use crate::widgets::tabs::model::TabContent;
use crate::widgets::tabs::view::tab_bar;
use crate::widgets::terminal_workspace::view::{
    pane_context_menu as terminal_pane_context_menu,
    pane_grid as terminal_pane_grid,
};

const HEADER_SEPARATOR_HEIGHT: f32 = 1.0;
const SEPARATOR_ALPHA: f32 = 0.3;
const PANE_GRID_SPACING: f32 = 1.0;
const PANE_GRID_RESIZE_GRAB: f32 = 8.0;

// Add menu overlay constants
const ADD_MENU_WIDTH: f32 = 220.0;
const ADD_MENU_ITEM_HEIGHT: f32 = 24.0;
const ADD_MENU_VERTICAL_PADDING: f32 = 16.0;
const ADD_MENU_MARGIN: f32 = 6.0;
const ADD_MENU_CONTAINER_PADDING_X: f32 = 8.0;

/// Render the root application view.
pub(super) fn view(app: &App) -> Element<'_, AppEvent, Theme, iced::Renderer> {
    let theme = app.theme_manager.current();
    let theme_props: ThemeProps<'_> = ThemeProps::new(theme);

    let sidebar_vm = app.widgets.sidebar.vm();

    let content_row: Element<'_, AppEvent, Theme, iced::Renderer> =
        if sidebar_vm.is_hidden {
            view_content_only(app, theme_props)
        } else {
            view_sidebar_layout(app, theme_props)
        };

    let content_row = mouse_area(content_row).on_move(|position| {
        AppEvent::SidebarUi(sidebar::SidebarEvent::WorkspaceCursorMoved {
            position,
        })
    });

    let mut layers: Vec<Element<'_, AppEvent, Theme, iced::Renderer>> =
        vec![content_row.into()];

    // Add menu overlay
    if sidebar_vm.has_add_menu_open {
        if let Some(cursor) = sidebar_vm.add_menu_cursor {
            layers.push(
                view_add_menu_overlay(
                    cursor,
                    app.state.screen_size,
                    theme_props,
                )
                .map(AppEvent::SidebarUi),
            );
        }
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
            .map(|event| AppEvent::QuickLaunch(QuickLaunchEvent::Ui(event))),
        );
    }

    if let Some(overlay) = view_terminal_context_menu_overlay(app, theme_props)
    {
        layers.push(overlay);
    }

    let content_stack = iced::widget::Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill);

    let resize_grips_layer = if app.widgets.sidebar.has_add_menu_open()
        || app.widgets.quick_launch.context_menu().is_some()
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

    let header = view_header(app, theme_props);

    let root_layers: Vec<Element<'_, AppEvent, Theme, iced::Renderer>> = vec![
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

/// Render the header: action bar + separator.
fn view_header<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let palette = theme_props.theme.iced_palette();

    let action_bar = action_bar::view(action_bar::ActionBarProps {
        title: app.widgets.tabs.active_tab_title().unwrap_or("OTTY"),
        theme: theme_props,
        fonts: &app.fonts,
    })
    .map(|event| AppEvent::Chrome(ChromeEvent::Ui(event)));

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

    column![action_bar, separator]
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
}

/// Render content without sidebar.
fn view_content_only<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    view_tab_area(app, theme_props)
}

/// Render sidebar + content split layout with PaneGrid.
fn view_sidebar_layout<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let sidebar_vm = app.widgets.sidebar.vm();

    let menu_rail = sidebar::view::view(sidebar::view::SidebarViewProps {
        vm: sidebar_vm,
        theme: theme_props,
    })
    .map(AppEvent::SidebarUi);

    let palette = theme_props.theme.iced_palette();

    let sidebar_pane_grid =
        PaneGrid::new(app.widgets.sidebar.panes(), move |_pane, pane_kind, _| {
            let content: Element<'_, AppEvent, Theme, iced::Renderer> =
                match *pane_kind {
                    SidebarPane::Workspace => {
                        let workspace_content =
                            view_workspace_content(app, sidebar_vm, theme_props);
                        sidebar_workspace_panel::view(
                            sidebar_workspace_panel::SidebarWorkspacePanelProps {
                                content: workspace_content,
                                visible: sidebar_vm.is_workspace_open,
                                theme: theme_props,
                            },
                        )
                    },
                    SidebarPane::Content => view_tab_area(app, theme_props),
                };
            iced::widget::pane_grid::Content::new(content)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(PANE_GRID_SPACING)
        .on_resize(PANE_GRID_RESIZE_GRAB, |event| {
            AppEvent::SidebarUi(sidebar::SidebarEvent::Resized(event))
        })
        .style(move |_: &Theme| {
            let mut separator = palette.dim_white;
            separator.a = SEPARATOR_ALPHA;

            iced::widget::pane_grid::Style {
                hovered_region: iced::widget::pane_grid::Highlight {
                    background: separator.into(),
                    border: iced::Border::default(),
                },
                picked_split: iced::widget::pane_grid::Line {
                    color: separator,
                    width: 1.0,
                },
                hovered_split: iced::widget::pane_grid::Line {
                    color: separator,
                    width: 1.0,
                },
            }
        });

    row![menu_rail, sidebar_pane_grid]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render workspace content based on the active sidebar item.
fn view_workspace_content<'a>(
    app: &'a App,
    sidebar_vm: sidebar::SidebarViewModel,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    match sidebar_vm.active_item {
        SidebarItem::Terminal => {
            sidebar_panel::view(sidebar_panel::SidebarPanelProps {
                vm: app.widgets.quick_launch.tree_vm(),
                theme: theme_props,
            })
            .map(|event| AppEvent::QuickLaunch(QuickLaunchEvent::Ui(event)))
        },
        SidebarItem::Explorer => {
            sidebar_tree::view(sidebar_tree::SidebarTreeProps {
                vm: app.widgets.explorer.tree_vm(),
                theme: theme_props,
            })
            .map(AppEvent::ExplorerUi)
        },
    }
}

/// Render the tab bar + tab content area.
fn view_tab_area<'a>(
    app: &'a App,
    theme_props: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let tabs_vm = app.widgets.tabs.vm();

    let tab_bar_props = tab_bar::TabBarProps {
        tabs: tabs_vm.tabs,
        active_tab_id: tabs_vm.active_tab_id,
        theme: theme_props,
    };

    let tab_bar = tab_bar::view(tab_bar_props).map(AppEvent::TabsUi);

    let content = view_tab_content(app, theme_props);

    column![tab_bar, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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
            let vm = app.widgets.terminal_workspace.vm(Some(tab_id));
            match vm.tab {
                Some(tab_vm) => terminal_pane_grid::view(tab_vm)
                    .map(AppEvent::TerminalWorkspaceUi),
                None => missing_tab_state("Terminal tab is not initialized."),
            }
        },
        (Some(_tab_id), Some(TabContent::Settings)) => {
            settings_form::view(settings_form::SettingsFormProps {
                vm: app.widgets.settings.vm(),
                theme: theme_props,
            })
            .map(AppEvent::SettingsUi)
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
                        AppEvent::QuickLaunch(QuickLaunchEvent::Ui(event))
                    })
                },
                None => {
                    missing_tab_state("Quick launch editor is not initialized.")
                },
            }
        },
        (Some(tab_id), Some(TabContent::QuickLaunchError)) => {
            match app.widgets.quick_launch.error_tab(tab_id) {
                Some(error) => error_tab::view(error_tab::ErrorTabProps {
                    error,
                    theme: theme_props,
                })
                .map(|event| {
                    AppEvent::QuickLaunch(QuickLaunchEvent::Ui(event))
                }),
                None => {
                    missing_tab_state("Quick launch error payload is missing.")
                },
            }
        },
        _ => container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
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
            .map(AppEvent::TerminalWorkspaceUi),
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
) -> Element<'a, sidebar::SidebarEvent, Theme, iced::Renderer> {
    let menu_items = [
        add_menu_item(
            "Create tab",
            theme_props,
            sidebar::SidebarEvent::AddMenuCreateTab,
        ),
        add_menu_item(
            "Create command",
            theme_props,
            sidebar::SidebarEvent::AddMenuCreateCommand,
        ),
        add_menu_item(
            "Create folder",
            theme_props,
            sidebar::SidebarEvent::AddMenuCreateFolder,
        ),
    ];

    let menu_height = menu_geometry::menu_height_for_items(
        menu_items.len(),
        ADD_MENU_ITEM_HEIGHT,
        ADD_MENU_VERTICAL_PADDING,
    );

    let menu_column = menu_items
        .into_iter()
        .fold(iced::widget::Column::new(), |col, item| col.push(item))
        .spacing(0)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    let anchor = menu_geometry::anchor_position(
        cursor,
        area_size,
        ADD_MENU_WIDTH,
        menu_height,
        ADD_MENU_MARGIN,
    );

    let padding = iced::Padding {
        top: anchor.y,
        left: anchor.x,
        ..iced::Padding::ZERO
    };

    let menu_container = container(menu_column)
        .padding([ADD_MENU_CONTAINER_PADDING_X, 0.0])
        .width(ADD_MENU_WIDTH)
        .style(menu_style::menu_panel_style(theme_props));

    let positioned_menu = container(menu_container)
        .padding(padding)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Top);

    let dismiss_layer = mouse_area(
        container(text("")).width(Length::Fill).height(Length::Fill),
    )
    .on_press(sidebar::SidebarEvent::DismissAddMenu)
    .on_right_press(sidebar::SidebarEvent::DismissAddMenu)
    .on_move(|position| sidebar::SidebarEvent::WorkspaceCursorMoved {
        position,
    });

    iced::widget::stack!(dismiss_layer, positioned_menu)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Create a single add menu item.
fn add_menu_item<'a>(
    label: &'a str,
    theme_props: ThemeProps<'a>,
    on_press: sidebar::SidebarEvent,
) -> Element<'a, sidebar::SidebarEvent, Theme, iced::Renderer> {
    menu_item::view(menu_item::MenuItemProps {
        label,
        theme: theme_props,
    })
    .map(move |event| match event {
        menu_item::MenuItemEvent::Pressed => on_press.clone(),
    })
}

/// Compute screen size from window size by subtracting header.
pub(crate) fn screen_size_from_window(window_size: Size) -> Size {
    let height =
        (window_size.height - ACTION_BAR_HEIGHT - HEADER_SEPARATOR_HEIGHT)
            .max(0.0);
    Size::new(window_size.width, height)
}
