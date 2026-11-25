//! `ProgressBar`: a thin bar that fills as work goes on (`.prog`).

use iced::widget::{Space, container, row};
use iced::{Border, Element, Length, border};

use crate::palette::UiPalette;

/// Width of the bar unless set (`.prog { width: 160px }`).
pub const DEFAULT_WIDTH: f32 = 160.0;
/// Height of the bar (`.prog { height: 4px }`).
pub const HEIGHT: f32 = 4.0;
/// Corner radius of the track and the fill (`border-radius: 2px`).
const RADIUS: f32 = 2.0;
/// How much of `text-muted` the empty track takes.
const TRACK_ALPHA: f32 = 0.28;
/// The least contrast of the empty track against the surface under it, so
/// the part of a limit not used yet still shows.
#[cfg(test)]
const TRACK_MIN: f32 = 1.3;
/// How many shares the width of a bar as wide as its parent is cut into:
/// a tenth of a percent each.
const PORTIONS: u16 = 1000;

/// Thin bar for work that takes a while, such as an image pull: an
/// `accent` fill on a track of `text-muted` at a low alpha. It does not react to the
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
    is_fill_width: bool,
    is_muted: bool,
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
            is_fill_width: false,
            is_muted: false,
            palette: palette.clone(),
        }
    }

    /// Sets the width of the bar; the default is [`DEFAULT_WIDTH`].
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Makes the bar as wide as its parent, such as a column of a
    /// dashboard, instead of a fixed width.
    pub fn fill_width(mut self) -> Self {
        self.is_fill_width = true;
        self
    }

    /// Fills the bar in `text-muted` instead of `accent` while `is_muted`,
    /// for a number that is no longer current, such as the last good
    /// answer kept after a failed read.
    pub fn muted(mut self, is_muted: bool) -> Self {
        self.is_muted = is_muted;
        self
    }

    /// The color of the filled part.
    fn fill_color(&self) -> iced::Color {
        if self.is_muted {
            self.palette.text_muted()
        } else {
            self.palette.accent()
        }
    }
}

/// The empty track: `text-muted` at [`TRACK_ALPHA`], so it shows on the
/// canvas, on a card and on a dialog alike (a solid surface color matched
/// one of them and vanished there).
fn track_color(palette: &UiPalette) -> iced::Color {
    palette.text_muted().scale_alpha(TRACK_ALPHA)
}

/// The color the empty track shows on `surface`.
#[cfg(test)]
fn track_on(palette: &UiPalette, surface: iced::Color) -> iced::Color {
    let track = track_color(palette);
    let mix = |under: f32, over: f32| under + (over - under) * track.a;

    iced::Color::from_rgb(
        mix(surface.r, track.r),
        mix(surface.g, track.g),
        mix(surface.b, track.b),
    )
}

impl<'a, Message: 'a> From<ProgressBar> for Element<'a, Message> {
    fn from(value: ProgressBar) -> Self {
        let track_color = track_color(&value.palette);
        let fill_color = value.fill_color();
        let shape = Border {
            radius: border::radius(RADIUS),
            ..Border::default()
        };

        let fill = |width: Length| {
            container(Space::new()).width(width).height(HEIGHT).style(
                move |_| container::Style {
                    background: Some(fill_color.into()),
                    border: shape,
                    ..container::Style::default()
                },
            )
        };

        let (filled, track_width): (Element<'a, Message>, Length) =
            if value.is_fill_width {
                (fill_portions(value.fraction, fill), Length::Fill)
            } else {
                let width = value.width * value.fraction;
                (
                    fill(Length::Fixed(width)).into(),
                    Length::Fixed(value.width),
                )
            };

        container(filled)
            .width(track_width)
            .height(HEIGHT)
            .style(move |_| container::Style {
                background: Some(track_color.into()),
                border: shape,
                ..container::Style::default()
            })
            .into()
    }
}

/// The filled part of a bar as wide as its parent: `fraction` of the
/// width, as a share of a row, so it follows the width the bar gets.
fn fill_portions<'a, Message: 'a, Fill>(
    fraction: f32,
    fill: impl Fn(Length) -> Fill,
) -> Element<'a, Message>
where
    Fill: Into<Element<'a, Message>>,
{
    let filled = (fraction * PORTIONS as f32).round() as u16;
    let empty = PORTIONS - filled.min(PORTIONS);

    let mut parts = row![];
    if filled > 0 {
        parts = parts.push(fill(Length::FillPortion(filled)));
    }
    if empty > 0 {
        parts = parts.push(Space::new().width(Length::FillPortion(empty)));
    }

    parts.width(Length::Fill).into()
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
    fn a_bar_that_fills_its_parent_takes_the_room_left_in_its_row() {
        let palette = UiPalette::otty_dark();
        let view: Element<'_, ()> = iced::widget::container(row![
            ProgressBar::new(0.25, &palette).fill_width(),
            text("after"),
        ])
        .width(400.0)
        .into();
        let mut ui = simulator(view);

        let after = ui.find("after").expect("text after the bar").bounds();

        assert!(after.x > DEFAULT_WIDTH + 100.0, "{after:?}");
    }

    #[test]
    fn a_muted_bar_fills_in_the_muted_text_color() {
        let palette = UiPalette::otty_dark();

        let lit = ProgressBar::new(0.5, &palette);
        let muted = ProgressBar::new(0.5, &palette).muted(true);

        assert_eq!(lit.fill_color(), palette.accent());
        assert_eq!(muted.fill_color(), palette.text_muted());
    }

    #[test]
    fn given_fraction_outside_zero_to_one_when_built_then_it_is_clamped() {
        let palette = UiPalette::otty_dark();

        assert_eq!(ProgressBar::new(1.7, &palette).fraction, 1.0);
        assert_eq!(ProgressBar::new(-0.2, &palette).fraction, 0.0);
        assert_eq!(ProgressBar::new(f32::NAN, &palette).fraction, 0.0);
        assert_eq!(ProgressBar::new(0.4, &palette).fraction, 0.4);
    }

    /// The empty part of the bar shows on every surface a bar sits on: the
    /// canvas, a card and a dialog.
    #[test]
    fn given_each_theme_when_drawn_then_the_track_shows_on_cards_and_dialogs() {
        for (theme, palette) in [
            ("OTTY Dark", UiPalette::otty_dark()),
            ("OTTY Light", UiPalette::otty_light()),
        ] {
            let surfaces = [
                ("canvas", palette.surface_canvas()),
                ("card", super::super::card::raised(&palette)),
                ("dialog", palette.surface_overlay()),
            ];
            for (name, surface) in surfaces {
                let track = track_on(&palette, surface);
                let ratio = contrast_ratio(track, surface);

                assert!(
                    ratio >= TRACK_MIN,
                    "{theme}: track on {name} is {ratio:.2}:1"
                );
            }
        }
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
