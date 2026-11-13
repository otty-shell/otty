use iced::keyboard::key::Named;
use iced::widget::{
    Space, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Color, Element, Length, alignment};
use otty_app_domain::quick_launch::NodePath;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::input_keys::InputKeys;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing, typography};
use otty_ui_tree::{TreeRowContext, TreeView};

use crate::event::QuickLaunchIntent;
use crate::model::{InlineEditViewModel, SidebarTreeViewModel};
use crate::types::{DropTarget, InlineEditKind, LaunchInfo, QuickLaunchNode};

/// Props for the quick launch tree view.
#[derive(Debug, Clone)]
pub(crate) struct SidebarTreeProps<'a> {
    pub(crate) vm: SidebarTreeViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch sidebar tree.
pub(crate) fn view(
    props: SidebarTreeProps<'_>,
) -> Element<'_, QuickLaunchIntent> {
    let ui = props.theme.theme.ui().clone();
    let error_color = ui.text_danger();
    let drop_background = ui.surface_selected();
    let scroll_style = scrollbar::style(&ui);

    let launching = props.vm.launching;
    let drop_target = props.vm.drop_target;
    let inline_edit = props.vm.inline_edit;

    let tree_view =
        TreeView::new(props.vm.data.root().children(), move |context| {
            render_entry(context, launching, drop_target, &ui)
        })
        .selected_row(props.vm.selected_path)
        .hovered_row(props.vm.hovered_path)
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
        .direction(scrollbar::vertical())
        .style(scroll_style);

    let tree_area = mouse_area(scrollable)
        .on_move(|position| QuickLaunchIntent::CursorMoved { position })
        .on_press(QuickLaunchIntent::BackgroundPressed)
        .on_release(QuickLaunchIntent::BackgroundReleased)
        .on_right_press(QuickLaunchIntent::BackgroundRightClicked);

    let root_drop_background = if matches!(drop_target, Some(DropTarget::Root))
    {
        Some(drop_background)
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
) -> Element<'a, QuickLaunchIntent> {
    let mut row = TreeRow::new(context.entry.node.title(), palette)
        .depth(context.entry.depth as u16)
        .selected(context.is_selected)
        .hovered(context.is_hovered)
        .drop_target(is_drop_target(drop_target, &context.entry.path));

    row = match context.entry.node {
        QuickLaunchNode::Folder(folder) => row.caret(folder.is_expanded()),
        QuickLaunchNode::Command(command) => {
            let kind = if command.is_ssh() {
                TargetKind::Ssh
            } else {
                TargetKind::Local
            };
            let row = row.dot(kind);

            if launching.contains_key(&context.entry.path) {
                row.state(i18n::t(Key::LaunchRunning))
            } else {
                row
            }
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
    inline_edit: Option<InlineEditViewModel<'a>>,
    error_color: Color,
) -> Option<Element<'a, QuickLaunchIntent>> {
    let edit = inline_edit?;
    if matches!(
        edit.kind,
        InlineEditKind::CreateFolder { parent_path }
            if parent_path.is_empty()
    ) {
        return Some(render_inline_edit(edit, 0, error_color));
    }
    None
}

fn inline_edit_after<'a>(
    inline_edit: Option<InlineEditViewModel<'a>>,
    context: &TreeRowContext<'a, QuickLaunchNode>,
    error_color: Color,
) -> Option<Element<'a, QuickLaunchIntent>> {
    let edit = inline_edit?;

    match edit.kind {
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
    inline_edit: Option<InlineEditViewModel<'_>>,
    context: &TreeRowContext<'_, QuickLaunchNode>,
) -> bool {
    matches!(inline_edit, Some(edit)
        if matches!(edit.kind, InlineEditKind::Rename { path } if path == &context.entry.path))
}

fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    if prefix.len() > path.len() {
        return false;
    }

    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn render_inline_edit<'a>(
    edit: InlineEditViewModel<'a>,
    depth: usize,
    error_color: Color,
) -> Element<'a, QuickLaunchIntent> {
    let indent = depth as f32 * size::TREE_INDENT;

    let input = text_input("", edit.value)
        .on_input(QuickLaunchIntent::InlineEditChanged)
        .on_submit(QuickLaunchIntent::InlineEditSubmit)
        .size(typography::BODY.size())
        .id(edit.id.clone());
    // Esc cancels the edit; a plain text input would only lose its focus.
    let input = InputKeys::new(input)
        .on_key(Named::Escape, QuickLaunchIntent::CancelInlineEdit);

    let mut col = column![
        row![Space::new().width(Length::Fixed(indent)), input]
            .spacing(spacing::SPACE_2)
            .width(Length::Fill)
            .height(Length::Fixed(size::ROW))
            .align_y(alignment::Vertical::Center),
    ]
    .width(Length::Fill)
    .height(Length::Shrink);

    if let Some(error) = edit.error {
        col = col.push(
            container(text(error).size(typography::CAPTION.size()))
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: spacing::SPACE_2 + indent + spacing::SPACE_1,
                })
                .style(move |_| iced::widget::container::Style {
                    text_color: Some(error_color),
                    ..Default::default()
                }),
        );
    }

    container(col).width(Length::Fill).padding(0.0).into()
}

#[cfg(test)]
mod tests {
    use iced::Color;
    use iced::keyboard::key::Named;
    use iced::widget::Id;
    use iced_test::simulator;

    use super::render_inline_edit;
    use crate::event::QuickLaunchIntent;
    use crate::model::InlineEditViewModel;
    use crate::state::InlineEditState;
    use crate::types::InlineEditKind;

    /// Scenario 5: Esc in the name field cancels the edit. Without it the
    /// field would only lose its focus, the edit would stay and Enter would
    /// do nothing.
    #[test]
    fn esc_in_the_inline_edit_cancels_it() {
        let edit = InlineEditState {
            kind: InlineEditKind::CreateFolder {
                parent_path: Vec::new(),
            },
            value: String::from("notes"),
            error: None,
            id: Id::new("inline_edit"),
        };
        let vm = InlineEditViewModel::from_state(&edit);
        let mut ui = simulator(render_inline_edit(vm, 0, Color::BLACK));

        ui.click(Id::new("inline_edit"))
            .expect("the name field should show");
        ui.tap_key(Named::Escape);

        let intents: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            intents.as_slice(),
            [QuickLaunchIntent::CancelInlineEdit]
        ));
    }
}
