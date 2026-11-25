//! `DayBars`: one bar per day on a baseline, such as the tokens of each of
//! the last 30 days.

use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Border, Color, Element, Length, border};

use super::divider;
use super::tooltip::{TooltipText, tooltip_above};
use crate::palette::UiPalette;
use crate::{fonts, spacing};

/// Height of the bars' area (`height: 96px`).
const HEIGHT: f32 = 96.0;
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

/// One bar: its value, the part of it lit in the accent, and its tooltip.
struct Bar {
    value: f64,
    part: Option<f64>,
    tooltip: TooltipText,
}

/// Bars of one value per day, oldest first and today last, standing on a
/// baseline. Today's bar is in the `accent` color, the others in
/// `text-faint`; [`DayBars::current`] names another bar for the accent,
/// such as the current hour. A bar added with [`DayBars::bar_with_part`]
/// shows its part in the accent at its bottom, such as the share of one
/// project, and then no bar is in the accent as a whole. The highest value
/// fills the height; a value above zero is never thinner than a sliver, and
/// zero has no bar. Each bar shows its tooltip, such as "2026-11-13: 2.9M
/// tokens" or a [`TooltipText`] with rows of what the bar is made of, over
/// the pointer, and the whole height above the bar takes the pointer, so a
/// low bar is easy to hover. Two labels under the baseline can name the
/// first and the last day.
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
/// let hours: Element<'_, ()> = DayBars::new(&palette)
///     .bar_with_part(3.0, 1.0, "12:00: 3M\notty: 1M")
///     .bar_with_part(2.0, 0.0, "13:00: 2M\notty: 0")
///     .axis("00:00", "23:00")
///     .into();
/// ```
pub struct DayBars {
    bars: Vec<Bar>,
    current: Option<usize>,
    axis: Option<(String, String)>,
    is_muted: bool,
    palette: UiPalette,
}

impl DayBars {
    /// Creates bars with no day.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            bars: Vec::new(),
            current: None,
            axis: None,
            is_muted: false,
            palette: palette.clone(),
        }
    }

    /// Adds the bar of the next day, of `value`, with its `tooltip`.
    pub fn bar(mut self, value: f64, tooltip: impl Into<TooltipText>) -> Self {
        self.bars.push(Bar {
            value,
            part: None,
            tooltip: tooltip.into(),
        });
        self
    }

    /// Adds the bar of the next day, of `value`, whose `part` is lit in the
    /// accent color at its bottom and the rest drawn faint, such as the
    /// tokens of one project in all the tokens of the day.
    pub fn bar_with_part(
        mut self,
        value: f64,
        part: f64,
        tooltip: impl Into<TooltipText>,
    ) -> Self {
        self.bars.push(Bar {
            value,
            part: Some(part),
            tooltip: tooltip.into(),
        });
        self
    }

    /// Draws the bar at `index` in the accent color in place of the last
    /// one, such as the current hour of today when later hours follow.
    pub fn current(mut self, index: usize) -> Self {
        self.current = Some(index);
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

    /// The bar drawn in the accent color: the one named with
    /// [`current`](Self::current), else the last. None while a bar has a
    /// lit part, since the lit parts are in the accent then.
    fn current_bar(&self) -> Option<usize> {
        if self.bars.iter().any(|bar| bar.part.is_some()) {
            return None;
        }

        self.current.or_else(|| self.bars.len().checked_sub(1))
    }
}

