//! The pieces the blocks of the pages and of the statistics dialog share:
//! a block title, alone, with a muted line under it or over its content,
//! the lists of facts and a keyed row in their look, the tables as tall as
//! their rows, the rows of the Limits and Money blocks and small lines.

use iced::widget::{Column, Space, column, container, row, text};
use iced::{Element, Length};
use otty_ui_kit::components::data_table;
use otty_ui_kit::components::key_value_list::{KeyValueList, ValuePart};
use otty_ui_kit::components::progress_bar::ProgressBar;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{FactViewModel, UsageRowViewModel};

/// Gap between the rows of the Usage block (`gap: 14px`).
pub(super) const USAGE_ROW_GAP: f32 = 14.0;
/// Gap between the blocks (`gap: 28px`).
pub(super) const BLOCK_GAP: f32 = 28.0;
/// Gap between a block title and its list or table (`gap: 10px`).
pub(super) const TITLE_GAP: f32 = 10.0;
/// Size of the key of a keyed row, as the keys of the kit `KeyValueList`
/// (`font-size: 12.5px`).
const KEY_SIZE: f32 = 12.5;
/// Size of a block title (`h2 { font-size: 14px; line-height: 20px }`).
const TITLE_SIZE: f32 = 14.0;
/// Line height of a block title.
const TITLE_LINE: f32 = 20.0;

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

/// Each fact as its label and its value; a value that names a plugin
/// opens its row on the Plugins page.
pub(super) fn facts<'a>(
    facts: Vec<FactViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut list = KeyValueList::new(palette);
    for fact in facts {
        list = match fact.plugin {
            Some(plugin) => list.entry_parts(
                fact.label,
                [ValuePart::link(
                    fact.value,
                    AgentsIntent::OpenPlugin(plugin),
                )],
            ),
            None => list.entry(fact.label, fact.value),
        };
    }

    list.into()
}

/// `content`, such as chips, after its muted `key` in the look of a row of
/// the facts, so it lines up with them.
pub(super) fn keyed<'a>(
    key: &'a str,
    content: impl Into<Element<'a, AgentsIntent>>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let key = text(key)
        .size(KEY_SIZE)
        .font(fonts::UI)
        .color(palette.text_muted())
        .width(size::KEY_VALUE_KEY);

    row![key, content.into()].spacing(spacing::SPACE_3).into()
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
