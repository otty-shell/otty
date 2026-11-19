//! The All accounts and All profiles pages in the main area (spec
//! 2026-11-19, Decision 6): one column like the dashboard, its header with
//! the icon, the title and Refresh, a table as tall as its rows, whose row
//! opens the account or the profile, and the "Needs you" to-do list.

use iced::widget::{Column, column, container, svg};
use iced::{Element, Length};
use otty_agents::{Account, Profile};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::data_table::{
    ColumnAlign, DataTable, TableCell, TableColumn,
};
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::heading_row::HeadingRow;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::components::session_row::SessionTone;
use otty_ui_kit::components::status_badge::StatusTone;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    AccountTableRowViewModel, AllAccountsViewModel, AllProfilesViewModel,
    NeedViewModel, ProfileTableRowViewModel,
};
use super::dashboard::{HEADER_GAP, LOGO_SIZE, PAGE_GAP, PAGE_PADDING_X};
use super::{blocks, needs};

/// Render the All accounts page.
pub(crate) fn all_accounts<'a>(
    vm: AllAccountsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let header = header(
        icons::ALL_ACCOUNTS,
        i18n::t(Key::AgentsAllAccounts),
        palette,
    );
    let table = accounts_table(vm.rows, palette);

    page(header, table, vm.needs, palette)
}

/// Render the All profiles page.
pub(crate) fn all_profiles<'a>(
    vm: AllProfilesViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let header = header(
        icons::ALL_PROFILES,
        i18n::t(Key::AgentsAllProfiles),
        palette,
    );
    let table = profiles_table(vm.rows, palette);

    page(header, table, vm.needs, palette)
}

/// The page: the header over the table and the needs, as wide as the room
/// and at most a form's width, centred across the main area and starting
/// at its top.
fn page<'a>(
    header: Element<'a, AgentsIntent>,
    table: Element<'a, AgentsIntent>,
    needs: Vec<NeedViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut body = Column::new().spacing(blocks::BLOCK_GAP).push(table);
    if !needs.is_empty() {
        body = body.push(blocks::titled(
            i18n::t(Key::AgentsNeedsYou),
            needs::view(needs, AgentsIntent::FixNeed, palette),
            palette,
        ));
    }

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

/// The header of a page: the icon and the title, then Refresh at the
/// right end. The title ends with "…" when it does not fit.
pub(super) fn header<'a>(
    icon: &'static [u8],
    title: impl Into<String>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let strong = palette.text_strong();
    let logo = svg(svg::Handle::from_memory(icon))
        .width(LOGO_SIZE)
        .height(LOGO_SIZE)
        .style(move |_, _| svg::Style {
            color: Some(strong),
        });
    let name = EllipsisText::new(
        title,
        typography::TITLE.size(),
        fonts::UI_SEMIBOLD,
        strong,
    )
    .shrink(0.0);
    let refresh = IconButton::new(
        svg::Handle::from_memory(icons::REFRESH),
        i18n::t(Key::AgentsRefreshAll),
        palette,
    )
    .on_press(AgentsIntent::Refresh);

    HeadingRow::new(name, refresh)
        .lead(logo)
        .spacing(HEADER_GAP)
        .into()
}

/// The accounts: Account, Provider and Now. A press on a row or on the
/// name opens the account.
fn accounts_table<'a>(
    rows: Vec<AccountTableRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let columns = vec![
        page_column(Key::AgentsFieldAccount, 3, 150),
        page_column(Key::AgentsColumnProvider, 2, 90),
        page_column(Key::AgentsColumnNow, 2, 110).align(ColumnAlign::End),
    ];
    let count = rows.len();
    let accounts: Vec<Account> =
        rows.iter().map(|row| row.account.clone()).collect();
    let linked = accounts.clone();

    let mut table = DataTable::new(columns, palette);
    for row in rows {
        table = table.row_without_menu(vec![
            TableCell::link(row.name),
            TableCell::plain(row.provider),
            number_cell(row.now, row.tone),
        ]);
    }
    let table = table
        .on_row_press(move |index| select_account(&accounts, index))
        .on_link_press(move |index, _, _| select_account(&linked, index));

    blocks::rows_tall(table, count)
}

/// The profiles: Profile, Agent, Version and Today. A press on a row or
/// on the name opens the profile's dashboard.
fn profiles_table<'a>(
    rows: Vec<ProfileTableRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let columns = vec![
        page_column(Key::AgentsProfile, 3, 120),
        page_column(Key::AgentsAgent, 2, 90),
        page_column(Key::AgentsColumnVersion, 2, 80),
        page_column(Key::AgentsToday, 2, 120).align(ColumnAlign::End),
    ];
    let count = rows.len();
    let profiles: Vec<Profile> =
        rows.iter().map(|row| row.profile.clone()).collect();
    let linked = profiles.clone();

    let mut table = DataTable::new(columns, palette);
    for row in rows {
        table = table.row_without_menu(vec![
            TableCell::link(row.name),
            TableCell::plain(row.agent),
            TableCell::muted_mono(row.version),
            number_cell(row.today, row.tone),
        ]);
    }
    let table = table
        .on_row_press(move |index| select_profile(&profiles, index))
        .on_link_press(move |index, _, _| select_profile(&linked, index));

    blocks::rows_tall(table, count)
}

