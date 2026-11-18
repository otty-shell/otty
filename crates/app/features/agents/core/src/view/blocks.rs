//! The pieces the blocks of a profile's dashboard and of the statistics
//! dialog share: a block title, alone, with a muted line under it or over
//! its content, the lists of facts, the tables as tall as their rows and their columns, the
//! rows of the Usage block, the row of totals, the bars per day and small
//! lines.

use iced::widget::{Column, Space, column, container, row, text};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::{self, ColumnAlign, TableColumn};
use otty_ui_kit::components::day_bars::DayBars;
use otty_ui_kit::components::key_value_list::KeyValueList;
use otty_ui_kit::components::progress_bar::ProgressBar;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    DayBarsViewModel, FactViewModel, TotalViewModel, UsageRowViewModel,
};

/// Gap between the rows of the Usage block (`gap: 14px`).
pub(super) const USAGE_ROW_GAP: f32 = 14.0;
/// Gap between the blocks (`gap: 28px`).
pub(super) const BLOCK_GAP: f32 = 28.0;
/// Gap between a block title and its list or table (`gap: 10px`).
pub(super) const TITLE_GAP: f32 = 10.0;
/// Least width of the name column of a table.
const NAME_MIN_WIDTH: u16 = 120;
/// Least width of a number column of a table.
const NUMBER_MIN_WIDTH: u16 = 64;
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
/// Gap between two lines of totals, when they do not fit on one.
const TOTALS_LINE_GAP: f32 = spacing::SPACE_4;

/// A block title, such as "Tokens, last 30 days".
pub(super) fn title<'a>(
    title: &'a str,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    text(title)
        .size(TITLE_SIZE)
        .line_height(iced::Pixels(TITLE_LINE))
        .font(fonts::UI_SEMIBOLD)
        .color(palette.text_strong())
        .into()
}

/// A block title, such as "Usage", with a muted line under it, such as
/// when its numbers were read.
pub(super) fn title_over_line<'a>(
    heading: &'a str,
    line: Option<String>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut lines = column![title(heading, palette)].spacing(spacing::SPACE_1);
    if let Some(line) = line {
        lines = lines.push(small_text(line, palette.text_muted()));
    }

    lines.into()
}

/// `content` under the block title `heading`.
pub(super) fn titled<'a>(
    heading: &'a str,
    content: impl Into<Element<'a, AgentsIntent>>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    column![title(heading, palette), content.into()]
        .spacing(TITLE_GAP)
        .into()
}

/// Each fact as its label and its value.
pub(super) fn facts<'a>(
    facts: Vec<FactViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut list = KeyValueList::new(palette);
    for fact in facts {
        list = list.entry(fact.label, fact.value);
    }

    list.into()
}

/// `content`, such as a table, as wide as the block and as tall as a
/// table with its header and `rows` rows, so the table never scrolls.
pub(super) fn rows_tall<'a>(
    content: impl Into<Element<'a, AgentsIntent>>,
    rows: usize,
) -> Element<'a, AgentsIntent> {
    container(content)
        .width(Length::Fill)
        .height(data_table::height_for_rows(rows))
        .into()
}

/// The first column of a table, named `key`: the project, the model or the
/// profile.
pub(super) fn name_column(key: Key) -> TableColumn {
    TableColumn::new(i18n::t(key), 3)
        .min_width(NAME_MIN_WIDTH)
        .sortable(false)
}

/// A column of numbers named `key`, at the right of the table.
pub(super) fn number_column(key: Key) -> TableColumn {
    TableColumn::new(i18n::t(key), 1)
        .min_width(NUMBER_MIN_WIDTH)
        .align(ColumnAlign::End)
        .sortable(false)
}

/// The totals side by side: each its label, its number and a muted line.
/// The totals that do not fit go whole to the next line. The numbers are
/// muted while they are an older answer.
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

    line.wrap().vertical_spacing(TOTALS_LINE_GAP).into()
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

/// One row of the Usage block, the same for every billing: its name and
/// how much is used, the bar when it has a limit, and when it resets.
pub(super) fn usage_row<'a>(
    row: UsageRowViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut rows =
        column![name_and_value(row.name, row.used, is_stale, palette)]
            .spacing(spacing::SPACE_2);
    if let Some(percent) = row.percent {
        rows = rows.push(bar(percent, is_stale, palette));
    }
    if let Some(resets) = row.resets {
        rows = rows.push(small_text(resets, palette.text_muted()));
    }

    rows.into()
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
