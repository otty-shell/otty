//! The usage block of a profile page and of the statistics popups (spec
//! 2026-11-19, Decision 6): the title with its line and the period
//! drop-down at its right, the bars in the look of the kit `DayBars`, the
//! tabs of the block and the table of the rows of the tab, their tokens and
//! their money side by side (issue 14 of spec
//! 2026-11-23-accounts-apart-from-profiles). Pressing a row, or the pointer
//! over it, lights its part of every bar. Money at API prices has the mark
//! "at API prices" under the table. On a page the drop-down opens the
//! menu of the mode at the pointer; in a popup, over which no menu shows,
//! its list opens under it.

use iced::widget::{Column, container, mouse_area, row};
use iced::{Alignment, Element, Length, Point};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::{
    ColumnAlign, DataTable, TableCell, TableColumn,
};
use otty_ui_kit::components::day_bars::DayBars;
use otty_ui_kit::components::note_select::{NoteSelect, NoteSelectOption};
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::super::event::AgentsIntent;
use super::super::model::{
    PeriodPicker, UsageBlockViewModel, UsageTableRowViewModel,
};
use super::super::usage::{UsagePeriod, UsageTab};
use super::{blocks, money};

/// Width of the period drop-down at the right of the title.
const DROPDOWN_WIDTH: f32 = 112.0;
/// Least width of the name column.
const NAME_MIN_WIDTH: u16 = 120;
/// Least width of the Tokens column: "999.9K" and its padding.
const TOKENS_MIN_WIDTH: u16 = 72;
/// Least width of the Money column: "$12 345.67" and its padding.
const MONEY_MIN_WIDTH: u16 = 88;
/// Least width of the share column.
const SHARE_MIN_WIDTH: u16 = 56;

/// Render the usage block.
pub(super) fn view<'a>(
    vm: UsageBlockViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut block = Column::new()
        .spacing(blocks::TITLE_GAP)
        .push(title_row(&vm, palette));
    if !vm.bars.is_empty() {
        block = block
            .push(bars(&vm, palette))
            .push(segmented(
                &vm.tabs,
                &vm.tab,
                AgentsIntent::ShowUsageTab,
                palette,
            ))
            .push(table(&vm, palette));
    }
    if let Some(mark) = vm
        .money_kind
        .as_ref()
        .and_then(|kind| money::mark(kind, palette))
    {
        block = block.push(mark);
    }

    block.push(blocks::notes(None, vm.failure, palette)).into()
}

/// The title with the total of the period, or the time of the last good
/// numbers, under it, and the period drop-down at its right.
fn title_row<'a>(
    vm: &UsageBlockViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let title = container(blocks::title_over_line(
        i18n::t(Key::AgentsUsage),
        vm.line.clone(),
        palette,
    ))
    .width(Length::Fill);

    let period = match vm.period_picker {
        PeriodPicker::List { is_open } => {
            period_list(vm.period, is_open, palette)
        },
        PeriodPicker::Menu => period_dropdown(vm.period, palette),
    };

    row![title, period]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .into()
}

/// A compact drop-down showing `period`; a press opens its menu at the
/// pointer.
fn period_dropdown<'a>(
    period: UsagePeriod,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let field = NoteSelect::new(
        vec![NoteSelectOption::new(period.label())],
        Some(0),
        palette,
    )
    .on_toggle(AgentsIntent::OpenPeriodMenu);

    container(field).width(DROPDOWN_WIDTH).into()
}

/// The period drop-down of a popup, `period` picked, whose list of the
/// periods opens under it while `is_open`.
fn period_list<'a>(
    period: UsagePeriod,
    is_open: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let options = UsagePeriod::ALL
        .iter()
        .map(|period| NoteSelectOption::new(period.label()))
        .collect();
    let picked = UsagePeriod::ALL.iter().position(|shown| *shown == period);
    let field = NoteSelect::new(options, picked, palette)
        .open(is_open)
        .on_toggle(AgentsIntent::OpenPeriodMenu)
        .on_pick(|index| {
            let period = UsagePeriod::ALL.get(index).copied();
            AgentsIntent::PickPeriod(period.unwrap_or_default())
        });

    container(field).width(DROPDOWN_WIDTH).into()
}

