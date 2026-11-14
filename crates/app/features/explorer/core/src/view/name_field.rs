use iced::keyboard::key::Named;
use iced::widget::{Space, column, container, row, text, text_input};
use iced::{Color, Element, Length, Padding, alignment};
use otty_ui_kit::components::input_keys::InputKeys;
use otty_ui_kit::{size, spacing, typography};
use otty_ui_tree::TreeRowContext;

use crate::event::ExplorerIntent;
use crate::interaction::{InlineEdit, InlineEditKind};
use crate::types::FileNode;

/// The name field above every row, for a new entry in the followed folder.
pub(crate) fn at_top(
    edit: Option<&InlineEdit>,
    error_color: Color,
) -> Option<Element<'_, ExplorerIntent>> {
    let edit = edit?;

    match &edit.kind {
        InlineEditKind::NewFile { parent }
        | InlineEditKind::NewFolder { parent }
            if parent.is_empty() =>
        {
            Some(view(edit, 0, error_color))
        },
        _ => None,
    }
}

/// The name field after the row of `context`: for a new entry in that
/// folder, one level deeper, or for its new name, in its place.
pub(crate) fn after_row<'a>(
    edit: Option<&'a InlineEdit>,
    context: &TreeRowContext<'_, FileNode>,
    error_color: Color,
) -> Option<Element<'a, ExplorerIntent>> {
    let edit = edit?;
    let row_path = &context.entry.path;
    let depth = context.entry.depth;

    match &edit.kind {
        InlineEditKind::NewFile { parent }
        | InlineEditKind::NewFolder { parent }
            if parent == row_path =>
        {
            Some(view(edit, depth + 1, error_color))
        },
        InlineEditKind::Rename { path } if path == row_path => {
            Some(view(edit, depth, error_color))
        },
        _ => None,
    }
}

/// Whether the row of `context` is being renamed: the name field shows in
/// its place.
pub(crate) fn is_renamed_row(
    edit: Option<&InlineEdit>,
    context: &TreeRowContext<'_, FileNode>,
) -> bool {
    matches!(
        edit.map(|edit| &edit.kind),
        Some(InlineEditKind::Rename { path }) if path == &context.entry.path
    )
}

/// The name field at tree depth `depth`, with the error of the last try
/// under it. Enter uses the name, Esc stops naming.
fn view(
    edit: &InlineEdit,
    depth: usize,
    error_color: Color,
) -> Element<'_, ExplorerIntent> {
    let indent = depth as f32 * size::TREE_INDENT;

    let input = text_input("", &edit.value)
        .on_input(ExplorerIntent::InlineEditChanged)
        .on_submit(ExplorerIntent::InlineEditSubmit)
        .size(typography::BODY.size())
        .id(edit.id.clone());
    // Esc cancels the edit; a plain text input would only lose its focus.
    let input = InputKeys::new(input)
        .on_key(Named::Escape, ExplorerIntent::CancelInlineEdit);

    let mut field = column![
        row![Space::new().width(Length::Fixed(indent)), input]
            .spacing(spacing::SPACE_2)
            .width(Length::Fill)
            .height(Length::Fixed(size::ROW))
            .align_y(alignment::Vertical::Center),
    ]
    .width(Length::Fill);

    if let Some(error) = &edit.error {
        let left = spacing::SPACE_2 + indent + spacing::SPACE_1;
        field = field.push(
            container(text(error).size(typography::CAPTION.size()))
                .padding(Padding::default().left(left))
                .style(move |_| container::Style {
                    text_color: Some(error_color),
                    ..Default::default()
                }),
        );
    }

    container(field).width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use iced::Color;
    use iced::keyboard::key::Named;
    use iced_test::simulator;

    use super::view;
    use crate::event::ExplorerIntent;
    use crate::interaction::{InlineEdit, InlineEditKind};

    fn new_file_edit(value: &str) -> InlineEdit {
        InlineEdit::new(
            InlineEditKind::NewFile { parent: Vec::new() },
            String::from(value),
        )
    }

    /// Esc in the name field cancels the edit; without it the field would
    /// only lose its focus and stay.
    #[test]
    fn esc_in_the_name_field_cancels_it() {
        let edit = new_file_edit("notes.md");
        let mut ui = simulator(view(&edit, 0, Color::BLACK));

        ui.click(edit.id.clone())
            .expect("the name field should show");
        ui.tap_key(Named::Escape);

        let intents: Vec<ExplorerIntent> = ui.into_messages().collect();
        assert!(matches!(
            intents.as_slice(),
            [ExplorerIntent::CancelInlineEdit]
        ));
    }

    #[test]
    fn the_error_of_the_last_try_shows_under_the_field() {
        let mut edit = new_file_edit("");
        edit.error = Some(String::from("Enter a name."));
        let mut ui = simulator(view(&edit, 1, Color::BLACK));

        assert!(ui.find("Enter a name.").is_ok());
    }
}
