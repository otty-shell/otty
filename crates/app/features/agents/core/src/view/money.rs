//! Money on screen. The Money block of an account page: when the money
//! was read, then what the key told and the money spent this month as
//! rows, drawn as the usage windows of a subscription are, and the
//! requests with no price. And the two forms every shown sum is drawn in
//! (owner, 2026-11-21): the full form puts the mark of an estimate under
//! the sums (`mark`); the tight form, whose text is
//! `ShownMoney::short_text`, shows the tooltip of the mark over the place
//! that holds it (`with_tip`, `cell_with_tip`).

use iced::widget::{Column, Space, row};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::TableCell;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tooltip::tooltip;
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::{MoneyKind, MoneyViewModel, UsageRowViewModel};
use super::blocks;

/// Render the block of money named `title`, "Money" on the page of an
/// account.
pub(super) fn view<'a>(
    vm: MoneyViewModel,
    title: &'static str,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut block = Column::new()
        .spacing(blocks::USAGE_ROW_GAP)
        .push(blocks::title_over_line(title, vm.updated, palette));
    for row in vm.rows {
        block = block.push(blocks::usage_row(row, vm.is_stale, palette));
    }
    if let Some(spent) = vm.spent {
        let row = UsageRowViewModel {
            name: i18n::t(Key::AgentsSpentThisMonth).to_string(),
            used: spent.amount().to_string(),
            percent: None,
            resets: None,
        };
        block = block.push(blocks::usage_row(row, vm.is_stale, palette));
        if let Some(mark) = mark(spent.kind(), palette) {
            block = block.push(mark);
        }
    }

    block
        .push(blocks::notes(vm.note, vm.failure, palette))
        .width(Length::Fill)
        .into()
}

/// The full form: the mark of money of `kind` right under the sums it is
/// about, at the right end of the block, with its tooltip; `None` for
/// exact money, which has no mark.
pub(super) fn mark<'a>(
    kind: &MoneyKind,
    palette: &UiPalette,
) -> Option<Element<'a, AgentsIntent>> {
    let mark = kind.mark()?;
    let label = small_text(mark.label, palette.text_muted());

    Some(
        row![
            Space::new().width(Length::Fill),
            tooltip(label, mark.tip.clone(), palette)
        ]
        .into(),
    )
}

/// The tight form over an element, such as a row of the left panel or a
/// chip: `content` shows `tip`, the tooltip of the mark of an estimate,
/// under the pointer; with no tip it is drawn as it is.
pub(super) fn with_tip<'a>(
    content: impl Into<Element<'a, AgentsIntent>>,
    tip: Option<&str>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    match tip {
        Some(tip) => tooltip(content, tip, palette),
        None => content.into(),
    }
}