/// A column of a page table named `key`, `share` parts of the width and
/// at least `min_width` wide, that does not sort.
fn page_column(key: Key, share: u16, min_width: u16) -> TableColumn {
    TableColumn::new(i18n::t(key), share)
        .min_width(min_width)
        .sortable(false)
}

/// A number with its word, or a state that needs the user as a badge.
fn number_cell(number: String, tone: SessionTone) -> TableCell {
    match tone {
        SessionTone::Failed => TableCell::badge(number, StatusTone::Warn),
        _ => TableCell::number(number),
    }
}

/// Open the account of the row at `index`; the page itself stays when
/// there is no such row.
fn select_account(accounts: &[Account], index: usize) -> AgentsIntent {
    accounts
        .get(index)
        .cloned()
        .map_or(AgentsIntent::SelectAllAccounts, AgentsIntent::SelectAccount)
}

/// Open the profile of the row at `index`; the page itself stays when
/// there is no such row.
fn select_profile(profiles: &[Profile], index: usize) -> AgentsIntent {
    profiles
        .get(index)
        .cloned()
        .map_or(AgentsIntent::SelectAllProfiles, AgentsIntent::SelectProfile)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, AgentId, FoundIn, Provider,
    };
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::NeedFixViewModel;
    use crate::need::NeedAction;

    fn codex() -> Profile {
        Profile::default_for(
            AgentId::new("codex"),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        )
    }

    fn chatgpt() -> Account {
        Account::new(
            Provider::OpenAI,
            AccountIdentity::AccountId(String::from("made-up-account")),
            AccountLogin::Email(String::from("you@example.com")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
        )
    }

    fn log_in_need() -> NeedViewModel {
        NeedViewModel {
            place: String::from("Codex"),
            title: String::from("needs login"),
            line: String::from("Nobody is logged in."),
            fix: Some(NeedFixViewModel {
                label: String::from("Log in"),
                action: NeedAction::LogIn(codex()),
            }),
        }
    }

    fn accounts_page() -> AllAccountsViewModel {
        AllAccountsViewModel {
            rows: vec![AccountTableRowViewModel {
                account: chatgpt(),
                name: String::from("OpenAI · you@example.com"),
                provider: "OpenAI",
                now: String::from("42 % used"),
                tone: SessionTone::Muted,
            }],
            needs: Vec::new(),
        }
    }

    fn profiles_page() -> AllProfilesViewModel {
        AllProfilesViewModel {
            rows: vec![ProfileTableRowViewModel {
                profile: codex(),
                name: String::from("Codex"),
                agent: "Codex",
                version: String::from("0.169.0"),
                today: String::from("needs login"),
                tone: SessionTone::Failed,
            }],
            needs: vec![log_in_need()],
        }
    }

    #[test]
    fn all_accounts_shows_its_table_and_a_row_opens_the_account() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let mut ui = simulator(all_accounts(accounts_page(), palette));

        ui.find(i18n::t(Key::AgentsAllAccounts))
            .expect("the title should show");
        ui.find(i18n::t(Key::AgentsColumnProvider))
            .expect("the Provider column should show");
        ui.find("42 % used").expect("the number should show");
        assert!(
            ui.find(i18n::t(Key::AgentsNeedsYou)).is_err(),
            "with no needs there is no to-do list"
        );
        ui.click("OpenAI").expect("the provider should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![AgentsIntent::SelectAccount(chatgpt())]);
    }

    #[test]
    fn all_profiles_shows_its_needs_and_their_buttons_act() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let mut ui = simulator(all_profiles(profiles_page(), palette));

        ui.find(i18n::t(Key::AgentsAllProfiles))
            .expect("the title should show");
        ui.find("0.169.0").expect("the version should show");
        ui.find(i18n::t(Key::AgentsNeedsYou))
            .expect("the to-do list should show");
        ui.find("Codex · needs login")
            .expect("the need should show");
        ui.click("Log in").expect("the need's button should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![AgentsIntent::FixNeed(NeedAction::LogIn(codex()))]
        );
    }

    #[test]
    fn a_profile_row_opens_its_dashboard() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let mut ui = simulator(all_profiles(profiles_page(), palette));

        ui.click("0.169.0").expect("the version should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![AgentsIntent::SelectProfile(codex())]);
    }

    /// Refresh ends the line of the title, at the right of the column as
    /// wide as a form in the middle of a main area 1000 wide.
    #[test]
    fn refresh_ends_the_line_of_the_title_and_reads_everything_again() {
        const WIDTH: f32 = 1000.0;
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let page =
            container(all_accounts(accounts_page(), palette)).width(WIDTH);
        let mut ui = simulator(page);
        let title = ui
            .find(i18n::t(Key::AgentsAllAccounts))
            .expect("the title should show")
            .bounds();
        let right_end = (WIDTH + size::FORM) / 2.0;

        ui.point_at(iced::Point::new(
            right_end - size::CONTROL_MD / 2.0,
            title.center_y(),
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![AgentsIntent::Refresh]);
    }
}
