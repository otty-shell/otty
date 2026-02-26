use iced::widget::{
    Space, column, container, row, scrollable, svg, text, text_input,
};
use iced::{Element, Length, Theme};

use crate::shared::ui::icons::{FOLDER, FOLDER_OPENED, PLAY};
use crate::shared::ui::theme::ThemeProps;
use crate::widgets::quick_launch::event::QuickLaunchEvent;
use crate::widgets::quick_launch::model::{
    LaunchInfo, NodePath, QuickLaunchFile, QuickLaunchNode,
};
use crate::widgets::quick_launch::state::{
    DropTarget, InlineEditKind, InlineEditState,
};

const TREE_ROW_HEIGHT: f32 = 24.0;
const TREE_FONT_SIZE: f32 = 12.0;
const TREE_INDENT: f32 = 14.0;
const ICON_WIDTH: f32 = 14.0;

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
) -> Element<'_, QuickLaunchEvent, Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette();

    let mut entries: Vec<Element<'_, QuickLaunchEvent, Theme, iced::Renderer>> =
        Vec::new();

    render_children(
        props.data.root().children(),
        &[],
        0,
        &props,
        palette,
        &mut entries,
    );

    // Inline create folder at root level
    if let Some(edit) = props.inline_edit {
        if let InlineEditKind::CreateFolder { parent_path } = &edit.kind {
            if parent_path.is_empty() {
                entries.push(render_inline_edit(edit, 0, palette));
            }
        }
    }

    if entries.is_empty() {
        entries.push(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );
    }

    let content = column(entries).width(Length::Fill).spacing(0);

    let tree_area = iced::widget::mouse_area(
        scrollable(content).width(Length::Fill).height(Length::Fill),
    )
    .on_press(QuickLaunchEvent::BackgroundPressed)
    .on_release(QuickLaunchEvent::BackgroundReleased)
    .on_right_press(QuickLaunchEvent::BackgroundRightClicked);

    container(tree_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn render_children<'a>(
    children: &'a [QuickLaunchNode],
    parent_path: &[String],
    depth: usize,
    props: &SidebarTreeProps<'a>,
    palette: &'a crate::shared::ui::theme::IcedColorPalette,
    entries: &mut Vec<Element<'a, QuickLaunchEvent, Theme, iced::Renderer>>,
) {
    for node in children {
        let mut path = parent_path.to_vec();
        path.push(node.title().to_string());

        let is_selected =
            props.selected_path.map(|s| s == &path).unwrap_or(false);
        let is_hovered =
            props.hovered_path.map(|h| h == &path).unwrap_or(false);

        // Check if this node has an active inline rename
        let is_renaming = props.inline_edit.is_some_and(|edit| {
            matches!(&edit.kind, InlineEditKind::Rename { path: edit_path } if edit_path == &path)
        });

        if is_renaming {
            if let Some(edit) = props.inline_edit {
                entries.push(render_inline_edit(edit, depth, palette));
            }
        } else {
            entries.push(render_tree_row(
                node,
                &path,
                depth,
                is_selected,
                is_hovered,
                props.launching.get(&path),
                palette,
            ));
        }

        // Recurse into expanded folders
        if let QuickLaunchNode::Folder(folder) = node {
            if folder.is_expanded() {
                render_children(
                    folder.children(),
                    &path,
                    depth + 1,
                    props,
                    palette,
                    entries,
                );

                // Inline create folder inside this folder
                if let Some(edit) = props.inline_edit {
                    if let InlineEditKind::CreateFolder { parent_path } =
                        &edit.kind
                    {
                        if parent_path == &path {
                            entries.push(render_inline_edit(
                                edit,
                                depth + 1,
                                palette,
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn render_tree_row<'a>(
    node: &'a QuickLaunchNode,
    path: &NodePath,
    depth: usize,
    is_selected: bool,
    is_hovered: bool,
    launch_info: Option<&'a LaunchInfo>,
    palette: &'a crate::shared::ui::theme::IcedColorPalette,
) -> Element<'a, QuickLaunchEvent, Theme, iced::Renderer> {
    let indent = depth as f32 * TREE_INDENT;
    let foreground = palette.foreground;
    let dim_foreground = palette.dim_foreground;

    let icon_data = match node {
        QuickLaunchNode::Folder(f) => {
            if f.is_expanded() {
                FOLDER_OPENED
            } else {
                FOLDER
            }
        },
        QuickLaunchNode::Command(_) => PLAY,
    };

    let icon = svg::Svg::new(svg::Handle::from_memory(icon_data))
        .width(Length::Fixed(ICON_WIDTH))
        .height(Length::Fixed(ICON_WIDTH));

    let label = text(node.title()).size(TREE_FONT_SIZE);

    let mut row_content = row![
        Space::new().width(Length::Fixed(indent)),
        container(icon)
            .width(Length::Fixed(ICON_WIDTH + 4.0))
            .height(Length::Fixed(TREE_ROW_HEIGHT))
            .align_y(iced::alignment::Vertical::Center),
        label,
    ]
    .align_y(iced::alignment::Vertical::Center)
    .width(Length::Fill)
    .height(Length::Fixed(TREE_ROW_HEIGHT));

    // Add launch indicator
    if let Some(info) = launch_info {
        if info.is_indicator_highlighted {
            row_content = row_content.push(
                container(
                    Space::new()
                        .width(Length::Fixed(6.0))
                        .height(Length::Fixed(6.0)),
                )
                .style(move |_| {
                    iced::widget::container::Style {
                        background: Some(palette.green.into()),
                        border: iced::Border {
                            radius: 3.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
            );
        }
    }

    let bg_color = if is_selected {
        let mut c = palette.overlay;
        c.a = 0.3;
        Some(c)
    } else if is_hovered {
        let mut c = palette.overlay;
        c.a = 0.15;
        Some(c)
    } else {
        None
    };

    let path_for_hover = path.clone();
    let path_for_press = path.clone();
    let path_for_release = path.clone();
    let path_for_right = path.clone();

    let styled_row = container(row_content)
        .width(Length::Fill)
        .height(Length::Fixed(TREE_ROW_HEIGHT))
        .style(move |_| iced::widget::container::Style {
            background: bg_color.map(Into::into),
            text_color: Some(if is_selected {
                foreground
            } else {
                dim_foreground
            }),
            ..Default::default()
        });

    let interactive = iced::widget::mouse_area(styled_row)
        .on_enter(QuickLaunchEvent::NodeHovered {
            path: path_for_hover,
        })
        .on_press(QuickLaunchEvent::NodePressed {
            path: path_for_press,
        })
        .on_release(QuickLaunchEvent::NodeReleased {
            path: path_for_release,
        })
        .on_right_press(QuickLaunchEvent::NodeRightClicked {
            path: path_for_right,
        });

    interactive.into()
}

fn render_inline_edit<'a>(
    edit: &'a InlineEditState,
    depth: usize,
    palette: &'a crate::shared::ui::theme::IcedColorPalette,
) -> Element<'a, QuickLaunchEvent, Theme, iced::Renderer> {
    let indent = depth as f32 * TREE_INDENT;

    let input = text_input("", &edit.value)
        .on_input(QuickLaunchEvent::InlineEditChanged)
        .on_submit(QuickLaunchEvent::InlineEditSubmit)
        .size(TREE_FONT_SIZE)
        .id(edit.id.clone());

    let mut col = column![
        row![Space::new().width(Length::Fixed(indent)), input,]
            .width(Length::Fill)
            .height(Length::Fixed(TREE_ROW_HEIGHT))
            .align_y(iced::alignment::Vertical::Center),
    ];

    if let Some(error) = &edit.error {
        let error_color = palette.red;
        col = col.push(
            container(text(error.as_str()).size(TREE_FONT_SIZE - 1.0))
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: indent + 4.0,
                })
                .style(move |_| iced::widget::container::Style {
                    text_color: Some(error_color),
                    ..Default::default()
                }),
        );
    }

    col.into()
}
