use iced::widget::{
    Space, column, container, mouse_area, row, scrollable, svg, text,
    text_input,
};
use iced::{Color, Element, Length, Theme, alignment};
use otty_ui_tree::{TreeNode, TreeRowContext, TreeView};

use crate::icons::{FOLDER, FOLDER_OPENED, PLAY};
use crate::style::{thin_scroll_style, tree_row_style};
use crate::theme::ThemeProps;
use crate::widgets::quick_launch::event::QuickLaunchIntent;
use crate::widgets::quick_launch::model::{
    LaunchInfo, NodePath, QuickLaunchFile, QuickLaunchNode,
};
use crate::widgets::quick_launch::state::{
    DropTarget, InlineEditKind, InlineEditState,
};

const TREE_ROW_HEIGHT: f32 = 24.0;
const TREE_FONT_SIZE: f32 = 12.0;
const TREE_INDENT: f32 = 14.0;
const TREE_ICON_WIDTH: f32 = 14.0;
const TREE_ROW_PADDING_X: f32 = 6.0;
const TREE_ROW_SPACING: f32 = 6.0;

/// Props for the quick launch tree view.
#[derive(Debug, Clone)]
pub(crate) struct SidebarTreeProps<'a> {
    pub(crate) data: &'a QuickLaunchFile,
    pub(crate) selected_path: Option<&'a NodePath>,
    pub(crate) hovered_path: Option<&'a NodePath>,
    pub(crate) inline_edit: Option<&'a InlineEditState>,
    pub(crate) launching: &'a std::collections::HashMap<NodePath, LaunchInfo>,
    pub(crate) drop_target: Option<&'a DropTarget>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch sidebar tree.
pub(crate) fn view(
    props: SidebarTreeProps<'_>,
) -> Element<'_, QuickLaunchIntent, Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette().clone();
    let dim_foreground = palette.dim_foreground;
    let foreground = palette.foreground;
    let indicator_color = palette.green;
    let error_color = palette.red;
    let overlay = palette.overlay;
    let row_palette = palette.clone();

    let launching = props.launching;
    let drop_target = props.drop_target;
    let inline_edit = props.inline_edit;

    let tree_view =
        TreeView::new(props.data.root().children(), move |context| {
            render_entry(
                context,
                launching,
                dim_foreground,
                foreground,
                indicator_color,
            )
        })
        .selected_row(props.selected_path)
        .hovered_row(props.hovered_path)
        .on_press(|path| QuickLaunchIntent::NodePressed { path })
        .on_release(|path| QuickLaunchIntent::NodeReleased { path })
        .on_right_press(|path| QuickLaunchIntent::NodeRightClicked { path })
        .row_style(move |context| {
            quick_launch_row_style(drop_target, &row_palette, context)
        })
        .row_visible_filter(move |context| {
            !is_rename_edit(inline_edit, context)
        })
        .after_row(move |context| {
            inline_edit_after(inline_edit, context, error_color)
        })
        .indent_size(TREE_INDENT)
        .spacing(0.0);

    let tree_content =
        if let Some(root_edit) = inline_edit_root(inline_edit, error_color) {
            column![tree_view.view(), root_edit].into()
        } else {
            tree_view.view()
        };

    let scrollable = scrollable::Scrollable::new(tree_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .style(thin_scroll_style(palette.clone()));

    let tree_area = mouse_area(scrollable)
        .on_move(|position| QuickLaunchIntent::CursorMoved { position })
        .on_press(QuickLaunchIntent::BackgroundPressed)
        .on_release(QuickLaunchIntent::BackgroundReleased)
        .on_right_press(QuickLaunchIntent::BackgroundRightClicked);

    let root_drop_background = if matches!(drop_target, Some(DropTarget::Root))
    {
        let mut color = overlay;
        color.a = 0.6;
        Some(color)
    } else {
        None
    };

    container(tree_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: root_drop_background.map(Into::into),
            ..Default::default()
        })
        .into()
}

