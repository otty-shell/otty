//! The Usage block of an API key profile: when the money was read, then
//! the money of today, 7 days and this month and the budget as rows, drawn
//! as the usage windows of a subscription are, and the requests with no
//! price.

use iced::widget::Column;
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::MoneyViewModel;
use super::blocks;

/// Render the Usage block of an API key profile.
pub(super) fn view<'a>(
    vm: MoneyViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut block = Column::new().spacing(blocks::USAGE_ROW_GAP).push(
        blocks::title_over_line(i18n::t(Key::AgentsUsage), vm.updated, palette),
    );
    for row in vm.rows {
        block = block.push(blocks::usage_row(row, vm.is_stale, palette));
    }

    block
        .push(blocks::notes(vm.note, vm.failure, palette))
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced::widget::container;
    use iced_test::{Simulator, simulator};
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{LimitsViewModel, UsageRowViewModel};

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
            rows: vec![
                row(i18n::t(Key::AgentsToday), "$12.40", None),
                row(i18n::t(Key::AgentsLastSevenDays), "$63.10", None),
                row(i18n::t(Key::AgentsThisMonth), "$184.75", None),
                row(
                    i18n::t(Key::AgentsBudgetTitle),
                    "$184.75 of $300.00",
                    Some(62),
                ),
            ],
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
        let mut ui = block(view(money(), themes.current().ui()));

        for shown in [
            i18n::t(Key::AgentsUsage),
            "Updated 2 min ago",
            i18n::t(Key::AgentsToday),
            "$12.40",
            i18n::t(Key::AgentsLastSevenDays),
            "$63.10",
            i18n::t(Key::AgentsThisMonth),
            "$184.75",
            i18n::t(Key::AgentsBudgetTitle),
            "$184.75 of $300.00",
            "3 requests have no price",
            "Could not count: z",
        ] {
            ui.find(shown).expect("the money should show");
        }
    }

    #[test]
    fn a_row_of_money_is_drawn_as_a_usage_window_is() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let money = MoneyViewModel {
            rows: vec![row("Today", "$12.40", None)],
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
            log_in_hint: None,
        };
        let mut money_ui = block(view(money, palette));
        let mut limits_ui = block(super::super::limits::view(limits, palette));

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
        let mut ui = block(view(money(), themes.current().ui()));

        let budget = at(&mut ui, "$184.75 of $300.00");
        let unpriced = at(&mut ui, "3 requests have no price");
        let title = at(&mut ui, i18n::t(Key::AgentsUsage));
        let updated = at(&mut ui, "Updated 2 min ago");

        assert!(unpriced.y > budget.y, "{unpriced:?} under {budget:?}");
        assert!(updated.y >= title.y + title.height, "{updated:?}");
    }

    /// Owner, issue 37: with no prices each amount is a dash, and one
    /// muted line under the rows says why.
    #[test]
    fn with_no_prices_the_rows_show_dashes_and_the_line_is_under_them() {
        let themes = ThemeManager::new();
        let no_prices = i18n::t(Key::AgentsNoPrices);
        let money = MoneyViewModel {
            rows: vec![
                row(i18n::t(Key::AgentsToday), "—", None),
                row(i18n::t(Key::AgentsThisMonth), "—", None),
            ],
            note: Some(no_prices.to_string()),
            failure: None,
            ..money()
        };
        let mut ui = block(view(money, themes.current().ui()));

        let month = at(&mut ui, i18n::t(Key::AgentsThisMonth));
        let line = at(&mut ui, no_prices);

        assert!(line.y > month.y, "{line:?} under {month:?}");
        assert!(
            ui.find("$0.00").is_err(),
            "no amount reads as nothing spent"
        );
    }
}
