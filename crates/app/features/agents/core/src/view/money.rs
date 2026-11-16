//! The Spending block of an API key profile: the money of today, 7 days and
//! this month, a bar per day, that it is an estimate, and the requests with
//! no price.

use iced::widget::Column;
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::MoneyViewModel;
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
    if let Some(days) = vm.days {
        block = block.push(blocks::day_bars(days, vm.is_stale, palette));
    }

    let muted = vm.unpriced.into_iter().chain(vm.last_read);
    block
        .push(blocks::notes(muted, vm.failure, palette))
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{DayBarViewModel, DayBarsViewModel, TotalViewModel};

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
        ] {
            ui.find(shown).expect("the money should show");
        }
    }
}
