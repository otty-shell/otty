//! The pieces the blocks of the pages and of the statistics dialog share:
//! a block title, alone or over its content, the data cards of the pages,
//! the lists of facts and a keyed row in their look, the tables as tall as
//! their rows, the figures of the Limits, Credits and Money cards and
//! small lines.

use iced::widget::text::Wrapping;
use iced::widget::{Column, column, container, row, text};
use iced::{Alignment, Element, Length};
use otty_ui_kit::components::card::{self, Card, CardTone};
use otty_ui_kit::components::data_table;
use otty_ui_kit::components::key_value_list::{KeyValueList, ValuePart};
use otty_ui_kit::components::progress_bar::ProgressBar;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{FactViewModel, UsageRowViewModel};

/// Gap between the cards of a page (`gap: 16px`).
pub(super) const BLOCK_GAP: f32 = spacing::SPACE_5;
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
/// opens its page in Plugins mode.
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

/// A data card (issue 24 of spec 2026-11-23-accounts-apart-from-profiles):
/// the kit card titled `title`, with the muted `line` under the title,
/// such as when its numbers were read, and `body`.
pub(super) fn card<'a>(
    title: &'a str,
    line: Option<String>,
    body: impl Into<Element<'a, AgentsIntent>>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut card = Card::new(body, palette).title(title);
    if let Some(line) = line {
        card = card.line(small_text(line, palette.text_muted()));
    }

    card.into()
}

/// One figure of a usage window or of money, the same for every billing:
/// its name and when it resets, how much is used, and the bar when it has
/// a limit.
pub(super) fn usage_figure<'a>(
    row: UsageRowViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let value = figure_value(row.used, is_stale, palette);

    figure(
        row.name,
        row.resets.into_iter().collect(),
        value,
        row.percent,
        is_stale,
        palette,
    )
}

/// One figure of a data card (issue 24): its name with its own muted
/// lines under it at the left, `value` at the right, and a bar `percent`
/// full under them when it has a limit; the name muted while the answer
/// is old.
pub(super) fn figure<'a>(
    name: String,
    explain: Vec<String>,
    value: impl Into<Element<'a, AgentsIntent>>,
    percent: Option<u8>,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let body = typography::BODY;
    let color = if is_stale {
        palette.text_muted()
    } else {
        palette.text_strong()
    };

    let mut label = column![
        text(name)
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI_MEDIUM)
            .color(color)
    ];
    for line in explain {
        label = label.push(small_text(line, palette.text_muted()));
    }
    let line = row![container(label).width(Length::Fill), value.into()]
        .spacing(spacing::SPACE_5)
        .align_y(Alignment::Start);

    let mut figure = column![line].spacing(spacing::SPACE_3);
    if let Some(percent) = percent {
        figure = figure.push(bar(percent, is_stale, palette));
    }

    figure.into()
}

/// The value of a figure, such as "42 % used", on one line; muted while
/// the answer is old or the value is not known.
pub(super) fn figure_value<'a>(
    value: String,
    is_muted: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let body = typography::BODY;
    let color = if is_muted {
        palette.text_muted()
    } else {
        palette.text_strong()
    };

    text(value)
        .size(body.size())
        .line_height(iced::Pixels(body.line_height()))
        .font(fonts::UI_MEDIUM)
        .color(color)
        .wrapping(Wrapping::None)
        .into()
}

/// The figures of a data card, one under the other, split by thin rules.
pub(super) fn figures<'a>(
    figures: Vec<Element<'a, AgentsIntent>>,
    palette: &UiPalette,
) -> Column<'a, AgentsIntent> {
    let mut rows = Column::new().spacing(spacing::SPACE_4);
    for (index, figure) in figures.into_iter().enumerate() {
        if index > 0 {
            rows = rows.push(card::rule(CardTone::Default, palette));
        }
        rows = rows.push(figure);
    }

    rows
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
