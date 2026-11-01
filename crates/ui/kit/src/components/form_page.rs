//! `FormPage`: a scrolling page that holds a form.

use iced::widget::{column, container, scrollable};
use iced::{Element, Length};

use super::scrollbar;
use crate::palette::UiPalette;
use crate::{size, spacing};

/// Scrolling form page: sections `space-6` apart and an optional footer.
/// The page title is the caller's first section, since it often carries
/// actions next to the text. The content is at most `size::FORM` wide,
/// centered horizontally, and has `space-7` padding.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::form_page::FormPage;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let page: Element<'_, ()> = FormPage::new(&palette)
///     .section(text("Settings"))
///     .section(text("General"))
///     .footer(text("Save"))
///     .into();
/// ```
pub struct FormPage<'a, Message> {
    palette: UiPalette,
    sections: Vec<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
}

impl<'a, Message> FormPage<'a, Message> {
    /// Creates an empty page.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            sections: Vec::new(),
            footer: None,
        }
    }

    /// Appends a section.
    pub fn section(mut self, section: impl Into<Element<'a, Message>>) -> Self {
        self.sections.push(section.into());
        self
    }

    /// Sets the footer, shown after the last section.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }
}

impl<'a, Message: 'a> From<FormPage<'a, Message>> for Element<'a, Message> {
    fn from(value: FormPage<'a, Message>) -> Self {
        let mut content = column![]
            .spacing(spacing::SPACE_6)
            .width(Length::Fill)
            .max_width(size::FORM);

        for section in value.sections {
            content = content.push(section);
        }
        if let Some(footer) = value.footer {
            content = content.push(footer);
        }

        let scrollable = scrollable(
            container(content)
                .padding(spacing::SPACE_7)
                .center_x(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(&value.palette));

        container(scrollable)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::text;
    use iced_test::simulator;

    use super::*;

    #[test]
    fn given_wide_window_when_laid_out_then_form_is_centered() {
        let palette = UiPalette::otty_dark();
        let page: Element<'_, ()> = FormPage::new(&palette)
            .section(container(text("start")).width(Length::Fill))
            .into();
        let mut ui = simulator(page);

        let start = ui.find("start").expect("marker should be visible");

        let window_width = 1024.0;
        let expected_x = (window_width - size::FORM) / 2.0;
        assert!(
            (start.bounds().x - expected_x).abs() < 0.5,
            "form left edge {} should be {expected_x}",
            start.bounds().x
        );
    }
}
