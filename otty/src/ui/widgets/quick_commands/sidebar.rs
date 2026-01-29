use iced::alignment;
use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Space, column, container, mouse_area, row, scrollable, text,
    text_input,
};
use iced::{Element, Length, Size};

use crate::features::quick_commands::event::QuickCommandsEvent;
use crate::features::quick_commands::model::QuickCommandNode;
use crate::features::quick_commands::state::{
    DropTarget, InlineEditKind, InlineEditState, QuickCommandsState,
};
use crate::icons;
use crate::theme::ThemeProps;
use crate::ui::components::icon_button::{
    IconButton, IconButtonEvent, IconButtonProps, IconButtonVariant,
};
use crate::ui::widgets::quick_commands::context_menu;
use crate::ui::widgets::tree::{TreeNode, flatten_tree};

const HEADER_HEIGHT: f32 = 28.0;
const HEADER_PADDING_X: f32 = 10.0;
const HEADER_FONT_SIZE: f32 = 12.0;
const HEADER_ICON_SIZE: f32 = 16.0;
const HEADER_BUTTON_SIZE: f32 = 20.0;

const TREE_ROW_HEIGHT: f32 = 24.0;
const TREE_FONT_SIZE: f32 = 12.0;
const TREE_INDENT: f32 = 14.0;
const TREE_ICON_WIDTH: f32 = 10.0;
const TREE_ROW_PADDING_X: f32 = 6.0;
const TREE_ROW_SPACING: f32 = 6.0;

const INPUT_PADDING_X: f32 = 6.0;
const INPUT_PADDING_Y: f32 = 4.0;
const INPUT_FONT_SIZE: f32 = 12.0;

/// Props for rendering quick commands in the terminal sidebar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Props<'a> {
    pub(crate) state: &'a QuickCommandsState,
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) workspace_size: Size,
}

pub(crate) fn view<'a>(props: Props<'a>) -> Element<'a, QuickCommandsEvent> {
    let header = quick_commands_header(props.theme);

    let tree_list = quick_commands_tree(props);

    let mut layers: Vec<Element<'a, QuickCommandsEvent>> = Vec::new();
    layers.push(tree_list);

    if let Some(menu) = props.state.context_menu.as_ref() {
        layers.push(context_menu::view(context_menu::Props {
            menu,
            theme: props.theme,
            area_size: props.workspace_size,
        }));
    }

    let tree_stack = iced::widget::Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill);

    column![header, tree_stack]
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

    let folder_button = IconButton::new(IconButtonProps {
        icon: icons::FOLDER,
        theme,
        size: HEADER_BUTTON_SIZE,
        icon_size: HEADER_ICON_SIZE,
        variant: IconButtonVariant::Standard,
    })
    .view()
    .map(|event| match event {
        IconButtonEvent::Pressed => QuickCommandsEvent::HeaderCreateFolder,
    });

    let add_button = IconButton::new(IconButtonProps {
        icon: icons::PLUS,
        theme,
        size: HEADER_BUTTON_SIZE,
        icon_size: HEADER_ICON_SIZE,
        variant: IconButtonVariant::Standard,
    })
    .view()
    .map(|event| match event {
        IconButtonEvent::Pressed => QuickCommandsEvent::HeaderCreateCommand,
    });

    let row = row![title, folder_button, add_button]
        .spacing(4)
        .align_y(alignment::Vertical::Center);

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

    let scrollable = scrollable::Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .margin(2)
                .scroller_width(4),
        ));

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
                let mut color = palette.dim_green;
                color.a = 0.2;
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

    let is_editing = matches!(props.state.inline_edit.as_ref(), Some(edit)
        if matches!(&edit.kind, InlineEditKind::Rename { path } if path == &entry.path));

    if is_editing {
        if let Some(edit) = props.state.inline_edit.as_ref() {
            return inline_edit_row(edit, entry.depth);
        }
    }

    let caret = if entry.node.is_folder() {
        if entry.node.expanded() { "v" } else { ">" }
    } else {
        ""
    };

    let caret_text = text(caret)
        .size(TREE_FONT_SIZE)
        .width(Length::Fixed(TREE_ICON_WIDTH))
        .align_x(alignment::Horizontal::Center);

    let title = text(entry.node.title())
        .size(TREE_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    let content =
        row![Space::new().width(Length::Fixed(indent)), caret_text, title]
            .spacing(TREE_ROW_SPACING)
            .align_y(alignment::Vertical::Center);

    let palette = props.theme.theme.iced_palette().clone();

    let row = container(content)
        .width(Length::Fill)
        .height(Length::Fixed(TREE_ROW_HEIGHT))
        .padding([0.0, TREE_ROW_PADDING_X])
        .style(move |_| {
            let background = if is_drop_target {
                let mut color = palette.dim_green;
                color.a = 0.35;
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
        .on_press(QuickCommandsEvent::NodePressed { path: path.clone() })
        .on_release(QuickCommandsEvent::NodeReleased { path: path.clone() })
        .on_right_press(QuickCommandsEvent::NodeRightClicked { path })
        .on_enter(QuickCommandsEvent::HoverChanged {
            path: Some(entry.path.clone()),
        })
        .on_exit(QuickCommandsEvent::HoverChanged { path: None })
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
        .width(Length::Fill);

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
