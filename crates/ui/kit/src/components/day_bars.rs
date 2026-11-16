//! `DayBars`: one bar per day on a baseline, such as the tokens of each of
//! the last 30 days.

use iced::widget::{Space, column, container, row, text, tooltip};
use iced::{Alignment, Border, Color, Element, Length, border};

use super::divider;
use super::icon_button::tooltip_hint;
use crate::palette::UiPalette;
use crate::{fonts, spacing};

/// Height of the bars' area unless set (`height: 96px`).
pub const DEFAULT_HEIGHT: f32 = 96.0;
/// Gap between two bars (`gap: 4px`).
const BAR_GAP: f32 = 4.0;
/// Room left above the highest bar, so it does not touch what is above
/// (the canvas draws the highest bar 92 of 96 high).
const TOP_ROOM: f32 = 4.0;
/// Height of the lowest bar of a value above zero, so it stays visible.
const MIN_BAR: f32 = 2.0;
/// Corner radius of the top of a bar (`border-radius: 2px 2px 0 0`).
const BAR_RADIUS: f32 = 2.0;
/// Text size of the labels under the baseline (`font-size: 11.5px`).
const AXIS_TEXT_SIZE: f32 = 11.5;

/// Bars of one value per day, oldest first and today last, standing on a
/// baseline. Today's bar is in the `accent` color, the others in
/// `text-faint`. The highest value fills the height; a value above zero is
/// never thinner than a sliver, and zero has no bar. Each bar shows its
/// tooltip, such as "2026-11-13: 2.9M tokens", under the pointer, and the
/// whole height above the bar takes the pointer, so a low bar is easy to
/// hover. Two labels under the baseline can name the first and the last
/// day.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::day_bars::DayBars;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let bars: Element<'_, ()> = DayBars::new(&palette)
///     .bar(1.2, "Nov 15: 1.2M tokens")
///     .bar(0.4, "Today: 0.4M tokens")
///     .axis("Nov 15", "Today")
///     .into();
/// ```
pub struct DayBars {
    bars: Vec<(f64, String)>,
    height: f32,
    axis: Option<(String, String)>,
    is_muted: bool,
    palette: UiPalette,
}

impl DayBars {
    /// Creates bars with no day.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            bars: Vec::new(),
            height: DEFAULT_HEIGHT,
            axis: None,
            is_muted: false,
            palette: palette.clone(),
        }
    }

    /// Adds the bar of the next day, of `value`, with its `tooltip`.
    pub fn bar(mut self, value: f64, tooltip: impl Into<String>) -> Self {
        self.bars.push((value, tooltip.into()));
        self
    }

    /// Sets the height of the bars' area; the default is
    /// [`DEFAULT_HEIGHT`].
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Draws every bar in `text-faint`, today's too, while `is_muted`, for
    /// numbers that are no longer current, such as the last good count
    /// kept after a failed one.
    pub fn muted(mut self, is_muted: bool) -> Self {
        self.is_muted = is_muted;
        self
    }

    /// Names the first and the last day under the baseline, such as
    /// "Oct 18" and "Today".
    pub fn axis(
        mut self,
        first: impl Into<String>,
        last: impl Into<String>,
    ) -> Self {
        self.axis = Some((first.into(), last.into()));
        self
    }
}

impl<'a, Message: 'a> From<DayBars> for Element<'a, Message> {
    fn from(value: DayBars) -> Self {
        let palette = value.palette;
        let values: Vec<f64> = value.bars.iter().map(|(bar, _)| *bar).collect();
        let heights = bar_heights(&values, value.height);
        let count = value.bars.len();

        let mut bars = row![]
            .spacing(BAR_GAP)
            .height(value.height)
            .align_y(Alignment::End);
        for (index, ((_, tip), height)) in
            value.bars.into_iter().zip(heights).enumerate()
        {
            let color = bar_color(index, count, value.is_muted, &palette);
            let bar = container(Space::new())
                .width(Length::Fill)
                .height(height)
                .style(move |_| container::Style {
                    background: Some(color.into()),
                    border: Border {
                        radius: border::radius(0.0)
                            .top_left(BAR_RADIUS)
                            .top_right(BAR_RADIUS),
                        ..Border::default()
                    },
                    ..container::Style::default()
                });
            let hover_area = container(bar)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::End);

            bars = bars.push(
                tooltip(
                    hover_area,
                    tooltip_hint(tip, &palette),
                    tooltip::Position::Top,
                )
                .gap(spacing::SPACE_1),
            );
        }

        let mut chart = column![bars, divider::horizontal(&palette)]
            .spacing(0.0)
            .width(Length::Fill);
        if let Some((first, last)) = value.axis {
            let muted = palette.text_muted();
            let label = move |content: String| {
                text(content)
                    .size(AXIS_TEXT_SIZE)
                    .font(fonts::UI)
                    .color(muted)
            };

            chart = chart.push(
                row![label(first).width(Length::Fill), label(last)]
                    .padding(iced::Padding::ZERO.top(spacing::SPACE_2)),
            );
        }

        chart.into()
    }
}

