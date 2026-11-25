//! Money on screen. The Money card of an account page: when the money
//! was read, then what the key told, the money spent this month and the
//! bill of an organization as figures, drawn as the usage windows of a
//! subscription are, all as the provider told them. And the mark "at API
//! prices" under the sums of the tokens of a profile (`mark`; spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3).

use iced::widget::{Column, Space, row};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tag::tag;
use otty_ui_kit::components::tooltip::{TooltipText, tooltip};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    MoneyKind, MoneyViewModel, OrganizationBillViewModel,
    SpentAtApiPricesViewModel,
};
use super::blocks;

/// The value of a figure OTTY does not know yet.
const NOT_KNOWN: &str = "—";

/// Render the card of money named `title`, "Money" on the page of an
/// account: a figure for each row the key told, the money spent this
/// month, the spend at API prices and the bill of an organization, split
/// by thin rules, then what failed.
pub(super) fn view<'a>(
    vm: MoneyViewModel,
    title: &'static str,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let is_stale = vm.is_stale;

    let mut figures: Vec<Element<'a, AgentsIntent>> = vm
        .rows
        .into_iter()
        .map(|row| blocks::usage_figure(row, is_stale, palette))
        .collect();
    if let Some(spent) = vm.spent {
        let name = i18n::t(Key::AgentsSpentThisMonth).to_string();
        let value = blocks::figure_value(spent, is_stale, palette);
        figures.push(blocks::figure(
            name,
            Vec::new(),
            value,
            None,
            is_stale,
            palette,
        ));
    }
    if let Some(spent) = vm.spent_at_api_prices {
        figures.push(spent_at_api_prices(spent, is_stale, palette));
    }
    if let Some(bill) = vm.organization_bill {
        figures.push(organization_bill(bill, is_stale, palette));
    }

    let mut body = Column::new()
        .spacing(spacing::SPACE_4)
        .push(blocks::figures(figures, palette));
    if vm.failure.is_some() {
        body = body.push(blocks::notes(None, vm.failure, palette));
    }
    blocks::card(title, vm.updated, body, palette)
}

/// "Your spend this month" of the agent's sessions at API prices (issues
/// 22 and 24 of spec 2026-11-23-accounts-apart-from-profiles): where it
/// was counted under the name, the mark "at API prices" with its tooltip
/// next to the amount, and the amount with its kinds on hover.
fn spent_at_api_prices<'a>(
    spent: SpentAtApiPricesViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let tip = kinds_tooltip(&spent);
    let name = i18n::t(Key::AgentsYourSpendThisMonth).to_string();
    let mark = tag(spent.mark.label, typography::BODY.line_height(), palette);
    let amount = blocks::figure_value(spent.amount, is_stale, palette);

    let value = row![
        tooltip(mark, spent.mark.tip, palette),
        tooltip(amount, tip, palette),
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);

    blocks::figure(
        name,
        spent.source.into_iter().collect(),
        value,
        None,
        is_stale,
        palette,
    )
}

/// "Organization bill": the bill read with the admin key, or "—" with
/// the line that it shows once OTTY has an admin key.
fn organization_bill<'a>(
    bill: OrganizationBillViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let name = i18n::t(Key::AgentsOrganizationBill).to_string();
    let (value, explain, is_muted) = match bill {
        OrganizationBillViewModel::Read(amount) => {
            (amount, Vec::new(), is_stale)
        },
        OrganizationBillViewModel::WaitsForAdminKey => {
            let waits = i18n::t(Key::AgentsOrganizationBillWaits).to_string();
            (String::from(NOT_KNOWN), vec![waits], true)
        },
    };

    let value = blocks::figure_value(value, is_muted, palette);
    blocks::figure(name, explain, value, None, is_stale, palette)
}

/// The tooltip of the spend at API prices: each kind at the left and its
/// money at the right.
fn kinds_tooltip(spent: &SpentAtApiPricesViewModel) -> TooltipText {
    let rows = spent
        .kinds
        .iter()
        .map(|line| (line.name.to_string(), line.value.clone()))
        .collect();

    TooltipText::default().rows(rows)
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
    use crate::model::{LimitsViewModel, UsageRowViewModel};
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
            spent_at_api_prices: None,
            organization_bill: None,
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

    /// Issue 22 of spec 2026-11-23: the spend at API prices tells its
    /// kinds on hover, each name at the left and its money at the right.
    #[test]
    fn the_spend_at_api_prices_lists_its_kinds_on_hover() {
        let spent = crate::testing::spent_at_api_prices();

        let tip = kinds_tooltip(&spent);

        assert_eq!(
            tip,
            TooltipText::default().rows(vec![
                (
                    String::from(i18n::t(Key::AgentsKindInput)),
                    String::from("~$30.00")
                ),
                (
                    String::from(i18n::t(Key::AgentsKindCacheWritten)),
                    String::from("~$10.00")
                ),
                (
                    String::from(i18n::t(Key::AgentsKindCached)),
                    String::from("~$2.10")
                ),
            ])
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
        // Issue 24: the row ends at the padding of its card.
        assert!(
            (value.x + value.width - (WIDTH - spacing::SPACE_5)).abs() < 1.0,
            "the value ends the row: {value:?}"
        );
    }
}
