use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Element, Length, alignment};

use crate::shared::ui::icons::{FILE, FOLDER, FOLDER_OPENED};
use crate::shared::ui::theme::{IcedColorPalette, ThemeProps};
use crate::shared::ui::tree_style;
use crate::widgets::explorer::event::ExplorerEvent;
use crate::widgets::explorer::model::{
    ExplorerTreeViewModel, FileNode, TreePath,
};

const HEADER_HEIGHT: f32 = 22.0;
const HEADER_PADDING_X: f32 = 10.0;
const HEADER_FONT_SIZE: f32 = 12.0;

const BAR_HEIGHT: f32 = 28.0;
const BAR_PADDING_X: f32 = 10.0;
const BAR_FONT_SIZE: f32 = 12.0;

const TREE_ROW_HEIGHT: f32 = 24.0;
const TREE_FONT_SIZE: f32 = 12.0;
const TREE_INDENT: f32 = 14.0;
const TREE_ICON_WIDTH: f32 = 14.0;
const TREE_ROW_PADDING_X: f32 = 6.0;
const TREE_ROW_SPACING: f32 = 6.0;

const WORKSPACE_PADDING_HORIZONTAL: f32 = 0.0;
const WORKSPACE_PADDING_VERTICAL: f32 = 10.0;

/// Props for the explorer sidebar tree view.
#[derive(Debug, Clone)]
pub(crate) struct SidebarTreeProps<'a> {
    pub(crate) vm: ExplorerTreeViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the full explorer sidebar panel (header + directory bar + tree).
pub(crate) fn view(
    props: SidebarTreeProps<'_>,
) -> Element<'_, ExplorerEvent, iced::Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette();

    let header = explorer_header(palette);
    let current_dir = current_directory_bar(props.vm.root_label, palette);
    let tree_list = explorer_tree(&props.vm, palette);

    let content = column![header, current_dir, tree_list]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([WORKSPACE_PADDING_VERTICAL, WORKSPACE_PADDING_HORIZONTAL])
        .into()
}

/// Render the "EXPLORER" section header.
fn explorer_header<'a>(
    palette: &'a IcedColorPalette,
) -> Element<'a, ExplorerEvent, iced::Theme, iced::Renderer> {
    let foreground = palette.foreground;

    let title = text("EXPLORER")
        .size(HEADER_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    container(title)
        .width(Length::Fill)
        .height(Length::Fixed(HEADER_HEIGHT))
        .padding([0.0, HEADER_PADDING_X])
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .style(move |_| iced::widget::container::Style {
            background: None,
            text_color: Some(foreground),
            ..Default::default()
        })
        .into()
}

/// Render the current directory label bar.
fn current_directory_bar<'a>(
    root_label: Option<&'a str>,
    palette: &'a IcedColorPalette,
) -> Element<'a, ExplorerEvent, iced::Theme, iced::Renderer> {
    let label = root_label.unwrap_or("No active folder");
    let overlay = palette.overlay;
    let foreground = palette.foreground;

    let label_text = text(label)
        .size(BAR_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    container(label_text)
        .width(Length::Fill)
        .height(Length::Fixed(BAR_HEIGHT))
        .padding([0.0, BAR_PADDING_X])
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .style(move |_| iced::widget::container::Style {
            background: Some(overlay.into()),
            text_color: Some(foreground),
            ..Default::default()
        })
        .into()
}

/// Render the scrollable explorer tree.
fn explorer_tree<'a>(
    vm: &ExplorerTreeViewModel<'a>,
    palette: &'a IcedColorPalette,
) -> Element<'a, ExplorerEvent, iced::Theme, iced::Renderer> {
    let mut entries: Vec<
        Element<'_, ExplorerEvent, iced::Theme, iced::Renderer>,
    > = Vec::new();

    render_children(vm.tree, &[], 0, vm, palette, &mut entries);

    if entries.is_empty() {
        entries.push(
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );
    }

    let content = column(entries).width(Length::Fill).spacing(0);

    let scrollable = scrollable::Scrollable::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .style(tree_style::thin_scroll_style(palette.clone()));

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Recursively render tree children.
fn render_children<'a>(
    children: &'a [FileNode],
    parent_path: &[String],
    depth: usize,
    vm: &ExplorerTreeViewModel<'a>,
    palette: &'a IcedColorPalette,
    entries: &mut Vec<Element<'a, ExplorerEvent, iced::Theme, iced::Renderer>>,
) {
    for node in children {
        let mut path = parent_path.to_vec();
        path.push(node.name().to_string());

        let is_selected = vm.selected_path.map(|s| s == &path).unwrap_or(false);
        let is_hovered = vm.hovered_path.map(|h| h == &path).unwrap_or(false);

        entries.push(render_tree_row(
            node,
            &path,
            depth,
            is_selected,
            is_hovered,
            palette,
        ));

        // Recurse into expanded folders.
        if node.is_folder() && node.is_expanded() {
            render_children(
                node.children(),
                &path,
                depth + 1,
                vm,
                palette,
                entries,
            );
        }
    }
}

/// Render a single tree row with icon, label, and interaction events.
fn render_tree_row<'a>(
    node: &'a FileNode,
    path: &TreePath,
    depth: usize,
    is_selected: bool,
    is_hovered: bool,
    palette: &'a IcedColorPalette,
) -> Element<'a, ExplorerEvent, iced::Theme, iced::Renderer> {
    let indent = depth as f32 * TREE_INDENT;

    let icon_color = palette.dim_foreground;
    let icon_data = if node.is_folder() {
        if node.is_expanded() {
            FOLDER_OPENED
        } else {
            FOLDER
        }
    } else {
        FILE
    };

    let icon_view = {
        let handle = svg::Handle::from_memory(icon_data);
        let svg_icon = svg::Svg::new(handle)
            .width(Length::Fixed(TREE_ICON_WIDTH))
            .height(Length::Fixed(TREE_ICON_WIDTH))
            .style(move |_, _| svg::Style {
                color: Some(icon_color),
            });
        container(svg_icon)
            .width(Length::Fixed(TREE_ICON_WIDTH))
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
    };

    let title = container(
        text(node.name())
            .size(TREE_FONT_SIZE)
            .width(Length::Fill)
            .wrapping(Wrapping::None)
            .align_x(alignment::Horizontal::Left),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(alignment::Vertical::Center);

    let leading = row![icon_view]
        .spacing(0)
        .align_y(alignment::Vertical::Center);

    let content = row![
        iced::widget::Space::new().width(Length::Fixed(indent)),
        leading,
        title,
    ]
    .spacing(TREE_ROW_SPACING)
    .align_y(alignment::Vertical::Center);

    let row_style =
        tree_style::tree_row_style(palette, is_selected, is_hovered);

    let styled_row = container(content)
        .width(Length::Fill)
        .height(Length::Fixed(TREE_ROW_HEIGHT))
        .padding([0.0, TREE_ROW_PADDING_X])
        .style(move |_| row_style);

    let path_for_hover = path.clone();
    let path_for_press = path.clone();

    let interactive = iced::widget::mouse_area(styled_row)
        .on_enter(ExplorerEvent::NodeHovered {
            path: Some(path_for_hover),
        })
        .on_press(ExplorerEvent::NodePressed {
            path: path_for_press,
        });

    interactive.into()
}
