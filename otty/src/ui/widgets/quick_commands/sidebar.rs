use iced::alignment;
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, column, container, mouse_area, row, scrollable, svg, text,
    text_input,
};
use iced::{Background, Color, Element, Length};
use std::time::{Duration, Instant};

use crate::features::quick_commands::event::{
    QUICK_COMMANDS_TICK_MS, QuickCommandsEvent,
};
use crate::features::quick_commands::model::QuickCommandNode;
use crate::features::quick_commands::state::{
    DropTarget, InlineEditKind, InlineEditState, QuickCommandsState,
};
use crate::icons;
use crate::theme::{IcedColorPalette, ThemeProps};
use otty_ui_tree::{TreeNode, TreeRowContext, TreeView};

const HEADER_HEIGHT: f32 = 28.0;
const HEADER_PADDING_X: f32 = 10.0;
const HEADER_FONT_SIZE: f32 = 12.0;

const TREE_ROW_HEIGHT: f32 = 24.0;
const TREE_FONT_SIZE: f32 = 12.0;
const TREE_INDENT: f32 = 14.0;
const TREE_ICON_WIDTH: f32 = 14.0;
const TREE_ROW_PADDING_X: f32 = 6.0;
const TREE_ROW_SPACING: f32 = 6.0;
const LAUNCH_ICON_DELAY: Duration = Duration::from_secs(1);
const LAUNCH_ICON_BLINK_MS: u128 = 500;

const INPUT_PADDING_X: f32 = 6.0;
const INPUT_PADDING_Y: f32 = 4.0;
const INPUT_FONT_SIZE: f32 = 12.0;

/// Props for rendering quick commands in the terminal sidebar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Props<'a> {
    pub(crate) state: &'a QuickCommandsState,
    pub(crate) theme: ThemeProps<'a>,
}

pub(crate) fn view<'a>(props: Props<'a>) -> Element<'a, QuickCommandsEvent> {
    let header = quick_commands_header(props.theme);

    let tree_list = quick_commands_tree(props);

    column![header, tree_list]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn quick_commands_header<'a>(
    theme: ThemeProps<'a>,
) -> Element<'a, QuickCommandsEvent> {
    let title = text("Quick commands")
        .size(HEADER_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    let row = row![title].spacing(4).align_y(alignment::Vertical::Center);

    let palette = theme.theme.iced_palette().clone();

    container(row)
        .width(Length::Fill)
        .height(Length::Fixed(HEADER_HEIGHT))
        .padding([0.0, HEADER_PADDING_X])
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .style(move |_| iced::widget::container::Style {
            background: Some(palette.overlay.into()),
            text_color: Some(palette.foreground),
            ..Default::default()
        })
        .into()
}

fn quick_commands_tree<'a>(
    props: Props<'a>,
) -> Element<'a, QuickCommandsEvent> {
    let row_props = props;
    let row_style_props = props;
    let after_props = props;
    let visible_props = props;

    let palette = props.theme.theme.iced_palette().clone();
    let row_palette = palette.clone();

    let tree_view =
        TreeView::new(&props.state.data.root.children, move |context| {
            render_entry(row_props, context)
        })
        .selected(props.state.selected.as_ref())
        .hovered(props.state.hovered.as_ref())
        .on_press(|path| QuickCommandsEvent::NodePressed { path })
        .on_release(|path| QuickCommandsEvent::NodeReleased { path })
        .on_right_press(|path| QuickCommandsEvent::NodeRightClicked { path })
        .on_hover(|path| QuickCommandsEvent::NodeHovered { path })
        .row_style(move |context| {
            tree_row_style(row_style_props, &row_palette, context)
        })
        .row_visible(move |context| !is_rename_edit(visible_props, context))
        .after_row(move |context| inline_edit_after(after_props, context))
        .indent_width(TREE_INDENT)
        .spacing(0.0);

    let palette = props.theme.theme.iced_palette().clone();

    let tree_content = if let Some(root_edit) = inline_edit_root(props) {
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
        .style(move |theme, status| {
            let mut style = scrollable::default(theme, status);
            let radius = iced::border::Radius::from(0.0);

            style.vertical_rail.border.radius = radius;
            style.vertical_rail.scroller.border.radius = radius;
            style.horizontal_rail.border.radius = radius;
            style.horizontal_rail.scroller.border.radius = radius;

            let mut scroller_color =
                match style.vertical_rail.scroller.background {
                    Background::Color(color) => color,
                    _ => palette.dim_foreground,
                };
            scroller_color.a = (scroller_color.a * 0.7).min(1.0);
            style.vertical_rail.scroller.background =
                Background::Color(scroller_color);
            style.horizontal_rail.scroller.background =
                Background::Color(scroller_color);

            style
        });

    let scrollable = mouse_area(scrollable)
        .on_move(|position| QuickCommandsEvent::CursorMoved { position })
        .on_press(QuickCommandsEvent::BackgroundPressed)
        .on_release(QuickCommandsEvent::BackgroundReleased)
        .on_right_press(QuickCommandsEvent::BackgroundRightClicked);

    let is_root_drop =
        matches!(props.state.drop_target, Some(DropTarget::Root))
            && props
                .state
                .drag
                .as_ref()
                .map(|drag| drag.active)
                .unwrap_or(false);
    let palette = props.theme.theme.iced_palette().clone();

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| {
            let background = if is_root_drop {
                let mut color = palette.overlay;
                color.a = 0.6;
                Some(color.into())
            } else {
                None
            };
            iced::widget::container::Style {
                background,
                ..Default::default()
            }
        })
        .into()
}