/// The tight form in a table: `cell` shows `tip`, the tooltip of the mark
/// of an estimate, under the pointer.
pub(super) fn cell_with_tip(cell: TableCell, tip: Option<&str>) -> TableCell {
    match tip {
        Some(tip) => cell.with_tooltip(tip),
        None => cell,
    }
}

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced::widget::{container, text};
    use iced_test::{Simulator, simulator};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{LimitsViewModel, ShownMoney};
    use crate::testing::estimated_money;

    /// Width of the block in the tests.
    const WIDTH: f32 = 400.0;

    fn row(name: &str, used: &str, percent: Option<u8>) -> UsageRowViewModel {
        UsageRowViewModel {
            name: String::from(name),
            used: String::from(used),
            percent,
            resets: None,
        }
    }

    fn money() -> MoneyViewModel {
        MoneyViewModel {
            updated: Some(String::from("Updated 2 min ago")),
            rows: vec![row(i18n::t(Key::AgentsLeft), "$110.00", None)],
            spent: Some(ShownMoney::exact(String::from("$184.75"))),
            note: Some(String::from("3 requests have no price")),
            is_stale: false,
            failure: Some(String::from("Could not count: z")),
        }
    }

    fn block<'a>(
        content: Element<'a, AgentsIntent>,
    ) -> Simulator<'a, AgentsIntent> {
        simulator(container(content).width(WIDTH))
    }

    fn at(ui: &mut Simulator<'_, AgentsIntent>, label: &str) -> Rectangle {
        ui.find(label).expect("the text should show").bounds()
    }

    #[test]
    fn the_block_shows_each_row_of_money_and_what_failed() {
        let themes = ThemeManager::new();
        let mut ui = block(view(
            money(),
            i18n::t(Key::AgentsNumberMoney),
            themes.current().ui(),
        ));

        for shown in [
            i18n::t(Key::AgentsNumberMoney),
            "Updated 2 min ago",
            i18n::t(Key::AgentsLeft),
            "$110.00",
            i18n::t(Key::AgentsSpentThisMonth),
            "$184.75",
            "3 requests have no price",
            "Could not count: z",
        ] {
            ui.find(shown).expect("the money should show");
        }
        assert!(
            ui.find(i18n::t(Key::AgentsEstimateMark)).is_err(),
            "a bill has no mark"
        );
    }

    /// Owner, 2026-11-21: the block only hands over the shown sum; an
    /// estimate brings its mark right under it by itself.
    #[test]
    fn an_estimate_is_marked_under_its_sum() {
        let themes = ThemeManager::new();
        let money = MoneyViewModel {
            spent: Some(estimated_money()),
            ..money()
        };
        let mut ui = block(view(
            money,
            i18n::t(Key::AgentsNumberMoney),
            themes.current().ui(),
        ));

        let sum = at(&mut ui, "$1.25");
        let mark = at(&mut ui, i18n::t(Key::AgentsEstimateMark));

        assert!(mark.y >= sum.y + sum.height, "{mark:?} under {sum:?}");
        assert!(
            (mark.x + mark.width - WIDTH).abs() < 1.0,
            "the mark ends the row: {mark:?}"
        );
    }

    #[test]
    fn exact_money_has_no_mark_to_draw() {
        let themes = ThemeManager::new();

        assert!(mark(&MoneyKind::Exact, themes.current().ui()).is_none());
    }

    /// The tight form: the place draws the short text and hands over the
    /// tip; an estimate's tooltip comes with it, exact money draws as it
    /// is.
    #[test]
    fn the_tight_form_shows_the_short_text_with_or_without_a_tip() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let estimate = estimated_money();
        let bill = ShownMoney::exact(String::from("$42.00"));

        let mut estimate_ui = block(with_tip(
            text(estimate.short_text()),
            estimate.tip(),
            palette,
        ));
        let mut bill_ui =
            block(with_tip(text(bill.short_text()), bill.tip(), palette));

        estimate_ui
            .find(estimate.short_text().as_str())
            .expect("the sum with its mark shows");
        bill_ui.find("$42.00").expect("the bill shows");
    }

    #[test]
    fn a_row_of_money_is_drawn_as_a_usage_window_is() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let money = MoneyViewModel {
            rows: vec![row("Today", "$12.40", None)],
            spent: None,
            note: None,
            failure: None,
            ..money()
        };
        let limits = LimitsViewModel {
            updated: Some(String::from("Updated 2 min ago")),
            windows: vec![row("Today", "$12.40", None)],
            credits: None,
            is_stale: false,
            recorded: None,
            failure: None,
        };
        let mut money_ui =
            block(view(money, i18n::t(Key::AgentsUsage), palette));
        let mut limits_ui = block(super::super::limits::view(
            limits,
            i18n::t(Key::AgentsUsage),
            palette,
        ));

        for text in ["Today", "$12.40"] {
            assert_eq!(at(&mut money_ui, text), at(&mut limits_ui, text));
        }
        let value = at(&mut money_ui, "$12.40");
        assert!(
            (value.x + value.width - WIDTH).abs() < 1.0,
            "the value ends the row: {value:?}"
        );
    }

    #[test]
    fn the_requests_with_no_price_are_the_last_row() {
        let themes = ThemeManager::new();
        let mut ui = block(view(
            money(),
            i18n::t(Key::AgentsNumberMoney),
            themes.current().ui(),
        ));

        let month = at(&mut ui, "$184.75");
        let unpriced = at(&mut ui, "3 requests have no price");
        let title = at(&mut ui, i18n::t(Key::AgentsNumberMoney));
        let updated = at(&mut ui, "Updated 2 min ago");

        assert!(unpriced.y > month.y, "{unpriced:?} under {month:?}");
        assert!(updated.y >= title.y + title.height, "{updated:?}");
    }

    /// Owner, issue 37: with no prices the amount is a dash, and one muted
    /// line under the rows says why.
    #[test]
    fn with_no_prices_the_sum_is_a_dash_and_the_line_is_under_it() {
        let themes = ThemeManager::new();
        let no_prices = i18n::t(Key::AgentsNoPrices);
        let money = MoneyViewModel {
            rows: Vec::new(),
            spent: Some(ShownMoney::exact(String::from("—"))),
            note: Some(no_prices.to_string()),
            failure: None,
            ..money()
        };
        let mut ui = block(view(
            money,
            i18n::t(Key::AgentsNumberMoney),
            themes.current().ui(),
        ));

        let month = at(&mut ui, i18n::t(Key::AgentsSpentThisMonth));
        let line = at(&mut ui, no_prices);

        assert!(line.y > month.y, "{line:?} under {month:?}");
        assert!(
            ui.find("$0.00").is_err(),
            "no amount reads as nothing spent"
        );
    }
}