/// The bars, as tall as their tokens, muted while the numbers are the
/// last good ones.
fn bars<'a>(
    vm: &UsageBlockViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut bars = DayBars::new(palette).muted(vm.is_stale);
    for bar in &vm.bars {
        bars = match bar.lit {
            Some(lit) => bars.bar_with_part(
                bar.value as f64,
                lit as f64,
                bar.tip.clone(),
            ),
            None => bars.bar(bar.value as f64, bar.tip.clone()),
        };
    }
    if let Some(current) = vm.current {
        bars = bars.current(current);
    }
    if let Some((first, last)) = vm.axis.clone() {
        bars = bars.axis(first, last);
    }

    bars.into()
}

/// The table of the rows, as tall as its rows. A press on a row picks it;
/// the pointer over a row lights it.
fn table<'a>(
    vm: &UsageBlockViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let names: Vec<String> =
        vm.rows.iter().map(|row| row.name.clone()).collect();
    let pressed = names.clone();

    let mut table = DataTable::new(columns(vm.tab), palette)
        .selected(vm.picked)
        .empty_message(i18n::t(Key::AgentsTopEmpty))
        .on_row_press(move |index| {
            AgentsIntent::PickUsageRow(
                pressed.get(index).cloned().unwrap_or_default(),
            )
        });
    for row in &vm.rows {
        table = table.row_without_menu(cells(row, vm.tab, vm.is_stale));
    }

    let rows = vm.rows.len().max(1);
    let hovered = mouse_area(blocks::rows_tall(table, rows))
        .on_move(move |point| {
            AgentsIntent::HoverUsageRow(
                row_at(point).and_then(|index| names.get(index).cloned()),
            )
        })
        .on_exit(AgentsIntent::HoverUsageRow(None));

    hovered.into()
}

/// The columns of the table: the name of the rows of `tab`, the tokens,
/// their money and the share, the numbers at the right end of their
/// columns. The four minimums fit the smallest screen, so the table never
/// scrolls sideways inside a page that does.
fn columns(tab: UsageTab) -> Vec<TableColumn> {
    let name = match tab {
        UsageTab::ByProject => Key::AgentsColumnProject,
        UsageTab::ByProfile => Key::AgentsProfile,
        UsageTab::ByModel => Key::AgentsColumnModel,
    };
    let number = |key: Key, min_width: u16| {
        TableColumn::new(i18n::t(key), 1)
            .min_width(min_width)
            .align(ColumnAlign::End)
            .sortable(false)
    };

    vec![
        TableColumn::new(i18n::t(name), 3)
            .min_width(NAME_MIN_WIDTH)
            .sortable(false),
        number(Key::AgentsTokens, TOKENS_MIN_WIDTH),
        number(Key::AgentsNumberMoney, MONEY_MIN_WIDTH),
        number(Key::AgentsColumnShare, SHARE_MIN_WIDTH),
    ]
}

/// The cells of one row: a model in the mono font, the numbers muted
/// while they are the last good ones.
fn cells(
    row: &UsageTableRowViewModel,
    tab: UsageTab,
    is_stale: bool,
) -> Vec<TableCell> {
    let name = match tab {
        UsageTab::ByModel => TableCell::mono(row.name.clone()),
        _ => TableCell::plain(row.name.clone()),
    };
    let number = |value: &str| {
        if is_stale {
            TableCell::muted(value)
        } else {
            TableCell::number(value)
        }
    };

    vec![
        name,
        number(&row.tokens),
        number(&row.money),
        number(&row.share),
    ]
}

