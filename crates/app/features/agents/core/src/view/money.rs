//! The Spending block of an API key profile: the money of today, 7 days and
//! this month, the budget bar, a bar per day, where the money comes from,
//! and the requests with no price.

use iced::widget::{Column, column};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::spacing;

use super::super::event::AgentsIntent;
use super::super::model::{BudgetViewModel, MoneyViewModel};
use super::blocks;

/// Gap between the parts of the block (`gap: 14px`).
const GAP: f32 = 14.0;

/// Render the Spending block.
pub(super) fn view<'a>(
    vm: MoneyViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut block = Column::new().spacing(GAP).push(blocks::title(
        i18n::t(Key::AgentsSpending),
        vm.note,
        palette,
    ));
    if !vm.totals.is_empty() {
        block = block.push(blocks::totals(vm.totals, vm.is_stale, palette));
    }
    if let Some(budget) = vm.budget {
        block = block.push(budget_bar(budget, vm.is_stale, palette));
    }
    if let Some(days) = vm.days {
        block = block.push(blocks::day_bars(days, vm.is_stale, palette));
    }

    let muted = vm.unpriced.into_iter().chain(vm.last_read);
    block
        .push(blocks::notes(muted, vm.failure, palette))
        .width(Length::Fill)
        .into()
}

/// The money of this month against the budget: the label and the amounts,
/// the bar, and that the budget only shows.
fn budget_bar<'a>(
    budget: BudgetViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    column![
        blocks::name_and_value(
            i18n::t(Key::AgentsBudgetTitle).to_string(),
            budget.spent_of,
            is_stale,
            palette,
        ),
        blocks::bar(budget.percent, is_stale, palette),
        blocks::small_text(
            i18n::t(Key::AgentsBudgetNote),
            palette.text_muted()
        ),
    ]
    .spacing(spacing::SPACE_2)
    .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{
        BudgetViewModel, DayBarViewModel, DayBarsViewModel, TotalViewModel,
    };

    fn money() -> MoneyViewModel {
        MoneyViewModel {
            note: Some(String::from("An estimate at the prices of 2026-11-16")),
            totals: vec![
                TotalViewModel {
                    label: i18n::t(Key::AgentsToday),
                    value: String::from("$12.40"),
                    note: String::from("2.1M tokens"),
                },
                TotalViewModel {
                    label: i18n::t(Key::AgentsThisMonth),
                    value: String::from("$184.75"),
                    note: String::from("30.2M tokens"),
                },
            ],
            days: Some(DayBarsViewModel {
                bars: vec![
                    DayBarViewModel {
                        value: 3_100_000,
                        tip: String::from("2026-11-15: $3.10"),
                    },
                    DayBarViewModel {
                        value: 12_400_000,
                        tip: String::from("2026-11-16: $12.40"),
                    },
                ],
                first_day: String::from("2026-11-15"),
            }),
            budget: Some(BudgetViewModel {
                spent_of: String::from("$184.75 of $300.00"),
                percent: 62,
            }),
            unpriced: Some(String::from("3 requests have no price")),
            is_stale: false,
            last_read: None,
            failure: Some(String::from("Could not count: z")),
        }
    }

    #[test]
    fn the_block_shows_the_money_the_estimate_and_the_bars() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(money(), themes.current().ui()));

        for shown in [
            i18n::t(Key::AgentsSpending),
            "An estimate at the prices of 2026-11-16",
            i18n::t(Key::AgentsToday),
            "$12.40",
            "2.1M tokens",
            i18n::t(Key::AgentsThisMonth),
            "$184.75",
            "2026-11-15",
            "3 requests have no price",
            "Could not count: z",
            i18n::t(Key::AgentsBudgetTitle),
            "$184.75 of $300.00",
            i18n::t(Key::AgentsBudgetNote),
        ] {
            ui.find(shown).expect("the money should show");
        }
    }
}