fn render_entry<'a>(
    props: Props<'a>,
    context: &TreeRowContext<'a, QuickCommandNode>,
) -> Element<'a, QuickCommandsEvent> {
    let launched_at = props
        .state
        .launching
        .get(&context.entry.path)
        .map(|info| info.started_at);

    let icon_palette = props.theme.theme.iced_palette();
    let icon_view: Element<'a, QuickCommandsEvent> =
        if context.entry.node.is_folder() {
            let icon = if context.entry.node.expanded() {
                icons::FOLDER_OPENED
            } else {
                icons::FOLDER
            };
            svg_icon(icon, icon_palette.dim_foreground)
        } else {
            command_icon(icon_palette, launched_at, props.state.blink_nonce)
        };

    let title = container(
        text(context.entry.node.title())
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

fn inline_edit_row<'a>(
    edit: &'a InlineEditState,
    depth: usize,
) -> Element<'a, QuickCommandsEvent> {
    let indent = depth as f32 * TREE_INDENT;
    let input = text_input("", &edit.value)
        .on_input(QuickCommandsEvent::InlineEditChanged)
        .on_submit(QuickCommandsEvent::InlineEditSubmit)
        .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
        .size(INPUT_FONT_SIZE)
        .width(Length::Fill)
        .id(edit.id.clone());

    let row = row![Space::new().width(Length::Fixed(indent)), input]
        .spacing(TREE_ROW_SPACING)
        .align_y(alignment::Vertical::Center);

    let mut column = column![row].width(Length::Fill).height(Length::Shrink);

    if let Some(error) = &edit.error {
        let error_color = iced::Color::from_rgb(0.9, 0.4, 0.4);
        let error_text =
            text(error)
                .size(10.0)
                .style(move |_| iced::widget::text::Style {
                    color: Some(error_color),
                });
        column = column.push(error_text);
    }

    container(column)
        .width(Length::Fill)
        .padding([0.0, TREE_ROW_PADDING_X])
        .into()
}

fn inline_edit_root(
    props: Props<'_>,
) -> Option<Element<'_, QuickCommandsEvent>> {
    let edit = props.state.inline_edit.as_ref()?;
    if matches!(
        &edit.kind,
        InlineEditKind::CreateFolder { parent_path }
            if parent_path.is_empty()
    ) {
        return Some(inline_edit_row(edit, 0));
    }
    None
}

fn inline_edit_after<'a>(
    props: Props<'a>,
    context: &TreeRowContext<'a, QuickCommandNode>,
) -> Option<Element<'a, QuickCommandsEvent>> {
    let edit = props.state.inline_edit.as_ref()?;

    match &edit.kind {
        InlineEditKind::CreateFolder { parent_path }
            if parent_path == &context.entry.path =>
        {
            Some(inline_edit_row(edit, context.entry.depth + 1))
        },
        InlineEditKind::Rename { path } if path == &context.entry.path => {
            Some(inline_edit_row(edit, context.entry.depth))
        },
        _ => None,
    }
}

fn is_rename_edit(
    props: Props<'_>,
    context: &TreeRowContext<'_, QuickCommandNode>,
) -> bool {
    matches!(props.state.inline_edit.as_ref(), Some(edit)
        if matches!(&edit.kind, InlineEditKind::Rename { path } if path == &context.entry.path))
}

fn tree_row_style(
    props: Props<'_>,
    palette: &IcedColorPalette,
    context: &TreeRowContext<'_, QuickCommandNode>,
) -> iced::widget::container::Style {
    let is_drop_target = props
        .state
        .drop_target
        .as_ref()
        .and_then(|target| match target {
            DropTarget::Folder(path) => {
                Some(is_prefix(path, &context.entry.path))
            },
            DropTarget::Root => None,
        })
        .unwrap_or(false);

    let background = if is_drop_target {
        let mut color = palette.overlay;
        color.a = 0.6;
        Some(color.into())
    } else if context.is_selected {
        let mut color = palette.dim_blue;
        color.a = 0.7;
        Some(color.into())
    } else if context.is_hovered {
        let mut color = palette.overlay;
        color.a = 0.6;
        Some(color.into())
    } else {
        None
    };

    iced::widget::container::Style {
        background,
        text_color: Some(palette.foreground),
        ..Default::default()
    }
}

fn command_icon<'a>(
    palette: &IcedColorPalette,
    launched_at: Option<Instant>,
    blink_nonce: u64,
) -> Element<'a, QuickCommandsEvent> {
    let show = launched_at
        .map(|start| start.elapsed())
        .filter(|elapsed| *elapsed >= LAUNCH_ICON_DELAY)
        .is_some();

    let color = if show {
        let blink_step = (blink_nonce as u128 * QUICK_COMMANDS_TICK_MS as u128)
            / LAUNCH_ICON_BLINK_MS;
        let blink_on = blink_step.is_multiple_of(2);
        if blink_on {
            palette.foreground
        } else {
            palette.dim_foreground
        }
    } else {
        palette.dim_foreground
    };

    svg_icon(icons::PLAY, color)
}

fn svg_icon<'a>(
    icon: &'static [u8],
    color: Color,
) -> Element<'a, QuickCommandsEvent> {
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

fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    if prefix.len() > path.len() {
        return false;
    }

    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

impl TreeNode for QuickCommandNode {
    fn title(&self) -> &str {
        QuickCommandNode::title(self)
    }

    fn children(&self) -> Option<&[Self]> {
        match self {
            QuickCommandNode::Folder(folder) => Some(&folder.children),
            QuickCommandNode::Command(_) => None,
        }
    }

    fn expanded(&self) -> bool {
        match self {
            QuickCommandNode::Folder(folder) => folder.expanded,
            QuickCommandNode::Command(_) => false,
        }
    }

    fn is_folder(&self) -> bool {
        QuickCommandNode::is_folder(self)
    }
}