/// The index of the row of the table at `point`, measured from the top of
/// the table; `None` over its header.
fn row_at(point: Point) -> Option<usize> {
    let top = size::TABLE_HEADER + size::LINE;
    if point.y < top {
        return None;
    }

    Some(((point.y - top) / (size::TABLE_ROW + size::LINE)) as usize)
}

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced::widget::container;
    use iced_test::{Simulator, simulator};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::MoneyKind;
    use crate::testing::{estimated_money, usage_block};

    /// Width of the block in the tests.
    const WIDTH: f32 = 600.0;

    fn at(ui: &mut Simulator<'_, AgentsIntent>, label: &str) -> Rectangle {
        ui.find(label).expect("the text should show").bounds()
    }

    fn ui<'a>(
        vm: UsageBlockViewModel,
        themes: &'a ThemeManager,
        width: f32,
    ) -> Simulator<'a, AgentsIntent> {
        simulator(container(view(vm, themes.current().ui())).width(width))
    }

    /// Issue 14 of spec 2026-11-23: the table has a Tokens and a Money
    /// column side by side, the money at the right end of its column like
    /// the numbers, and "—" for a model with no price.
    #[test]
    fn the_table_has_tokens_and_money_side_by_side() {
        let themes = ThemeManager::new();
        let mut ui = ui(usage_block(), &themes, WIDTH);

        let tokens = at(&mut ui, i18n::t(Key::AgentsTokens));
        let money = at(&mut ui, i18n::t(Key::AgentsNumberMoney));
        let share = at(&mut ui, i18n::t(Key::AgentsColumnShare));
        let priced = at(&mut ui, "$1.25");
        let unpriced = at(&mut ui, "—");

        assert!((tokens.y - money.y).abs() < 1.0, "{tokens:?} {money:?}");
        assert!(tokens.x + tokens.width < money.x, "Tokens, then Money");
        assert!(money.x + money.width < share.x, "Money, then Share");
        let money_end = money.x + money.width;
        for cell in [priced, unpriced] {
            assert!(
                (cell.x + cell.width - money_end).abs() < 1.0,
                "{cell:?} ends where its title {money:?} does"
            );
        }
    }

    /// Issue 14: no Tokens / Money switch. The block has one drop-down,
    /// the period, and Tokens and Money are the titles of columns of the
    /// table, under the tabs.
    #[test]
    fn there_is_no_measure_switch_on_a_page_or_in_a_popup() {
        let in_popup = UsageBlockViewModel {
            period_picker: PeriodPicker::List { is_open: false },
            ..usage_block()
        };

        for vm in [usage_block(), in_popup] {
            let themes = ThemeManager::new();
            let mut ui = ui(vm, &themes, WIDTH);

            let tabs = at(&mut ui, i18n::t(Key::AgentsTopByModel));
            let period = at(&mut ui, i18n::t(Key::AgentsSevenDays));
            for title in [Key::AgentsTokens, Key::AgentsNumberMoney] {
                let column = at(&mut ui, i18n::t(title));
                assert!(column.y > tabs.y, "{title:?} is under the tabs");
            }
            assert!(period.y < tabs.y, "the period is in the title row");
        }
    }

    /// Spec 2026-11-23, Decision 3: money at API prices carries the mark
    /// "at API prices" under the table, at the right end.
    #[test]
    fn money_at_api_prices_is_marked_under_the_table() {
        let themes = ThemeManager::new();
        let vm = UsageBlockViewModel {
            money_kind: Some(estimated_money().kind().clone()),
            ..usage_block()
        };
        let mut ui = ui(vm, &themes, WIDTH);

        let last_row = at(&mut ui, "gpt-5.4");
        let mark = at(&mut ui, i18n::t(Key::AgentsApiPricesMark));

        assert!(mark.y >= last_row.y + last_row.height, "{mark:?}");
        assert!(
            (mark.x + mark.width - WIDTH).abs() < 1.0,
            "the mark ends the row: {mark:?}"
        );
    }

    /// Spec 2026-11-23, Decision 1: the tabs have no By account.
    #[test]
    fn the_tabs_have_no_by_account() {
        let themes = ThemeManager::new();
        let mut ui = ui(usage_block(), &themes, WIDTH);

        ui.find(i18n::t(Key::AgentsTopByProject))
            .expect("By project shows");
        ui.find(i18n::t(Key::AgentsTopByModel))
            .expect("By model shows");
        assert!(ui.find("By account").is_err());
    }

    /// Money not read yet has no mark.
    #[test]
    fn money_not_read_yet_has_no_mark() {
        let themes = ThemeManager::new();
        let vm = UsageBlockViewModel {
            money_kind: None,
            ..usage_block()
        };
        let mut ui = ui(vm, &themes, WIDTH);

        assert!(ui.find(i18n::t(Key::AgentsApiPricesMark)).is_err());
    }

    /// The cost the agent wrote is exact: the block shows no mark.
    #[test]
    fn exact_money_has_no_mark() {
        let themes = ThemeManager::new();
        let vm = UsageBlockViewModel {
            money_kind: Some(MoneyKind::Exact),
            ..usage_block()
        };
        let mut ui = ui(vm, &themes, WIDTH);

        assert!(ui.find(i18n::t(Key::AgentsApiPricesMark)).is_err());
    }
}
