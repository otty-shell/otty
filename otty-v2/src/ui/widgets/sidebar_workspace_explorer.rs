use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Element, Length, alignment};
use otty_ui_tree::{TreeNode, TreeRowContext, TreeView};

use super::services as helpers;
use crate::icons;
use crate::theme::{IcedColorPalette, ThemeProps};
use crate::widgets::explorer::{ExplorerFeature, ExplorerUiEvent, FileNode};

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

/// Props for rendering the explorer workspace.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarWorkspaceExplorerProps<'a> {
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) explorer: &'a ExplorerFeature,
}

/// Events emitted by sidebar workspace explorer widget.
pub(crate) type SidebarWorkspaceExplorerEvent = ExplorerUiEvent;

pub(crate) fn view<'a>(
    props: SidebarWorkspaceExplorerProps<'a>,
) -> Element<'a, SidebarWorkspaceExplorerEvent, iced::Theme, iced::Renderer> {
    let header = explorer_header(props.theme);
    let current_dir = current_directory_bar(props);
    let tree_list = explorer_tree(props);

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

fn explorer_header<'a>(
    theme: ThemeProps<'a>,
) -> Element<'a, SidebarWorkspaceExplorerEvent> {
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
            text_color: Some(theme.theme.iced_palette().foreground),
            ..Default::default()
        })
        .into()
}

fn current_directory_bar<'a>(
    props: SidebarWorkspaceExplorerProps<'a>,
) -> Element<'a, SidebarWorkspaceExplorerEvent> {
    let label = props.explorer.root_label().unwrap_or("No active folder");

    let text = text(label)
        .size(BAR_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    let palette = props.theme.theme.iced_palette().clone();

    container(text)
        .width(Length::Fill)
        .height(Length::Fixed(BAR_HEIGHT))
        .padding([0.0, BAR_PADDING_X])
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .style(move |_| iced::widget::container::Style {
            background: Some(palette.overlay.into()),
            text_color: Some(palette.foreground),
            ..Default::default()
        })
        .into()
}

fn explorer_tree<'a>(
    props: SidebarWorkspaceExplorerProps<'a>,
) -> Element<'a, SidebarWorkspaceExplorerEvent> {
    let row_props = props;
    let row_style_props = props;

    let palette = props.theme.theme.iced_palette().clone();
    let row_palette = palette.clone();

    let tree_view = TreeView::new(props.explorer.tree(), move |context| {
        render_entry(row_props, context)
    })
    .selected_row(props.explorer.selected_path())
    .hovered_row(props.explorer.hovered_path())
    .on_press(|path| ExplorerUiEvent::NodePressed { path })
    .on_hover(|path| ExplorerUiEvent::NodeHovered { path })
    .row_style(move |context| {
        tree_row_style(row_style_props, &row_palette, context)
    })
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
        .style(helpers::thin_scroll_style(palette));

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn render_entry<'a>(
    props: SidebarWorkspaceExplorerProps<'a>,
    context: &TreeRowContext<'a, FileNode>,
) -> Element<'a, SidebarWorkspaceExplorerEvent> {
    let icon_palette = props.theme.theme.iced_palette();
    let icon_view: Element<'a, SidebarWorkspaceExplorerEvent> =
        if context.entry.node.is_folder() {
            let icon = if context.entry.node.is_expanded() {
                icons::FOLDER_OPENED
            } else {
                icons::FOLDER
            };
            svg_icon(icon, icon_palette.dim_foreground)
        } else {
            svg_icon(icons::FILE, icon_palette.dim_foreground)
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

    let content = row![leading, title]
        .spacing(TREE_ROW_SPACING)
        .align_y(alignment::Vertical::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fixed(TREE_ROW_HEIGHT))
        .padding([0.0, TREE_ROW_PADDING_X])
        .into()
}

fn tree_row_style(
    _props: SidebarWorkspaceExplorerProps<'_>,
    palette: &IcedColorPalette,
    context: &TreeRowContext<'_, FileNode>,
) -> iced::widget::container::Style {
    helpers::tree_row_style(palette, context.is_selected, context.is_hovered)
}

fn svg_icon<'a>(
    icon: &'static [u8],
    color: iced::Color,
) -> Element<'a, SidebarWorkspaceExplorerEvent> {
    let handle = svg::Handle::from_memory(icon);
    let svg_icon = svg::Svg::new(handle)
        .width(Length::Fixed(TREE_ICON_WIDTH))
        .height(Length::Fixed(TREE_ICON_WIDTH))
        .style(move |_, _| svg::Style { color: Some(color) });
    container(svg_icon)
        .width(Length::Fixed(TREE_ICON_WIDTH))
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
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
