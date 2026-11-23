//! Money on screen. The Money block of an account page: when the money
//! was read, then what the key told and the money spent this month as
//! rows, drawn as the usage windows of a subscription are, all as the
//! provider told them. And the mark "at API prices" under the sums of the
//! tokens of a profile (`mark`; spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3).

use iced::widget::{Column, Space, row};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
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
            used: spent,
            percent: None,
            resets: None,
        };
        block = block.push(blocks::usage_row(row, vm.is_stale, palette));
    }

    block
        .push(blocks::notes(None, vm.failure, palette))
        .width(Length::Fill)
        .into()
}

/// The mark "at API prices" of money of `kind` right under the sums it is
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

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced::widget::container;
    use iced_test::{Simulator, simulator};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::LimitsViewModel;
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
            spent: Some(String::from("$184.75")),
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
            "Could not count: z",
        ] {
            ui.find(shown).expect("the money should show");
        }
        assert!(
            ui.find(i18n::t(Key::AgentsApiPricesMark)).is_err(),
            "a bill has no mark"
        );
    }

    /// Spec 2026-11-23, Decision 3: money at API prices has the mark
    /// under its sums, at the right end, with the tooltip of the time of
    /// the prices.
    #[test]
    fn money_at_api_prices_has_its_mark() {
        let themes = ThemeManager::new();
        let money = estimated_money();
        let palette = themes.current().ui();
        let drawn = mark(money.kind(), palette).expect("the mark draws");

        let mut ui = block(drawn);

        let label = at(&mut ui, i18n::t(Key::AgentsApiPricesMark));
        assert!(
            (label.x + label.width - WIDTH).abs() < 1.0,
            "the mark ends the row: {label:?}"
        );
    }

    #[test]
    fn exact_money_has_no_mark_to_draw() {
        let themes = ThemeManager::new();

        assert!(mark(&MoneyKind::Exact, themes.current().ui()).is_none());
    }

    #[test]
    fn a_row_of_money_is_drawn_as_a_usage_window_is() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let money = MoneyViewModel {
            rows: vec![row("Today", "$12.40", None)],
            spent: None,
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
}
