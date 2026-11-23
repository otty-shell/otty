//! The usage block of a profile page and of the statistics popups (spec
//! 2026-11-19, Decision 6): the title with its line and two drop-downs at
//! its right, the bars in the look of the kit `DayBars`, the tabs of the
//! block and the table of the rows of the tab. Pressing a row, or the
//! pointer over it, lights its part of every bar. Money at the list prices
//! has the mark "estimate, list prices" under the table, as the Money
//! block of an account page has under its sum. On a page a drop-down
//! opens the menu of the mode at the pointer; in a popup, over which no
//! menu shows, its list opens under it.

use iced::widget::{Column, container, mouse_area, row};
use iced::{Alignment, Element, Length, Point};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
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
    AccountLabel, UsageBlockViewModel, UsageListsViewModel,
    UsageTableRowViewModel,
};
use super::super::usage::{UsageList, UsageMeasure, UsagePeriod, UsageTab};
use super::{blocks, money};

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
    if !vm.bars.is_empty() {
        block = block.push(bars(&vm, palette)).push(tabs(&vm, palette));
        if let Some(reason) = &vm.account_reason {
            let line =
                i18n::filled(Key::TplAgentsByAccountOff, &[("reason", reason)]);
            block = block.push(small_text(line, palette.text_muted()));
        }
        block = block.push(table(&vm, palette));
    }
    if let Some(mark) = vm
        .money_kind
        .as_ref()
        .and_then(|kind| money::mark(kind, palette))
    {
        block = block.push(mark);
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
/// under it. Money while it is off can not be picked and says why on
/// hover.
fn measure_list<'a>(
    vm: &UsageBlockViewModel,
    lists: &UsageListsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let options = UsageMeasure::ALL
        .iter()
        .map(|measure| {
            let option = NoteSelectOption::new(measure.label());
            match (*measure, &lists.money_reason) {
                (UsageMeasure::Money, Some(reason)) => {
                    option.disabled_reason(reason.as_str())
                },
                _ => option,
            }
        })
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
    let name = match (tab, &row.account) {
        (UsageTab::ByModel, _) => TableCell::mono(row.name.clone()),
        (_, Some(account)) => account_cell(account),
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

/// The name cell of a row of By account, as the chips of an account show
/// it: the provider's logo and the short name, the whole name in the
/// tooltip. The table draws only SVG icons, so a provider with none shows
/// the icon of Agents mode.
fn account_cell(account: &AccountLabel) -> TableCell {
    TableCell::icon_text(
        account.icon.svg_or(icons::MODE_AGENTS),
        account.name.clone(),
    )
    .with_tooltip(account.full_name.clone())
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

    /// Owner, 2026-11-21: one mark everywhere. Money at the list prices
    /// carries the mark of the account page under the sums of the table,
    /// at the right end, and no note of an estimate.
    #[test]
    fn an_estimate_is_marked_under_the_table_of_its_sums() {
        let themes = ThemeManager::new();
        let vm = UsageBlockViewModel {
            measure: UsageMeasure::Money,
            money_kind: Some(estimated_money().kind().clone()),
            ..usage_block()
        };
        let mut ui =
            simulator(container(view(vm, themes.current().ui())).width(WIDTH));

        let last_row = at(&mut ui, "gpt-5.4");
        let mark = at(&mut ui, i18n::t(Key::AgentsEstimateMark));

        assert!(mark.y >= last_row.y + last_row.height, "{mark:?}");
        assert!(
            (mark.x + mark.width - WIDTH).abs() < 1.0,
            "the mark ends the row: {mark:?}"
        );
    }

    /// Owner, 2026-11-23: a row of By account names its account by the
    /// login alone, as its chips do; the whole name is its tooltip.
    #[test]
    fn a_row_by_account_shows_the_login_and_the_whole_name_on_hover() {
        let themes = ThemeManager::new();
        let row = UsageTableRowViewModel {
            name: String::from("Anthropic · dev@example.com"),
            account: Some(crate::model::AccountLabel {
                icon: crate::testing::provider_icon("anthropic"),
                name: String::from("dev@example.com"),
                full_name: String::from("Anthropic · dev@example.com"),
            }),
            value: String::from("50"),
            share: String::from("100 %"),
        };
        let vm = UsageBlockViewModel {
            tab: UsageTab::ByAccount,
            rows: vec![row],
            ..usage_block()
        };
        let mut ui =
            simulator(container(view(vm, themes.current().ui())).width(WIDTH));

        ui.find("dev@example.com").expect("the login shows");
        assert!(ui.find("Anthropic · dev@example.com").is_err());
    }

    #[test]
    fn tokens_have_no_mark() {
        let themes = ThemeManager::new();
        let mut ui = simulator(
            container(view(usage_block(), themes.current().ui())).width(WIDTH),
        );

        assert!(ui.find(i18n::t(Key::AgentsEstimateMark)).is_err());
    }

    /// The cost the agent wrote is exact: the block shows no mark.
    #[test]
    fn exact_money_has_no_mark() {
        let themes = ThemeManager::new();
        let vm = UsageBlockViewModel {
            measure: UsageMeasure::Money,
            money_kind: Some(MoneyKind::Exact),
            ..usage_block()
        };
        let mut ui =
            simulator(container(view(vm, themes.current().ui())).width(WIDTH));

        assert!(ui.find(i18n::t(Key::AgentsEstimateMark)).is_err());
    }
}
