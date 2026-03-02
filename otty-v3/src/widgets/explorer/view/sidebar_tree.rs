use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Element, Length, alignment};
use otty_ui_tree::{TreeNode, TreeRowContext, TreeView};

use crate::icons::{FILE, FOLDER, FOLDER_OPENED};
use crate::style::{thin_scroll_style, tree_row_style};
use crate::theme::{IcedColorPalette, ThemeProps};
use crate::widgets::explorer::event::ExplorerIntent;
use crate::widgets::explorer::model::{ExplorerTreeViewModel, FileNode};

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
) -> Element<'_, ExplorerIntent, iced::Theme, iced::Renderer> {
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
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
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
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
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
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
    let icon_color = palette.dim_foreground;
    let tree_view = TreeView::new(vm.tree, move |context| {
        render_entry(context, icon_color)
    })
    .selected_row(vm.selected_path)
    .hovered_row(vm.hovered_path)
    .on_press(|path| ExplorerIntent::NodePressed { path })
    .on_hover(|path| ExplorerIntent::NodeHovered { path })
    .row_style(move |context| nav_row_style(palette, context))
    .indent_size(TREE_INDENT)
    .spacing(0.0);

    let scrollable = scrollable::Scrollable::new(tree_view.view())
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .style(thin_scroll_style(palette.clone()));

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn render_entry<'a>(
    context: &TreeRowContext<'a, FileNode>,
    icon_color: iced::Color,
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
    let icon_data = if context.entry.node.is_folder() {
        if context.entry.node.is_expanded() {
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
        text(context.entry.node.name())
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

    container(
        row![leading, title]
            .spacing(TREE_ROW_SPACING)
            .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(TREE_ROW_HEIGHT))
    .padding([0.0, TREE_ROW_PADDING_X])
    .into()
}

fn nav_row_style(
    palette: &IcedColorPalette,
    context: &TreeRowContext<'_, FileNode>,
) -> iced::widget::container::Style {
    tree_row_style(palette, context.is_selected, context.is_hovered)
}

impl TreeNode for FileNode {
    fn title(&self) -> &str {
        self.name()
    }

    fn children(&self) -> Option<&[Self]> {
        if self.is_folder() {
            Some(self.children())
        } else {
            None
        }
    }

    fn expanded(&self) -> bool {
        self.is_expanded()
    }

    fn is_folder(&self) -> bool {
        self.is_folder()
    }
}