impl<'a, Message: 'a> From<DayBars> for Element<'a, Message> {
    fn from(value: DayBars) -> Self {
        let palette = value.palette.clone();
        let current = value.current_bar();
        let values: Vec<f64> = value.bars.iter().map(|bar| bar.value).collect();
        let heights = bar_heights(&values, HEIGHT);

        let mut bars = row![]
            .spacing(BAR_GAP)
            .height(HEIGHT)
            .align_y(Alignment::End);
        for (index, (bar, height)) in
            value.bars.into_iter().zip(heights).enumerate()
        {
            let color = bar_color(index, current, value.is_muted, &palette);
            let lit = bar
                .part
                .map_or(0.0, |part| part_height(height, bar.value, part));
            let column = column![
                bar_piece(height - lit, color, true),
                bar_piece(lit, palette.accent(), lit >= height),
            ];
            let hover_area = container(column)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::End);

            bars = bars.push(tooltip_above(hover_area, bar.tooltip, &palette));
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

/// One piece of a bar, `height` high, in `color`; with rounded top corners
/// when it is the top of the bar.
fn bar_piece<'a, Message: 'a>(
    height: f32,
    color: Color,
    is_top: bool,
) -> Element<'a, Message> {
    let radius = if is_top { BAR_RADIUS } else { 0.0 };

    container(Space::new())
        .width(Length::Fill)
        .height(height.max(0.0))
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: Border {
                radius: border::radius(0.0).top_left(radius).top_right(radius),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
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

/// How high the lit `part` of a bar of `value`, `height` high, is drawn:
/// its share of the bar, at least a sliver when it is above zero, and never
/// higher than the bar.
fn part_height(height: f32, value: f64, part: f64) -> f32 {
    if value <= 0.0 || part.is_nan() || part <= 0.0 || height <= 0.0 {
        return 0.0;
    }

    let share = (part / value).min(1.0);
    let lit = (f64::from(height) * share).round() as f32;
    lit.clamp(MIN_BAR.min(height), height)
}

/// The color of the bar at `index`: the `current` one in the accent color
/// unless the bars are muted; the others in `text-faint`.
fn bar_color(
    index: usize,
    current: Option<usize>,
    is_muted: bool,
    palette: &UiPalette,
) -> Color {
    if current == Some(index) && !is_muted {
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
            .map(|index| bar_color(index, Some(2), false, &palette))
            .collect();

        assert_eq!(
            colors,
            [palette.text_faint(), palette.text_faint(), palette.accent()]
        );
    }

    #[test]
    fn the_current_bar_can_be_another_than_the_last() {
        let palette = UiPalette::otty_dark();

        let colors: Vec<Color> = (0..3)
            .map(|index| bar_color(index, Some(1), false, &palette))
            .collect();

        assert_eq!(
            colors,
            [palette.text_faint(), palette.accent(), palette.text_faint()]
        );
    }

    #[test]
    fn the_last_bar_is_current_unless_another_is_named_or_parts_are_lit() {
        let palette = UiPalette::otty_dark();
        let plain = DayBars::new(&palette).bar(1.0, "a").bar(2.0, "b");
        let hour = DayBars::new(&palette)
            .bar(1.0, "a")
            .bar(2.0, "b")
            .bar(0.0, "c")
            .current(1);
        let lit = DayBars::new(&palette)
            .bar_with_part(1.0, 0.5, "a")
            .bar_with_part(2.0, 0.0, "b");

        assert_eq!(plain.current_bar(), Some(1));
        assert_eq!(hour.current_bar(), Some(1));
        assert_eq!(lit.current_bar(), None, "the lit parts take the accent");
        assert_eq!(DayBars::new(&palette).current_bar(), None);
    }

    #[test]
    fn the_lit_part_is_its_share_of_the_bar_and_never_lost() {
        assert_eq!(part_height(92.0, 4.0, 1.0), 23.0);
        assert_eq!(part_height(92.0, 4.0, 4.0), 92.0);
        assert_eq!(part_height(92.0, 4.0, 9.0), 92.0, "never above the bar");
        assert_eq!(part_height(92.0, 4.0, 0.0), 0.0);
        assert_eq!(part_height(92.0, 1000.0, 0.001), MIN_BAR, "a sliver");
        assert_eq!(part_height(0.0, 0.0, 0.0), 0.0);
        assert_eq!(part_height(2.0, 4.0, 1.0), 2.0, "as high as a sliver bar");
    }

    #[test]
    fn bars_with_lit_parts_show_their_tooltips_and_axis() {
        let palette = UiPalette::otty_light();
        let view: Element<'_, ()> = DayBars::new(&palette)
            .bar_with_part(1.0, 0.25, "08:00: 1")
            .bar_with_part(2.0, 2.0, "09:00: 2")
            .axis("00:00", "23:00")
            .into();
        let mut ui = simulator(view);

        ui.find("00:00").expect("the first point");
        ui.find("23:00").expect("the last point");
    }

    #[test]
    fn muted_bars_are_all_faint_today_too() {
        let palette = UiPalette::otty_light();

        assert_eq!(bar_color(2, Some(2), true, &palette), palette.text_faint());
    }

    #[test]
    fn the_axis_names_the_first_and_the_last_day_under_the_bars() {
        let palette = UiPalette::otty_light();
        let view: Element<'_, ()> = column![
            DayBars::new(&palette)
                .bar(1.0, "Nov 15: 1")
                .bar(2.0, "Today: 2")
                .axis("Nov 15", "Today"),
            text("below"),
        ]
        .width(300)
        .into();
        let mut ui = simulator(view);

        let first = ui.find("Nov 15").expect("the first day").bounds();
        let last = ui.find("Today").expect("the last day").bounds();
        let below = ui.find("below").expect("text under the bars").bounds();

        assert!(first.y > HEIGHT, "{first:?}");
        assert!(last.x > first.x, "{last:?}");
        assert!(below.y > first.y, "{below:?}");
    }

    #[test]
    fn bars_with_no_day_draw_only_the_baseline() {
        let palette = UiPalette::otty_dark();
        let view: Element<'_, ()> =
            column![DayBars::new(&palette), text("below")].into();
        let mut ui = simulator(view);

        let below = ui.find("below").expect("text under the bars").bounds();

        assert!(below.y >= HEIGHT, "{below:?}");
    }
}
