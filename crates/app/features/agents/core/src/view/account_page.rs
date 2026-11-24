//! The page of one account in the main area (spec 2026-11-19, Decisions 6
//! and 7): one column like the profile page, its header with the provider
//! icon, the name, Refresh and "⋯", then for a hidden account a line that
//! says so, "Needs from you" as a to-do list when there is anything, the
//! Limits, Credits or Money block, and "Account info" with the plugin that
//! reads the account. No profiles: nothing links an account to a profile
//! (spec 2026-11-23-accounts-apart-from-profiles, Decision 1). Each part
//! is a kit card, the needs a warning card (issue 24).

use iced::widget::{Column, column, container};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::card::Card;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::super::event::AgentsIntent;
use super::super::model::{
    AccountNumbersViewModel, AccountPageViewModel, LimitsViewModel,
};
use super::profile_page::{PAGE_GAP, PAGE_PADDING_X};
use super::{blocks, limits, money, needs, overview};

/// Render the page of an account: one column as wide as the room, at most
/// a form's width, centred across the main area and starting at its top.
pub(crate) fn view<'a>(
    vm: AccountPageViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let header = header(&vm, palette);
    let body = page_blocks(vm, palette);

    let page = column![header, body]
        .spacing(PAGE_GAP)
        .width(Length::Fill)
        .max_width(size::FORM);

    container(page)
        .padding([spacing::SPACE_5, PAGE_PADDING_X])
        .center_x(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The header: the logo and the name, then Refresh and "⋯". OTTY's own
/// login has the mark "OTTY's own login" after the name and a small Log in
/// again before Refresh (issue 09 of spec
/// 2026-11-23-accounts-apart-from-profiles).
fn header<'a>(
    vm: &AccountPageViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let menu = AgentsIntent::OpenAccountMenu(vm.account.clone());
    if !vm.has_own_login {
        return overview::header(
            vm.icon.clone(),
            vm.name.clone(),
            menu,
            palette,
        );
    }

    let mark = i18n::t(Key::AgentsSourceOwnLogin).to_string();
    let log_in_again = Button::new(i18n::t(Key::AgentsLogInAgain), palette)
        .kind(ButtonKind::Secondary)
        .size(ButtonSize::Small)
        .on_press(AgentsIntent::LogInAgain(vm.account.clone()));

    overview::header_with(
        vm.icon.clone(),
        vm.name.clone(),
        Some(mark),
        Some(log_in_again.into()),
        menu,
        palette,
    )
}

/// The blocks under the header, in their order: the line of a hidden
/// account, the needs when there are any, the numbers and its facts.
fn page_blocks<'a>(
    vm: AccountPageViewModel,
    palette: &UiPalette,
) -> Column<'a, AgentsIntent> {
    let mut body = Column::new().spacing(blocks::BLOCK_GAP);

    if vm.hidden_note.is_some() {
        body = body.push(blocks::notes(vm.hidden_note, None, palette));
    }
    if !vm.needs.is_empty() {
        body = body.push(needs::card(
            i18n::t(Key::AgentsNeedsFromYou),
            vm.needs,
            AgentsIntent::FixNeed,
            palette,
        ));
    }
    if let Some(numbers) = vm.numbers {
        body = body.push(numbers_block(numbers, palette));
    }
    let info = blocks::facts(vm.info, palette);
    body.push(Card::new(info, palette).title(i18n::t(Key::AgentsAccountInfo)))
}

/// The Limits, Credits or Money card, named after what it shows. A
/// subscription with credits and windows has two: Limits, then Credits.
fn numbers_block<'a>(
    numbers: AccountNumbersViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let title = numbers.title();

    match numbers {
        AccountNumbersViewModel::Money(vm) => money::view(vm, title, palette),
        AccountNumbersViewModel::Credits(vm) if !vm.windows.is_empty() => {
            limits_and_credits(vm, palette)
        },
        AccountNumbersViewModel::Limits(vm)
        | AccountNumbersViewModel::Credits(vm) => {
            limits::view(vm, title, palette)
        },
    }
}

