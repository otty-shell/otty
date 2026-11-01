//! `FormPage`: a scrolling page that holds a form.

use iced::widget::{column, container, scrollable};
use iced::{Alignment, Element, Length, Padding};

use super::{divider, scrollbar};
use crate::palette::UiPalette;
use crate::{size, spacing};

/// Height of the footer bar (`.form-foot`: `height: 60px`).
const FOOTER_BAR_HEIGHT: f32 = 60.0;

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
    footer_bar: Option<Element<'a, Message>>,
}

impl<'a, Message> FormPage<'a, Message> {
    /// Creates an empty page.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            sections: Vec::new(),
            footer: None,
            footer_bar: None,
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

    /// Sets a bar fixed under the scrolling sections (`.form-foot`): 60
    /// high, a line on top, `space-7` side padding. Use it for a form whose
    /// action must stay in reach, such as Run container.
    pub fn footer_bar(mut self, bar: impl Into<Element<'a, Message>>) -> Self {
        self.footer_bar = Some(bar.into());
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

        let Some(bar) = value.footer_bar else {
            return container(scrollable)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        };

        let canvas = value.palette.surface_canvas();
        let bar = container(bar)
            .width(Length::Fill)
            .height(FOOTER_BAR_HEIGHT - size::LINE)
            .padding(Padding::from([0.0, spacing::SPACE_7]))
            .align_y(Alignment::Center)
            .style(move |_| container::Style {
                background: Some(canvas.into()),
                ..container::Style::default()
            });

        column![scrollable, divider::horizontal(&value.palette), bar]
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
    fn given_a_footer_bar_when_laid_out_then_it_stays_at_the_bottom() {
        let palette = UiPalette::otty_dark();
        let mut page = FormPage::new(&palette);
        for index in 0..40 {
            page = page.section(text(format!("section {index}")));
        }
        let page: Element<'_, ()> = page.footer_bar(text("Run")).into();
        let mut ui = simulator(page);

        let run = ui.find("Run").expect("footer should be visible").bounds();

        let window_height = 768.0;
        let bar_top = window_height - FOOTER_BAR_HEIGHT;
        assert!(
            run.y > bar_top && run.y + run.height < window_height,
            "footer text at {run:?} should be inside the bar from {bar_top}"
        );
        assert!(ui.find("section 39").is_ok());
    }

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