fn render_entry<'a>(
    context: &TreeRowContext<'a, QuickLaunchNode>,
    launching: &std::collections::HashMap<NodePath, LaunchInfo>,
    dim_foreground: Color,
    foreground: Color,
    indicator_color: Color,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let is_indicator_highlighted = launching
        .get(&context.entry.path)
        .map(|launch| launch.is_indicator_highlighted)
        .unwrap_or(false);

    let (icon_data, icon_color) = match context.entry.node {
        QuickLaunchNode::Folder(folder) => {
            let icon = if folder.is_expanded() {
                FOLDER_OPENED
            } else {
                FOLDER
            };
            (icon, dim_foreground)
        },
        QuickLaunchNode::Command(_) => (
            PLAY,
            if is_indicator_highlighted {
                foreground
            } else {
                dim_foreground
            },
        ),
    };

    let icon_view = svg_icon(icon_data, icon_color);

    let title = container(
        text(context.entry.node.title())
            .size(TREE_FONT_SIZE)
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Left),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(alignment::Vertical::Center);

    let mut content = row![icon_view, title]
        .spacing(TREE_ROW_SPACING)
        .align_y(alignment::Vertical::Center);

    if is_indicator_highlighted {
        content = content.push(
            container(
                Space::new()
                    .width(Length::Fixed(6.0))
                    .height(Length::Fixed(6.0)),
            )
            .style(move |_| iced::widget::container::Style {
                background: Some(indicator_color.into()),
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
    }

    mouse_area(
        container(content)
            .width(Length::Fill)
            .height(Length::Fixed(TREE_ROW_HEIGHT))
            .padding([0.0, TREE_ROW_PADDING_X]),
    )
    .on_enter(QuickLaunchIntent::NodeHovered {
        path: context.entry.path.clone(),
    })
    .into()
}

fn quick_launch_row_style(
    drop_target: Option<&DropTarget>,
    palette: &crate::theme::IcedColorPalette,
    context: &TreeRowContext<'_, QuickLaunchNode>,
) -> iced::widget::container::Style {
    let is_drop_target = drop_target
        .and_then(|target| match target {
            DropTarget::Folder(path) => {
                Some(is_prefix(path, &context.entry.path))
            },
            DropTarget::Root => None,
        })
        .unwrap_or(false);

    let mut style =
        tree_row_style(palette, context.is_selected, context.is_hovered);

    if is_drop_target {
        let mut color = palette.overlay;
        color.a = 0.6;
        style.background = Some(color.into());
    }

    style
}

fn inline_edit_root<'a>(
    inline_edit: Option<&'a InlineEditState>,
    error_color: Color,
) -> Option<Element<'a, QuickLaunchIntent, Theme, iced::Renderer>> {
    let edit = inline_edit?;
    if matches!(
        edit.kind,
        InlineEditKind::CreateFolder { ref parent_path }
            if parent_path.is_empty()
    ) {
        return Some(render_inline_edit(edit, 0, error_color));
    }
    None
}

fn inline_edit_after<'a>(
    inline_edit: Option<&'a InlineEditState>,
    context: &TreeRowContext<'a, QuickLaunchNode>,
    error_color: Color,
) -> Option<Element<'a, QuickLaunchIntent, Theme, iced::Renderer>> {
    let edit = inline_edit?;

    match &edit.kind {
        InlineEditKind::CreateFolder { parent_path }
            if parent_path == &context.entry.path =>
        {
            Some(render_inline_edit(
                edit,
                context.entry.depth + 1,
                error_color,
            ))
        },
        InlineEditKind::Rename { path } if path == &context.entry.path => {
            Some(render_inline_edit(edit, context.entry.depth, error_color))
        },
        _ => None,
    }
}

fn is_rename_edit(
    inline_edit: Option<&InlineEditState>,
    context: &TreeRowContext<'_, QuickLaunchNode>,
) -> bool {
    matches!(inline_edit, Some(edit)
        if matches!(&edit.kind, InlineEditKind::Rename { path } if path == &context.entry.path))
}

fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    if prefix.len() > path.len() {
        return false;
    }

    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn render_inline_edit<'a>(
    edit: &'a InlineEditState,
    depth: usize,
    error_color: Color,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let indent = depth as f32 * TREE_INDENT;

    let input = text_input("", &edit.value)
        .on_input(QuickLaunchIntent::InlineEditChanged)
        .on_submit(QuickLaunchIntent::InlineEditSubmit)
        .size(TREE_FONT_SIZE)
        .id(edit.id.clone());

    let mut col = column![
        row![Space::new().width(Length::Fixed(indent)), input]
            .spacing(TREE_ROW_SPACING)
            .width(Length::Fill)
            .height(Length::Fixed(TREE_ROW_HEIGHT))
            .align_y(alignment::Vertical::Center),
    ]
    .width(Length::Fill)
    .height(Length::Shrink);

    if let Some(error) = &edit.error {
        col = col.push(
            container(text(error.as_str()).size(TREE_FONT_SIZE - 1.0))
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: TREE_ROW_PADDING_X + indent + 4.0,
                })
                .style(move |_| iced::widget::container::Style {
                    text_color: Some(error_color),
                    ..Default::default()
                }),
        );
    }

    container(col)
        .width(Length::Fill)
        .padding([0.0, TREE_ROW_PADDING_X])
        .into()
}

fn svg_icon<'a>(
    icon: &'static [u8],
    color: Color,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
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

impl TreeNode for QuickLaunchNode {
    fn title(&self) -> &str {
        QuickLaunchNode::title(self)
    }

    fn children(&self) -> Option<&[Self]> {
        match self {
            QuickLaunchNode::Folder(folder) => Some(folder.children()),
            QuickLaunchNode::Command(_) => None,
        }
    }

    fn expanded(&self) -> bool {
        match self {
            QuickLaunchNode::Folder(folder) => folder.is_expanded(),
            QuickLaunchNode::Command(_) => false,
        }
    }

    fn is_folder(&self) -> bool {
        matches!(self, QuickLaunchNode::Folder(_))
    }
}