/// The height of the bar of each of `values` in an area `area_height`
/// high: the highest value reaches the top room, the others in proportion.
/// A value above zero is at least a sliver high; zero, a value below it
/// and a value that is not a number have no bar.
fn bar_heights(values: &[f64], area_height: f32) -> Vec<f32> {
    let highest = values
        .iter()
        .copied()
        .filter(|value| *value > 0.0)
        .fold(0.0, f64::max);
    let room = f64::from((area_height - TOP_ROOM).max(0.0));

    values
        .iter()
        .map(|value| {
            if highest <= 0.0 || value.is_nan() || *value <= 0.0 {
                return 0.0;
            }

            let height = (value / highest * room).round() as f32;
            height.max(MIN_BAR)
        })
        .collect()
}

/// The color of the bar at `index` of `count`: today, the last one, in the
/// accent color unless the bars are muted; the others in `text-faint`.
fn bar_color(
    index: usize,
    count: usize,
    is_muted: bool,
    palette: &UiPalette,
) -> Color {
    if index + 1 == count && !is_muted {
        palette.accent()
    } else {
        palette.text_faint()
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::{column, text};
    use iced_test::simulator;

    use super::*;

    #[test]
    fn the_highest_value_fills_the_area_and_the_others_follow_it() {
        let heights = bar_heights(&[0.0, 1.0, 2.0], 96.0);

        assert_eq!(heights, [0.0, 46.0, 92.0]);
    }

    #[test]
    fn a_small_value_is_a_sliver_and_no_value_has_no_bar() {
        let small = bar_heights(&[0.001, 1000.0], 96.0);
        let nothing = bar_heights(&[0.0, 0.0], 96.0);
        let odd = bar_heights(&[f64::NAN, -3.0, 4.0], 96.0);

        assert_eq!(small, [MIN_BAR, 92.0]);
        assert_eq!(nothing, [0.0, 0.0]);
        assert_eq!(odd, [0.0, 0.0, 92.0]);
    }

    #[test]
    fn today_is_in_the_accent_color_and_the_other_days_are_faint() {
        let palette = UiPalette::otty_dark();

        let colors: Vec<Color> = (0..3)
            .map(|index| bar_color(index, 3, false, &palette))
            .collect();

        assert_eq!(
            colors,
            [palette.text_faint(), palette.text_faint(), palette.accent()]
        );
    }

    #[test]
    fn muted_bars_are_all_faint_today_too() {
        let palette = UiPalette::otty_light();

        assert_eq!(bar_color(2, 3, true, &palette), palette.text_faint());
    }

    #[test]
    fn the_axis_names_the_first_and_the_last_day_under_the_bars() {
        let palette = UiPalette::otty_light();
        let view: Element<'_, ()> = column![
            DayBars::new(&palette)
                .bar(1.0, "Nov 15: 1")
                .bar(2.0, "Today: 2")
                .height(60.0)
                .axis("Nov 15", "Today"),
            text("below"),
        ]
        .width(300)
        .into();
        let mut ui = simulator(view);

        let first = ui.find("Nov 15").expect("the first day").bounds();
        let last = ui.find("Today").expect("the last day").bounds();
        let below = ui.find("below").expect("text under the bars").bounds();

        assert!(first.y > 60.0, "{first:?}");
        assert!(last.x > first.x, "{last:?}");
        assert!(below.y > first.y, "{below:?}");
    }

    #[test]
    fn bars_with_no_day_draw_only_the_baseline() {
        let palette = UiPalette::otty_dark();
        let view: Element<'_, ()> =
            column![DayBars::new(&palette).height(40.0), text("below")].into();
        let mut ui = simulator(view);

        let below = ui.find("below").expect("text under the bars").bounds();

        assert!(below.y >= 40.0, "{below:?}");
    }
}
