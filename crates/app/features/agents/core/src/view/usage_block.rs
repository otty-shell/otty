//! The usage block of a profile page and of the statistics popups (spec
//! 2026-11-19, Decision 6): the title with its line and two drop-downs at
//! its right, the bars in the look of the kit `DayBars`, the tabs of the
//! block and the table of the rows of the tab. Pressing a row, or the
//! pointer over it, lights its part of every bar. On a page a drop-down
//! opens the menu of the mode at the pointer; in a popup, over which no
//! menu shows, its list opens under it.

use iced::widget::{Column, container, mouse_area, row};
use iced::{Alignment, Element, Length, Point};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::{
    ColumnAlign, DataTable, TableCell, TableColumn,
};
use otty_ui_kit::components::day_bars::DayBars;
use otty_ui_kit::components::note_select::{NoteSelect, NoteSelectOption};
use otty_ui_kit::components::segmented::segmented_with_disabled;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::super::event::AgentsIntent;
use super::super::model::{
    UsageBlockViewModel, UsageListsViewModel, UsageTableRowViewModel,
};
use super::super::usage::{UsageList, UsageMeasure, UsagePeriod, UsageTab};
use super::blocks;

/// Width of each drop-down at the right of the title.
const DROPDOWN_WIDTH: f32 = 112.0;
/// Least width of the name column.
const NAME_MIN_WIDTH: u16 = 120;
/// Least width of the value column.
const VALUE_MIN_WIDTH: u16 = 80;
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
    let money_reason = vm
        .lists
        .as_ref()
        .and_then(|lists| lists.money_reason.as_deref());
    if let Some(reason) = money_reason {
        let line = i18n::filled(Key::TplAgentsMoneyOff, &[("reason", reason)]);
        block = block.push(small_text(line, palette.text_muted()));
    }

    if !vm.bars.is_empty() {
        block = block.push(bars(&vm, palette)).push(tabs(&vm, palette));
        if let Some(reason) = &vm.account_reason {
            let line =
                i18n::filled(Key::TplAgentsByAccountOff, &[("reason", reason)]);
            block = block.push(small_text(line, palette.text_muted()));
        }
        block = block.push(table(&vm, palette));
    }

    block
        .push(blocks::notes(vm.notes, vm.failure, palette))
        .into()
}

/// The title with the total of the period, or the time of the last good
/// numbers, under it, and the period and measure drop-downs at its right.
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

    let (period, measure) = match &vm.lists {
        Some(lists) => (
            period_list(vm, lists, palette),
            measure_list(vm, lists, palette),
        ),
        None => (
            dropdown(vm.period.label(), AgentsIntent::OpenPeriodMenu, palette),
            dropdown(
                vm.measure.label(),
                AgentsIntent::OpenMeasureMenu,
                palette,
            ),
        ),
    };

    row![title, period, measure]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .into()
}

/// A compact drop-down showing `label`; a press opens its menu at the
/// pointer.
fn dropdown<'a>(
    label: &str,
    on_press: AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let field =
        NoteSelect::new(vec![NoteSelectOption::new(label)], Some(0), palette)
            .on_toggle(on_press);

    container(field).width(DROPDOWN_WIDTH).into()
}

/// The period drop-down of a popup, whose list of the periods opens under
/// it.
fn period_list<'a>(
    vm: &UsageBlockViewModel,
    lists: &UsageListsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let options = UsagePeriod::ALL
        .iter()
        .map(|period| NoteSelectOption::new(period.label()))
        .collect();
    let picked = UsagePeriod::ALL
        .iter()
        .position(|period| *period == vm.period);
    let field = NoteSelect::new(options, picked, palette)
        .open(lists.open == Some(UsageList::Period))
        .on_toggle(AgentsIntent::OpenPeriodMenu)
        .on_pick(|index| {
            let period = UsagePeriod::ALL.get(index).copied();
            AgentsIntent::PickPeriod(period.unwrap_or_default())
        });

    container(field).width(DROPDOWN_WIDTH).into()
}

/// The measure drop-down of a popup, whose list of the measures opens
/// under it. Picking Money while it is off changes nothing: the reason
/// shows under the title.
fn measure_list<'a>(
    vm: &UsageBlockViewModel,
    lists: &UsageListsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let options = UsageMeasure::ALL
        .iter()
        .map(|measure| NoteSelectOption::new(measure.label()))
        .collect();
    let picked = UsageMeasure::ALL
        .iter()
        .position(|measure| *measure == vm.measure);
    let field = NoteSelect::new(options, picked, palette)
        .open(lists.open == Some(UsageList::Measure))
        .on_toggle(AgentsIntent::OpenMeasureMenu)
        .on_pick(|index| {
            let measure = UsageMeasure::ALL.get(index).copied();
            AgentsIntent::PickMeasure(measure.unwrap_or_default())
        });

    container(field).width(DROPDOWN_WIDTH).into()
}

/// The bars, muted while the numbers are the last good ones.
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

/// The tabs; By account is off while the agent does not tell the accounts
/// apart.
fn tabs<'a>(
    vm: &UsageBlockViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let disabled: &[UsageTab] = if vm.account_reason.is_some() {
        &[UsageTab::ByAccount]
    } else {
        &[]
    };

    segmented_with_disabled(
        &vm.tabs,
        disabled,
        &vm.tab,
        AgentsIntent::ShowUsageTab,
        palette,
    )
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

    let mut table = DataTable::new(columns(vm.tab, vm.measure), palette)
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

/// The columns of the table: the name of the rows of `tab`, the value in
/// `measure` and the share.
fn columns(tab: UsageTab, measure: UsageMeasure) -> Vec<TableColumn> {
    let name = match tab {
        UsageTab::ByProject => Key::AgentsColumnProject,
        UsageTab::ByProfile => Key::AgentsProfile,
        UsageTab::ByModel => Key::AgentsColumnModel,
        UsageTab::ByAccount => Key::AgentsFieldAccount,
    };

    vec![
        TableColumn::new(i18n::t(name), 3)
            .min_width(NAME_MIN_WIDTH)
            .sortable(false),
        TableColumn::new(measure.label(), 1)
            .min_width(VALUE_MIN_WIDTH)
            .align(ColumnAlign::End)
            .sortable(false),
        TableColumn::new(i18n::t(Key::AgentsColumnShare), 1)
            .min_width(SHARE_MIN_WIDTH)
            .align(ColumnAlign::End)
            .sortable(false),
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

    vec![name, number(&row.value), number(&row.share)]
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
