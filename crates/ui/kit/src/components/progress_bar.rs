//! `ProgressBar`: a thin bar that fills as work goes on (`.prog`).

use iced::widget::{Space, container};
use iced::{Border, Element, border};

use crate::palette::UiPalette;

/// Width of the bar unless set (`.prog { width: 160px }`).
pub const DEFAULT_WIDTH: f32 = 160.0;
/// Height of the bar (`.prog { height: 4px }`).
pub const HEIGHT: f32 = 4.0;
/// Corner radius of the track and the fill (`border-radius: 2px`).
const RADIUS: f32 = 2.0;

/// Thin bar for work that takes a while, such as an image pull: an
/// `accent` fill on a `surface-raised` track. It does not react to the
/// pointer; put a text next to it that says what runs and how far it is.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::progress_bar::ProgressBar;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let bar: Element<'_, ()> = ProgressBar::new(0.32, &palette).into();
/// ```
pub struct ProgressBar {
    fraction: f32,
    width: f32,
    palette: UiPalette,
}

impl ProgressBar {
    /// Creates a bar filled to `fraction`, from 0 (empty) to 1 (full);
    /// values outside are clamped and NaN counts as empty.
    pub fn new(fraction: f32, palette: &UiPalette) -> Self {
        let fraction = if fraction.is_nan() {
            0.0
        } else {
            fraction.clamp(0.0, 1.0)
        };

        Self {
            fraction,
            width: DEFAULT_WIDTH,
            palette: palette.clone(),
        }
    }

    /// Sets the width of the bar; the default is [`DEFAULT_WIDTH`].
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

impl<'a, Message: 'a> From<ProgressBar> for Element<'a, Message> {
    fn from(value: ProgressBar) -> Self {
        let track_color = value.palette.surface_raised();
        let fill_color = value.palette.accent();
        let shape = Border {
            radius: border::radius(RADIUS),
            ..Border::default()
        };

        let fill = container(Space::new())
            .width(value.width * value.fraction)
            .height(HEIGHT)
            .style(move |_| container::Style {
                background: Some(fill_color.into()),
                border: shape,
                ..container::Style::default()
            });

        container(fill)
            .width(value.width)
            .height(HEIGHT)
            .style(move |_| container::Style {
                background: Some(track_color.into()),
                border: shape,
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::{column, row, text};
    use iced_test::simulator;

    use super::*;
    use crate::contrast::contrast_ratio;

    /// WCAG 2.1 AA minimum for non-text marks.
    const NON_TEXT_MIN: f32 = 3.0;

    #[test]
    fn given_bar_when_laid_out_then_it_is_160_by_4() {
        let palette = UiPalette::otty_dark();
        let view: Element<'_, ()> = column![
            row![ProgressBar::new(0.3, &palette), text("after")],
            text("below"),
        ]
        .into();
        let mut ui = simulator(view);

        let after = ui.find("after").expect("text after the bar").bounds();
        let below = ui.find("below").expect("text under the row").bounds();

        assert_eq!(after.x, DEFAULT_WIDTH);
        assert!(below.y >= HEIGHT, "{below:?}");
    }

    #[test]
    fn given_width_when_laid_out_then_the_bar_takes_it() {
        let palette = UiPalette::otty_light();
        let view: Element<'_, ()> =
            row![ProgressBar::new(1.0, &palette).width(240.0), text("after")]
                .into();
        let mut ui = simulator(view);

        let after = ui.find("after").expect("text after the bar").bounds();

        assert_eq!(after.x, 240.0);
    }

    #[test]
    fn given_fraction_outside_zero_to_one_when_built_then_it_is_clamped() {
        let palette = UiPalette::otty_dark();

        assert_eq!(ProgressBar::new(1.7, &palette).fraction, 1.0);
        assert_eq!(ProgressBar::new(-0.2, &palette).fraction, 0.0);
        assert_eq!(ProgressBar::new(f32::NAN, &palette).fraction, 0.0);
        assert_eq!(ProgressBar::new(0.4, &palette).fraction, 0.4);
    }

    #[test]
    fn given_each_theme_when_drawn_then_the_fill_stands_out_on_dialogs() {
        for (theme, palette) in [
            ("OTTY Dark", UiPalette::otty_dark()),
            ("OTTY Light", UiPalette::otty_light()),
        ] {
            let ratio =
                contrast_ratio(palette.accent(), palette.surface_overlay());

            assert!(
                ratio >= NON_TEXT_MIN,
                "{theme}: fill on surface-overlay is {ratio:.2}:1"
            );
        }
    }
}
