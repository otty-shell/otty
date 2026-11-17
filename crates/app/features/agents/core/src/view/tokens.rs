//! The "Tokens, last 30 days" block of a profile's dashboard: today, 7 and
//! 30 days, and a bar per day.

use iced::widget::Column;
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::TokensViewModel;
use super::blocks;

/// Gap between the parts of the block (`gap: 14px`).
const GAP: f32 = 14.0;

/// Render the tokens block.
pub(super) fn view<'a>(
    vm: TokensViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut block = Column::new()
        .spacing(GAP)
        .push(blocks::title(i18n::t(Key::AgentsTokensTitle), palette));
    if !vm.totals.is_empty() {
        block = block.push(blocks::totals(vm.totals, vm.is_stale, palette));
    }
    if let Some(days) = vm.days {
        block = block.push(blocks::day_bars(days, vm.is_stale, palette));
    }

    block
        .push(blocks::notes(vm.last_read, vm.failure, palette))
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

    #[test]
    fn the_block_shows_the_totals_their_split_and_the_bars() {
        let vm = TokensViewModel {
            totals: vec![TotalViewModel {
                label: i18n::t(Key::AgentsLastThirtyDays),
                value: String::from("31.6M"),
                note: String::from("24.1M in · 0 cached · 7.5M out"),
            }],
            days: Some(DayBarsViewModel {
                bars: vec![DayBarViewModel {
                    value: 1,
                    tip: String::from("2026-11-16: 1 tokens"),
                }],
                first_day: String::from("2026-10-18"),
            }),
            is_stale: true,
            last_read: Some(String::from("Last read 10 min ago")),
            failure: Some(String::from("Could not count: z")),
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, themes.current().ui()));

        for shown in [
            i18n::t(Key::AgentsTokensTitle),
            i18n::t(Key::AgentsLastThirtyDays),
            "31.6M",
            "24.1M in · 0 cached · 7.5M out",
            "2026-10-18",
            i18n::t(Key::AgentsToday),
            "Last read 10 min ago",
            "Could not count: z",
        ] {
            ui.find(shown).expect("the tokens should show");
        }
        assert!(
            ui.find("Counted from the session files").is_err(),
            "the title has no note (owner, issue 26)"
        );
    }
}
