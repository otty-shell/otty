use iced::alignment;
use iced::border::Radius;
use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Space, column, container, mouse_area, row, scrollable, svg, text,
    text_input,
};
use iced::{Background, Element, Length, mouse};
use std::time::{Duration, Instant};

use crate::features::quick_commands::event::{
    QUICK_COMMANDS_TICK_MS, QuickCommandsEvent,
};
use crate::features::quick_commands::model::QuickCommandNode;
use crate::features::quick_commands::state::{
    DropTarget, InlineEditKind, InlineEditState, QuickCommandsState,
};
use crate::icons;
use crate::theme::ThemeProps;
use crate::ui::widgets::tree::{TreeNode, flatten_tree};

const HEADER_HEIGHT: f32 = 28.0;
const HEADER_PADDING_X: f32 = 10.0;
const HEADER_FONT_SIZE: f32 = 12.0;

const TREE_ROW_HEIGHT: f32 = 24.0;
const TREE_FONT_SIZE: f32 = 12.0;
const TREE_INDENT: f32 = 14.0;
const TREE_ICON_WIDTH: f32 = 14.0;
const TREE_ROW_PADDING_X: f32 = 6.0;
const TREE_ROW_SPACING: f32 = 6.0;
const LAUNCH_INDICATOR_SIZE: f32 = 6.0;
const LAUNCH_INDICATOR_SLOT: f32 = 12.0;
const LAUNCH_INDICATOR_DELAY: Duration = Duration::from_secs(1);
const LAUNCH_INDICATOR_BLINK_MS: u128 = 500;

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
    let entries = flatten_tree(&props.state.data.root.children);

    let mut column = Column::new().spacing(0);

    if let Some(edit) = props.state.inline_edit.as_ref()
        && matches!(
            &edit.kind,
            InlineEditKind::CreateFolder { parent_path }
                if parent_path.is_empty()
        )
    {
        column = column.push(inline_edit_row(edit, 0));
    }

    for entry in entries {
        let row = render_entry(props, &entry);
        column = column.push(row);

        if let Some(edit) = props.state.inline_edit.as_ref()
            && matches!(
                &edit.kind,
                InlineEditKind::CreateFolder { parent_path }
                    if parent_path == &entry.path
            )
        {
            column = column.push(inline_edit_row(edit, entry.depth + 1));
        }
    }

    let palette = props.theme.theme.iced_palette().clone();

    let scrollable = scrollable::Scrollable::new(column)
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
    entry: &crate::ui::widgets::tree::FlattenedNode<'a, QuickCommandNode>,
) -> Element<'a, QuickCommandsEvent> {
    let indent = entry.depth as f32 * TREE_INDENT;
    let is_hovered = props
        .state
        .hovered
        .as_ref()
        .map(|path| path == &entry.path)
        .unwrap_or(false);
    let is_selected = props
        .state
        .selected
        .as_ref()
        .map(|path| path == &entry.path)
        .unwrap_or(false);
    let is_drop_target = props
        .state
        .drop_target
        .as_ref()
        .and_then(|target| match target {
            DropTarget::Folder(path) => Some(is_prefix(path, &entry.path)),
            DropTarget::Root => None,
        })
        .unwrap_or(false);
    let launched_at = props
        .state
        .launching
        .get(&entry.path)
        .map(|info| info.started_at);

    let is_editing = matches!(props.state.inline_edit.as_ref(), Some(edit)
        if matches!(&edit.kind, InlineEditKind::Rename { path } if path == &entry.path));

    if is_editing {
        if let Some(edit) = props.state.inline_edit.as_ref() {
            return inline_edit_row(edit, entry.depth);
        }
    }

    let icon_view: Element<'a, QuickCommandsEvent> = if entry.node.is_folder() {
        let icon = if entry.node.expanded() {
            icons::FOLDER_OPENED
        } else {
            icons::FOLDER
        };
        let handle = svg::Handle::from_memory(icon);
        let svg_icon = svg::Svg::new(handle)
            .width(Length::Fixed(TREE_ICON_WIDTH))
            .height(Length::Fixed(TREE_ICON_WIDTH))
            .style({
                let palette = props.theme.theme.iced_palette().clone();
                move |_, _| svg::Style {
                    color: Some(palette.dim_foreground),
                }
            });
        container(svg_icon)
            .width(Length::Fixed(TREE_ICON_WIDTH))
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .into()
    } else {
        Space::new()
            .width(Length::Fixed(TREE_ICON_WIDTH))
            .height(Length::Fill)
            .into()
    };

    let title = container(
        text(entry.node.title())
            .size(TREE_FONT_SIZE)
            .width(Length::Fill)
            .wrapping(Wrapping::None)
            .align_x(alignment::Horizontal::Left),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(alignment::Vertical::Center);

    let indicator = launch_indicator(
        props.theme,
        !entry.node.is_folder(),
        launched_at,
        props.state.blink_nonce,
    );

    let content = row![
        Space::new().width(Length::Fixed(indent)),
        icon_view,
        indicator,
        title,
    ]
    .spacing(TREE_ROW_SPACING)
    .align_y(alignment::Vertical::Center);

    let palette = props.theme.theme.iced_palette().clone();
    let row = container(content)
        .width(Length::Fill)
        .height(Length::Fixed(TREE_ROW_HEIGHT))
        .padding([0.0, TREE_ROW_PADDING_X])
        .style(move |_| {
            let background = if is_drop_target {
                let mut color = palette.overlay;
                color.a = 0.6;
                Some(color.into())
            } else if is_selected {
                let mut color = palette.dim_blue;
                color.a = 0.7;
                Some(color.into())
            } else if is_hovered {
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
        });

    let path = entry.path.clone();
    mouse_area(row)
        .interaction(mouse::Interaction::Pointer)
        .on_press(QuickCommandsEvent::NodePressed { path: path.clone() })
        .on_release(QuickCommandsEvent::NodeReleased { path: path.clone() })
        .on_right_press(QuickCommandsEvent::NodeRightClicked { path })
        .on_enter(QuickCommandsEvent::HoverEntered {
            path: entry.path.clone(),
        })
        .on_exit(QuickCommandsEvent::HoverLeft {
            path: entry.path.clone(),
        })
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

fn launch_indicator<'a>(
    theme: ThemeProps<'a>,
    is_command: bool,
    launched_at: Option<Instant>,
    blink_nonce: u64,
) -> Element<'a, QuickCommandsEvent> {
    let slot = Length::Fixed(LAUNCH_INDICATOR_SLOT);
    if !is_command {
        return Space::new().width(slot).height(Length::Fill).into();
    }

    let show = launched_at
        .map(|start| start.elapsed())
        .filter(|elapsed| *elapsed >= LAUNCH_INDICATOR_DELAY);

    if let Some(_elapsed) = show {
        let blink_step = (blink_nonce as u128 * QUICK_COMMANDS_TICK_MS as u128)
            / LAUNCH_INDICATOR_BLINK_MS;
        let blink_on = blink_step.is_multiple_of(2);
        if blink_on {
            let palette = theme.theme.iced_palette();
            let dot = container(Space::new())
                .width(Length::Fixed(LAUNCH_INDICATOR_SIZE))
                .height(Length::Fixed(LAUNCH_INDICATOR_SIZE))
                .style(move |_| iced::widget::container::Style {
                    background: Some(palette.blue.into()),
                    border: iced::Border {
                        radius: Radius::from(LAUNCH_INDICATOR_SIZE / 2.0),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            container(dot)
                .width(slot)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .into()
        } else {
            Space::new().width(slot).height(Length::Fill).into()
        }
    } else {
        Space::new().width(slot).height(Length::Fill).into()
    }
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
