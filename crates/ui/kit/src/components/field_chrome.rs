//! Parts shared by the form fields: label row, note under the field and the
//! input style.

use iced::widget::{column, row, text, text_input};
use iced::{Border, Color, Element, Theme};

use super::faded;
use crate::palette::UiPalette;
use crate::{fonts, radius, spacing, typography};

/// Label size (`font-size: 12.5px`).
const LABEL_SIZE: f32 = 12.5;

/// Text shown under a field.
pub(super) enum Note {
    /// Explains what happens with the value, in `text-muted`.
    Hint(String),
    /// Says how to fix the value, in `status-error`.
    Error(String),
}

/// Stacks the label row, `body` and the optional `note` of one field.
///
/// An empty `label` leaves the label row out, for fields that sit inside a
/// row which already names them.
pub(super) fn labelled<'a, Message: 'a>(
    label: &str,
    is_optional: bool,
    body: Element<'a, Message>,
    note: Option<Note>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut label_row = row![
        text(label.to_owned())
            .size(LABEL_SIZE)
            .font(fonts::UI)
            .color(palette.text_body())
    ]
    .spacing(spacing::SPACE_1);
    if is_optional {
        label_row = label_row.push(
            text("optional")
                .size(LABEL_SIZE)
                .font(fonts::UI)
                .color(palette.text_muted()),
        );
    }

    let mut field = column![].spacing(spacing::SPACE_2);
    if !label.is_empty() {
        field = field.push(label_row);
    }
    field = field.push(body);
    if let Some(note) = note {
        let (content, color) = match note {
            Note::Hint(content) => (content, palette.text_muted()),
            Note::Error(content) => (content, palette.status_error()),
        };
        let style = typography::SMALL;
        field = field.push(
            text(content)
                .size(style.size())
                .font(fonts::UI)
                .color(color),
        );
    }

    field.into()
}

/// Returns the style of a boxed mono input. `invalid` colors the border
/// `status-error`; a disabled input is faded.
pub(super) fn input_style(
    palette: &UiPalette,
    invalid: bool,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style + 'static {
    let palette = palette.clone();

    move |_, status| {
        let disabled = status == text_input::Status::Disabled;
        let border_color = if invalid {
            palette.status_error()
        } else if matches!(status, text_input::Status::Focused { .. }) {
            palette.focus()
        } else {
            Color::TRANSPARENT
        };
        let fade = |color: Color| if disabled { faded(color) } else { color };

        text_input::Style {
            background: fade(palette.surface_raised()).into(),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: radius::RADIUS_SM.into(),
            },
            icon: palette.text_muted(),
            placeholder: palette.text_muted(),
            value: fade(palette.text_strong()),
            selection: Color {
                a: 0.35,
                ..palette.accent()
            },
        }
    }
}
