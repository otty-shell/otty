//! The page of one account in the main area (spec 2026-11-19, Decisions 6
//! and 7): one column like the profile page, its header with the provider
//! icon, the name, Refresh and "⋯", then for a hidden account a line that
//! says so, "Needs from you" as a to-do list when there is anything, the
//! Limits, Credits or Money block, and "Account info" with the plugin that
//! reads the account. No profiles: nothing links an account to a profile
//! (spec 2026-11-23-accounts-apart-from-profiles, Decision 1).

use iced::widget::{Column, column, container};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::super::event::AgentsIntent;
use super::super::model::{AccountNumbersViewModel, AccountPageViewModel};
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
        body = body.push(blocks::titled(
            i18n::t(Key::AgentsNeedsFromYou),
            needs::view(vm.needs, AgentsIntent::FixNeed, palette),
            palette,
        ));
    }
    if let Some(numbers) = vm.numbers {
        body = body.push(numbers_block(numbers, palette));
    }
    body.push(blocks::titled(
        i18n::t(Key::AgentsAccountInfo),
        blocks::facts(vm.info, palette),
        palette,
    ))
}

/// The Limits, Credits or Money block, named after what it shows.
fn numbers_block<'a>(
    numbers: AccountNumbersViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let title = numbers.title();

    match numbers {
        AccountNumbersViewModel::Money(vm) => money::view(vm, title, palette),
        AccountNumbersViewModel::Limits(vm)
        | AccountNumbersViewModel::Credits(vm) => {
            limits::view(vm, title, palette)
        },
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::container;
    use iced_test::simulator;
    use otty_agents::Account;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{
        AccountNumbersViewModel, FactViewModel, LimitsViewModel,
        MoneyViewModel, NeedFixViewModel, NeedViewModel, UsageRowViewModel,
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
            name: String::from("OpenAI · you@example.com"),
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
            place: String::from("OpenAI · you@example.com"),
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
            "OpenAI · you@example.com",
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

    /// Spec 2026-11-19, Decision 7: "⋯" ends the title line and opens the
    /// menu of the account.
    #[test]
    fn the_menu_button_opens_the_menu_of_the_account() {
        let themes = ThemeManager::new();
        let mut ui = simulator(
            container(view(page(), themes.current().ui())).width(WIDTH),
        );
        let title = ui
            .find("OpenAI · you@example.com")
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
}
