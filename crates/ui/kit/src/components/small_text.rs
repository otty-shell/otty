//! `small_text`: a line of small text, such as a hint under a field, a note
//! in an empty list or the subtitle of a form.

use iced::widget::text::{IntoFragment, Text};
use iced::{Color, Pixels};

use crate::{fonts, typography};

/// Returns `content` in the `small` text style (12/16) of the UI font, in
/// `color`, such as `text-muted` for a hint or `status-error` for an error.
/// It is a plain `iced` text, so the caller can still set its width or
/// wrapping.
///
/// # Example
///
/// ```
/// use iced::widget::Text;
/// use otty_ui_kit::components::small_text::small_text;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let hint: Text<'_> =
///     small_text("Leave empty to use the default", palette.text_muted());
/// ```
pub fn small_text<'a>(
    content: impl IntoFragment<'a>,
    color: Color,
) -> Text<'a> {
    let style = typography::SMALL;

    iced::widget::text(content)
        .size(style.size())
        .line_height(Pixels(style.line_height()))
        .font(fonts::UI)
        .color(color)
}