/// The windows of `vm` in the Limits card and its credits in the Credits
/// card under it, with what is old and what failed.
fn limits_and_credits<'a>(
    vm: LimitsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let windows = LimitsViewModel {
        updated: vm.updated.clone(),
        windows: vm.windows,
        credits: None,
        is_stale: vm.is_stale,
        recorded: None,
        failure: None,
    };
    let credits = LimitsViewModel {
        updated: vm.updated,
        windows: Vec::new(),
        credits: vm.credits,
        is_stale: vm.is_stale,
        recorded: vm.recorded,
        failure: vm.failure,
    };

    column![
        limits::view(windows, i18n::t(Key::AgentsNumberLimits), palette),
        limits::view(credits, i18n::t(Key::AgentsFieldCredits), palette),
    ]
    .spacing(blocks::BLOCK_GAP)
    .into()
}

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced::widget::container;
    use iced_test::simulator;
    use otty_agents::Account;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{
        AccountNumbersViewModel, FactViewModel, LimitsViewModel,
        MoneyViewModel, NeedFixViewModel, NeedViewModel,
        OrganizationBillViewModel, UsageRowViewModel,
    };
    use crate::need::NeedAction;

    /// Width of the main area in the tests.
    const WIDTH: f32 = 1000.0;

    fn chatgpt() -> Account {
        crate::testing::chatgpt_login()
    }

    fn page() -> AccountPageViewModel {
        AccountPageViewModel {
            account: chatgpt(),
            name: String::from("OpenAI (you@example.com)"),
            icon: crate::testing::provider_icon("openai"),
            needs: Vec::new(),
            numbers: Some(AccountNumbersViewModel::Limits(LimitsViewModel {
                updated: Some(String::from("Updated 2 min ago")),
                windows: vec![UsageRowViewModel {
                    name: String::from("5 hours"),
                    used: String::from("42 % used"),
                    percent: Some(42),
                    resets: None,
                }],
                credits: None,
                is_stale: false,
                recorded: None,
                failure: None,
            })),
            info: vec![
                FactViewModel::new(
                    i18n::t(Key::AgentsColumnProvider),
                    String::from("OpenAI"),
                ),
                FactViewModel::new(
                    i18n::t(Key::AgentsColumnFoundIn),
                    String::from("~/.codex"),
                ),
            ],
            hidden_note: None,
            has_own_login: false,
        }
    }

    fn expired() -> NeedViewModel {
        NeedViewModel {
            place: String::from("OpenAI (you@example.com)"),
            plugin: None,
            title: String::from("The login expired"),
            line: String::from("Open Codex to log in again."),
            fix: Some(NeedFixViewModel {
                label: String::from("Open Codex"),
                action: Some(NeedAction::Run {
                    title: String::from("Codex"),
                    command: otty_agents::AgentCommand::new("codex"),
                }),
            }),
            extra: None,
        }
    }

    fn sent(vm: AccountPageViewModel, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let page = container(view(vm, themes.current().ui())).width(WIDTH);
        let mut ui = simulator(page);

        ui.click(label).expect("the text should show");

        ui.into_messages().collect()
    }

    #[test]
    fn the_page_shows_its_blocks_and_no_needs_when_there_are_none() {
        let themes = ThemeManager::new();
        let page = container(view(page(), themes.current().ui())).width(WIDTH);
        let mut ui = simulator(page);

        for shown in [
            "OpenAI (you@example.com)",
            i18n::t(Key::AgentsNumberLimits),
            "Updated 2 min ago",
            "42 % used",
            i18n::t(Key::AgentsAccountInfo),
            "~/.codex",
        ] {
            ui.find(shown).expect("the part of the page should show");
        }
        assert!(ui.find(i18n::t(Key::AgentsNeedsFromYou)).is_err());
        assert!(ui.find(i18n::t(Key::AgentsUsage)).is_err());
    }

    #[test]
    fn the_button_of_a_need_opens_the_agent() {
        let vm = AccountPageViewModel {
            needs: vec![expired()],
            ..page()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(
            container(view(vm.clone(), themes.current().ui())).width(WIDTH),
        );
        ui.find(i18n::t(Key::AgentsNeedsFromYou))
            .expect("the needs show");

        let intents = sent(vm, "Open Codex");

        assert_eq!(
            intents,
            vec![AgentsIntent::FixNeed(NeedAction::Run {
                title: String::from("Codex"),
                command: otty_agents::AgentCommand::new("codex"),
            })]
        );
    }

    #[test]
    fn a_money_account_shows_the_money_block() {
        let vm = AccountPageViewModel {
            numbers: Some(AccountNumbersViewModel::Money(MoneyViewModel {
                updated: None,
                rows: Vec::new(),
                spent: Some(String::from("$42.00")),
                spent_at_api_prices: None,
                organization_bill: None,
                is_stale: false,
                failure: None,
            })),
            ..page()
        };
        let themes = ThemeManager::new();
        let mut ui =
            simulator(container(view(vm, themes.current().ui())).width(WIDTH));

        ui.find(i18n::t(Key::AgentsNumberMoney))
            .expect("the block is named Money");
        ui.find(i18n::t(Key::AgentsSpentThisMonth))
            .expect("the row shows");
        ui.find("$42.00").expect("the bill shows");
        assert!(
            ui.find(i18n::t(Key::AgentsApiPricesMark)).is_err(),
            "the money of an account is as its provider told it"
        );
    }

    /// Issue 22 of spec 2026-11-23: what a key spent this month by its
    /// sessions shows as "Your spend this month", with the mark "at API
    /// prices" and where it was counted.
    #[test]
    fn a_key_shows_its_spend_at_api_prices_with_the_mark() {
        let vm = AccountPageViewModel {
            numbers: Some(AccountNumbersViewModel::Money(MoneyViewModel {
                updated: None,
                rows: Vec::new(),
                spent: None,
                spent_at_api_prices: Some(crate::testing::spent_at_api_prices()),
                organization_bill: None,
                is_stale: false,
                failure: None,
            })),
            ..page()
        };
        let themes = ThemeManager::new();
        let mut ui =
            simulator(container(view(vm, themes.current().ui())).width(WIDTH));

        for shown in [
            i18n::t(Key::AgentsYourSpendThisMonth),
            "~$42.10",
            i18n::t(Key::AgentsApiPricesMark),
            "From your Claude Code sessions on this computer",
        ] {
            ui.find(shown).expect("the spend at API prices shows");
        }
        assert!(
            ui.find(i18n::t(Key::AgentsSpentThisMonth)).is_err(),
            "no exact money"
        );
    }

    /// Spec 2026-11-19, Decision 7: "⋯" ends the title line and opens the
    /// menu of the account.
    #[test]
    fn the_menu_button_opens_the_menu_of_the_account() {
        let themes = ThemeManager::new();
        let mut ui = simulator(
            container(view(page(), themes.current().ui())).width(WIDTH),
        );
        let title = ui
            .find("OpenAI (you@example.com)")
            .expect("the title should show")
            .bounds();
        let right_end = (WIDTH + otty_ui_kit::size::FORM) / 2.0;

        ui.point_at(iced::Point::new(
            right_end - otty_ui_kit::size::CONTROL_MD / 2.0,
            title.center_y(),
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![AgentsIntent::OpenAccountMenu(chatgpt())]);
    }

    /// Issue 09 of spec 2026-11-23-accounts-apart-from-profiles: the
    /// header of an account with OTTY's own login has the mark "OTTY's own
    /// login" and a Log in again button; a found account has neither.
    #[test]
    fn the_header_of_an_own_login_has_its_mark_and_logs_in_again() {
        let vm = AccountPageViewModel {
            has_own_login: true,
            ..page()
        };
        let themes = ThemeManager::new();
        let mut found = simulator(
            container(view(page(), themes.current().ui())).width(WIDTH),
        );
        let mut own = simulator(
            container(view(vm.clone(), themes.current().ui())).width(WIDTH),
        );

        own.find(i18n::t(Key::AgentsSourceOwnLogin))
            .expect("the mark shows");
        assert!(found.find(i18n::t(Key::AgentsSourceOwnLogin)).is_err());
        assert!(found.find(i18n::t(Key::AgentsLogInAgain)).is_err());
        assert_eq!(
            sent(vm, i18n::t(Key::AgentsLogInAgain)),
            [AgentsIntent::LogInAgain(chatgpt())]
        );
    }

    #[test]
    fn a_hidden_account_says_so_under_its_title() {
        let vm = AccountPageViewModel {
            numbers: None,
            hidden_note: Some(String::from("Hidden: OTTY does not read it.")),
            ..page()
        };
        let themes = ThemeManager::new();
        let mut ui =
            simulator(container(view(vm, themes.current().ui())).width(WIDTH));

        ui.find("Hidden: OTTY does not read it.")
            .expect("the note shows");
    }

    /// The left and the right edge of the column of the page in the main
    /// area [`WIDTH`] wide: a form's width in the middle.
    fn page_edges() -> (f32, f32) {
        let form = otty_ui_kit::size::FORM;

        ((WIDTH - form) / 2.0, (WIDTH + form) / 2.0)
    }

    fn at(
        ui: &mut iced_test::Simulator<'_, AgentsIntent>,
        label: &str,
    ) -> Rectangle {
        ui.find(label).expect("the text should show").bounds()
    }

    /// Whether `part` lies inside a card that spans the column of the page
    /// from under its title `title` to over `next`, the title of the card
    /// after it, with the padding of a card around.
    fn inside_card(part: Rectangle, title: Rectangle, next: Rectangle) -> bool {
        let (left, right) = page_edges();
        let padding = spacing::SPACE_5;

        part.x >= left + padding - 0.5
            && part.x + part.width <= right - padding + 0.5
            && part.y >= title.y + title.height
            && part.y + part.height + padding <= next.y + 0.5
    }

    fn need_of(title: &str, buttons: &[&str]) -> NeedViewModel {
        let fix = |label: &str| NeedFixViewModel {
            label: label.to_string(),
            action: Some(NeedAction::OpenPage(String::from(
                "https://example.com",
            ))),
        };

        NeedViewModel {
            place: String::from("Example Org"),
            plugin: Some(String::from("Anthropic plugin")),
            title: title.to_string(),
            line: String::from(
                "OTTY shows no money until it has an admin key.",
            ),
            fix: buttons.last().map(|label| fix(label)),
            extra: (buttons.len() > 1).then(|| fix(buttons[0])),
        }
    }

    /// The page of an organization with no admin key: two needs and the
    /// money of its sessions, its bill waiting.
    fn organization_page() -> AccountPageViewModel {
        AccountPageViewModel {
            account: crate::testing::organization(),
            name: String::from("Example Org"),
            needs: vec![
                need_of("the real bill needs an admin key", &["Admin key…"]),
                need_of("asks for new permissions", &["Review", "Later"]),
            ],
            numbers: Some(AccountNumbersViewModel::Money(MoneyViewModel {
                updated: Some(String::from("Updated 2 min ago")),
                rows: Vec::new(),
                spent: None,
                spent_at_api_prices: Some(crate::testing::spent_at_api_prices()),
                organization_bill: Some(
                    OrganizationBillViewModel::WaitsForAdminKey,
                ),
                is_stale: false,
                failure: None,
            })),
            ..page()
        }
    }

    /// Issue 24 of spec 2026-11-23: every need, its label and its buttons
    /// lie inside the Needs card.
    #[test]
    fn the_need_rows_and_their_buttons_lie_inside_the_needs_card() {
        let themes = ThemeManager::new();
        let page = view(organization_page(), themes.current().ui());
        let mut ui = simulator(container(page).width(WIDTH));
        let title = at(&mut ui, i18n::t(Key::AgentsNeedsFromYou));
        let next = at(&mut ui, i18n::t(Key::AgentsNumberMoney));
        let heading = i18n::filled(
            Key::TplAgentsNeedHeading,
            &[
                ("place", "Example Org"),
                ("title", "asks for new permissions"),
            ],
        );

        for part in [
            heading.as_str(),
            "Anthropic plugin",
            "Admin key…",
            "Review",
            "Later",
        ] {
            let bounds = at(&mut ui, part);
            assert!(
                inside_card(bounds, title, next),
                "{part} at {bounds:?} inside the card from {title:?} to {next:?}"
            );
        }
    }

    /// Issue 24: Money of an organization has a row for the spend of its
    /// sessions, its source under its name and the mark next to the
    /// amount, and a row for its bill, which waits for an admin key; all
    /// inside the Money card.
    #[test]
    fn the_spend_and_the_bill_of_an_organization_lie_inside_the_money_card() {
        let themes = ThemeManager::new();
        let page = view(organization_page(), themes.current().ui());
        let mut ui = simulator(container(page).width(WIDTH));
        let (left, right) = page_edges();
        let title = at(&mut ui, i18n::t(Key::AgentsNumberMoney));
        let next = at(&mut ui, i18n::t(Key::AgentsAccountInfo));
        let source = "From your Claude Code sessions on this computer";

        let spend = at(&mut ui, i18n::t(Key::AgentsYourSpendThisMonth));
        let amount = at(&mut ui, "~$42.10");
        let mark = at(&mut ui, i18n::t(Key::AgentsApiPricesMark));
        let from = at(&mut ui, source);
        let bill = at(&mut ui, i18n::t(Key::AgentsOrganizationBill));
        let dash = at(&mut ui, "—");
        let waits = at(&mut ui, i18n::t(Key::AgentsOrganizationBillWaits));

        for part in [spend, amount, mark, from, bill, dash, waits] {
            assert!(inside_card(part, title, next), "{part:?} inside");
        }
        assert!((spend.x - (left + spacing::SPACE_5)).abs() < 0.5);
        assert!(
            (amount.x + amount.width - (right - spacing::SPACE_5)).abs() < 0.5
        );
        assert!(
            mark.x + mark.width <= amount.x,
            "{mark:?} before {amount:?}"
        );
        assert!((mark.center_y() - amount.center_y()).abs() < 1.0);
        assert_eq!(from.x, spend.x, "the source under its name");
        assert!(from.y >= spend.y + spend.height);
        assert!(bill.y >= from.y + from.height, "{bill:?} under {from:?}");
        assert!((dash.x + dash.width - (right - spacing::SPACE_5)).abs() < 0.5);
        assert_eq!(waits.x, bill.x);
        assert!(waits.y >= bill.y + bill.height);
    }

    /// Issue 24: the windows of a subscription with credits are one card,
    /// Limits, and its credits another, Credits; each figure a row with
    /// its value at the right and its line under its name.
    #[test]
    fn limits_and_credits_are_two_cards_with_a_row_per_figure() {
        let vm = AccountPageViewModel {
            numbers: Some(AccountNumbersViewModel::Credits(LimitsViewModel {
                updated: Some(String::from("Updated just now")),
                windows: vec![UsageRowViewModel {
                    name: String::from("5 hours"),
                    used: String::from("12 % used"),
                    percent: Some(12),
                    resets: Some(String::from("Resets at 15:30")),
                }],
                credits: Some(crate::model::CreditsViewModel {
                    value: String::from("1 240 of 5 000 used"),
                    percent: Some(25),
                    line: Some(String::from("3 760 left")),
                    note: None,
                }),
                is_stale: false,
                recorded: None,
                failure: None,
            })),
            ..page()
        };
        let themes = ThemeManager::new();
        let mut ui =
            simulator(container(view(vm, themes.current().ui())).width(WIDTH));
        let (left, right) = page_edges();

        let limits = at(&mut ui, i18n::t(Key::AgentsNumberLimits));
        let credits = at(&mut ui, i18n::t(Key::AgentsFieldCredits));
        let info = at(&mut ui, i18n::t(Key::AgentsAccountInfo));
        let window = at(&mut ui, "5 hours");
        let used = at(&mut ui, "12 % used");
        let resets = at(&mut ui, "Resets at 15:30");
        let pool = at(&mut ui, "1 240 of 5 000 used");
        let left_line = at(&mut ui, "3 760 left");

        for part in [window, used, resets] {
            assert!(inside_card(part, limits, credits), "{part:?} in Limits");
        }
        for part in [pool, left_line] {
            assert!(inside_card(part, credits, info), "{part:?} in Credits");
        }
        assert!((window.x - (left + spacing::SPACE_5)).abs() < 0.5);
        assert!((used.x + used.width - (right - spacing::SPACE_5)).abs() < 0.5);
        assert!((window.center_y() - used.center_y()).abs() < 1.0);
        assert_eq!(resets.x, window.x, "the line under the name");
        assert!(resets.y >= window.y + window.height);
        assert!((pool.x + pool.width - (right - spacing::SPACE_5)).abs() < 0.5);
    }
}
