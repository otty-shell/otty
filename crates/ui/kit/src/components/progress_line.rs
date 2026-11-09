//! `ProgressLine`: a muted label, a [`ProgressBar`] and how much is done, in
//! one line that goes on to the next when it does not fit.
//!
//! [`ProgressBar`]: super::progress_bar::ProgressBar

use iced::widget::{row, text};
use iced::{Alignment, Element, Pixels};

use super::progress_bar::ProgressBar;
use crate::palette::UiPalette;
use crate::{fonts, spacing, typography};

/// What a [`progress_line`] shows. Plain data, so a view model can hold and
/// compare it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressLine {
    label: String,
    fraction: f32,
    amount: Option<String>,
}

impl ProgressLine {
    /// Returns the text before the bar, e.g. "Pulling nginx:latest".
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns how far the bar is filled, from 0.0 to 1.0.
    pub fn fraction(&self) -> f32 {
        self.fraction
    }

    /// Returns the text after the bar, e.g. "15.0 MB of 47.0 MB", if any.
    pub fn amount(&self) -> Option<&str> {
        self.amount.as_deref()
    }

    /// Creates a line with `label` and the bar filled to `fraction`, from 0.0
    /// to 1.0, with no amount.
    pub fn new(label: impl Into<String>, fraction: f32) -> Self {
        Self {
            label: label.into(),
            fraction,
            amount: None,
        }
    }

    /// Sets the text after the bar. `None` shows nothing there.
    pub fn with_amount(mut self, amount: Option<String>) -> Self {
        self.amount = amount;
        self
    }
}

/// Returns `line` drawn with the colors of `palette`: the label in small
/// muted text, the bar, then the amount in the mono font.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::progress_line::{ProgressLine, progress_line};
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let line = ProgressLine::new("Pulling nginx:latest", 0.3)
///     .with_amount(Some("15.0 MB of 47.0 MB".to_string()));
/// let view: Element<'_, ()> = progress_line(line, &palette);
/// ```
pub fn progress_line<'a, Message: 'a>(
    line: ProgressLine,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let hint = typography::SMALL;
    let muted = |content: String| {
        text(content)
            .size(hint.size())
            .line_height(Pixels(hint.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted())
    };

    let mut parts =
        row![muted(line.label), ProgressBar::new(line.fraction, palette),]
            .spacing(spacing::SPACE_3)
            .align_y(Alignment::Center);
    if let Some(amount) = line.amount {
        parts = parts.push(muted(amount).font(fonts::MONO));
    }

    parts.wrap().vertical_spacing(spacing::SPACE_1).into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    #[test]
    fn given_a_line_with_an_amount_then_the_label_and_the_amount_are_shown() {
        let palette = UiPalette::otty_dark();
        let line = ProgressLine::new("Pulling nginx:latest", 0.3)
            .with_amount(Some("15.0 MB of 47.0 MB".to_string()));
        let mut ui = simulator(progress_line::<()>(line, &palette));

        assert!(ui.find("Pulling nginx:latest").is_ok());
        assert!(ui.find("15.0 MB of 47.0 MB").is_ok());
    }
}
