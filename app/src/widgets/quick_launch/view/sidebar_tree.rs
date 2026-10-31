use iced::widget::{
    Space, column, container, mouse_area, row, scrollable, svg, text,
    text_input,
};
use iced::{Color, Element, Length, Theme, alignment};
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, typography};
use otty_ui_tree::{TreeRowContext, TreeView};

use super::super::event::QuickLaunchIntent;
use super::super::state::InlineEditState;
use super::super::types::{
    DropTarget, InlineEditKind, LaunchInfo, NodePath, QuickLaunchFile,
    QuickLaunchNode,
};
use crate::icons::PLAY;
use crate::style::thin_scroll_style;
use crate::theme::ThemeProps;

/// Gap between the indent and the inline edit field.
const INLINE_EDIT_SPACING: f32 = 6.0;

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
    let ui = props.theme.theme.ui().clone();
    let error_color = ui.text_danger();
    let accent = ui.accent();

    let launching = props.launching;
    let drop_target = props.drop_target;
    let inline_edit = props.inline_edit;

    let tree_view =
        TreeView::new(props.data.root().children(), move |context| {
            render_entry(context, launching, drop_target, &ui)
        })
        .selected_row(props.selected_path)
        .hovered_row(props.hovered_path)
        .on_press(|path| QuickLaunchIntent::NodePressed { path })
        .on_release(|path| QuickLaunchIntent::NodeReleased { path })
        .on_right_press(|path| QuickLaunchIntent::NodeRightClicked { path })
        .row_visible_filter(move |context| {
            !is_rename_edit(inline_edit, context)
        })
        .after_row(move |context| {
            inline_edit_after(inline_edit, context, error_color)
        })
        .spacing(0.0);

    let tree_content =
        if let Some(root_edit) = inline_edit_root(inline_edit, error_color) {
            column![root_edit, tree_view.view()].into()
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
        Some(Color { a: 0.2, ..accent })
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
    drop_target: Option<&DropTarget>,
    palette: &UiPalette,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let is_indicator_highlighted = launching
        .get(&context.entry.path)
        .map(|launch| launch.is_indicator_highlighted)
        .unwrap_or(false);

    let mut row = TreeRow::new(context.entry.node.title(), palette)
        .depth(context.entry.depth as u16)
        .selected(context.is_selected)
        .hovered(context.is_hovered)
        .drop_target(is_drop_target(drop_target, &context.entry.path));

    row = match context.entry.node {
        QuickLaunchNode::Folder(folder) => row.caret(folder.is_expanded()),
        QuickLaunchNode::Command(_) => {
            let color = is_indicator_highlighted.then(|| palette.accent());

            row.icon(svg::Handle::from_memory(PLAY), color)
        },
    };

    mouse_area(row)
        .on_enter(QuickLaunchIntent::NodeHovered {
            path: context.entry.path.clone(),
        })
        .into()
}

fn is_drop_target(drop_target: Option<&DropTarget>, path: &[String]) -> bool {
    match drop_target {
        Some(DropTarget::Folder(target)) => is_prefix(target, path),
        _ => false,
    }
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
    let indent = depth as f32 * size::TREE_INDENT;

    let input = text_input("", &edit.value)
        .on_input(QuickLaunchIntent::InlineEditChanged)
        .on_submit(QuickLaunchIntent::InlineEditSubmit)
        .size(typography::BODY.size())
        .id(edit.id.clone());

    let mut col = column![
        row![Space::new().width(Length::Fixed(indent)), input]
            .spacing(INLINE_EDIT_SPACING)
            .width(Length::Fill)
            .height(Length::Fixed(size::ROW))
            .align_y(alignment::Vertical::Center),
    ]
    .width(Length::Fill)
    .height(Length::Shrink);

    if let Some(error) = &edit.error {
        col = col.push(
            container(text(error.as_str()).size(typography::CAPTION.size()))
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: INLINE_EDIT_SPACING + indent + 4.0,
                })
                .style(move |_| iced::widget::container::Style {
                    text_color: Some(error_color),
                    ..Default::default()
                }),
        );
    }

    container(col).width(Length::Fill).padding(0.0).into()
}
