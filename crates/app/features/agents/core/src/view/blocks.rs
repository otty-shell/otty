//! The pieces the blocks of a profile's dashboard share: a block title with
//! its muted note, the row of totals, the bars per day and small lines.

use iced::widget::{Column, Space, column, row, text};
use iced::{Alignment, Color, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::day_bars::DayBars;
use otty_ui_kit::components::progress_bar::ProgressBar;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{DayBarsViewModel, TotalViewModel};

/// Size of a block title (`h2 { font-size: 14px; line-height: 20px }`).
const TITLE_SIZE: f32 = 14.0;
/// Line height of a block title.
const TITLE_LINE: f32 = 20.0;
/// Size of a total (`font-size: 20px; line-height: 28px`).
const TOTAL_SIZE: f32 = 20.0;
/// Line height of a total.
const TOTAL_LINE: f32 = 28.0;
/// Gap between two totals (`gap: 40px`).
const TOTALS_GAP: f32 = 40.0;

/// A block title, such as "Limits", with a muted note after it, such as
/// where the numbers came from.
pub(super) fn title<'a>(
    title: &'a str,
    note: Option<String>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut line = row![
        text(title)
            .size(TITLE_SIZE)
            .line_height(iced::Pixels(TITLE_LINE))
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong())
    ]
    .spacing(spacing::SPACE_4)
    .align_y(Alignment::End);
    if let Some(note) = note {
        line = line.push(small_text(note, palette.text_muted()));
    }

    line.into()
}

/// The totals side by side: each its label, its number and a muted line.
/// The numbers are muted while they are an older answer.
pub(super) fn totals<'a>(
    totals: Vec<TotalViewModel>,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let value_color = if is_stale {
        palette.text_muted()
    } else {
        palette.text_strong()
    };

    let mut line = row![].spacing(TOTALS_GAP);
    for total in totals {
        line = line.push(column![
            small_text(total.label, palette.text_muted()),
            text(total.value)
                .size(TOTAL_SIZE)
                .line_height(iced::Pixels(TOTAL_LINE))
                .font(fonts::UI_SEMIBOLD)
                .color(value_color),
            small_text(total.note, palette.text_muted()),
        ]);
    }

    line.into()
}

/// One bar per day, with the first day and "Today" under them; muted while
/// they are an older answer.
pub(super) fn day_bars<'a>(
    days: DayBarsViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut bars = DayBars::new(palette).muted(is_stale);
    for bar in days.bars {
        bars = bars.bar(bar.value as f64, bar.tip);
    }

    bars.axis(days.first_day, i18n::t(Key::AgentsToday)).into()
}

/// The lines under a block: what is old, what failed and what to do.
pub(super) fn notes<'a>(
    muted: impl IntoIterator<Item = String>,
    failure: Option<String>,
    palette: &UiPalette,
) -> Column<'a, AgentsIntent> {
    let mut lines = Column::new().spacing(spacing::SPACE_1);
    for line in muted {
        lines = lines.push(small_text(line, palette.text_muted()));
    }
    if let Some(failure) = failure {
        lines = lines.push(small_text(failure, palette.status_error()));
    }

    lines
}

/// A name on the left, such as a window, and its value on the right; muted
/// while the answer is old.
pub(super) fn name_and_value<'a>(
    name: String,
    value: String,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let body = typography::BODY;
    let color = if is_stale {
        palette.text_muted()
    } else {
        palette.text_strong()
    };

    row![
        text(name)
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI_MEDIUM)
            .color(color),
        Space::new().width(Length::Fill),
        text(value)
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI)
            .color(color),
    ]
    .spacing(spacing::SPACE_3)
    .into()
}

/// A bar `percent` full, as wide as the block; muted while the answer is
/// old.
pub(super) fn bar<'a>(
    percent: u8,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    ProgressBar::new(f32::from(percent) / 100.0, palette)
        .fill_width()
        .muted(is_stale)
        .into()
}

/// A small line of text in `color`.
pub(super) fn small_text<'a>(
    content: impl text::IntoFragment<'a>,
    color: Color,
) -> Element<'a, AgentsIntent> {
    let small = typography::SMALL;

    text(content)
        .size(small.size())
        .line_height(iced::Pixels(small.line_height()))
        .font(fonts::UI)
        .color(color)
        .into()
}
