//! `FormSection`: a heading with the fields of one part of a form.

use iced::widget::{column, text};
use iced::{Element, Length};

use crate::palette::UiPalette;
use crate::{fonts, spacing, typography};

/// Titled group of fields, `space-4` apart. Stack sections in a column with
/// `spacing::SPACE_6` between them.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::form_section::FormSection;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let section: Element<'_, ()> = FormSection::new("Command", &palette)
///     .field(text("Program"))
///     .field(text("Arguments"))
///     .into();
/// ```
pub struct FormSection<'a, Message> {
    title: String,
    palette: UiPalette,
    fields: Vec<Element<'a, Message>>,
}

impl<'a, Message> FormSection<'a, Message> {
    /// Creates a section without fields.
    pub fn new(title: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            title: title.into(),
            palette: palette.clone(),
            fields: Vec::new(),
        }
    }

    /// Appends a field.
    pub fn field(mut self, field: impl Into<Element<'a, Message>>) -> Self {
        self.fields.push(field.into());
        self
    }
}

impl<'a, Message: 'a> From<FormSection<'a, Message>> for Element<'a, Message> {
    fn from(value: FormSection<'a, Message>) -> Self {
        let style = typography::HEADING;

        let mut fields = column![].spacing(spacing::SPACE_4);
        for field in value.fields {
            fields = fields.push(field);
        }

        column![
            text(value.title)
                .size(style.size())
                .font(fonts::UI_SEMIBOLD)
                .color(value.palette.text_strong()),
            fields,
        ]
        .spacing(spacing::SPACE_4)
        .width(Length::Fill)
        .into()
    }
}
