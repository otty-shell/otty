//! The All accounts and All profiles pages in the main area (spec
//! 2026-11-19, Decisions 6 and 7): one column like the profile page, its
//! header with the icon, the title, Refresh and "⋯", a table as tall as its
//! rows, whose row opens the account or the profile and whose right press
//! opens its menu, and the "Needs you" to-do list.

use iced::widget::{Column, column, container, row, svg};
use iced::{Alignment, Element, Length};
use otty_agents::{Account, Profile};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::data_table::{
    ColumnAlign, DataTable, TableCell, TableColumn,
};
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::glyph;
use otty_ui_kit::components::heading_row::HeadingRow;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::components::session_row::SessionTone;
use otty_ui_kit::components::status_badge::StatusTone;
use otty_ui_kit::components::tag::{tag, tag_width};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    AccountTableRowViewModel, AllAccountsViewModel, AllProfilesViewModel,
    EntryIcon, NeedViewModel, ProfileTableRowViewModel,
};
use super::profile_page::{
    BUTTON_GAP, HEADER_GAP, LOGO_SIZE, PAGE_GAP, PAGE_PADDING_X,
};
use super::{blocks, entry_icon, needs};

/// Render the All accounts page.
pub(crate) fn all_accounts<'a>(
    vm: AllAccountsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let header = header(
        EntryIcon::Svg(icons::ALL_ACCOUNTS.into()),
        i18n::t(Key::AgentsAllAccounts),
        AgentsIntent::OpenAllAccountsMenu,
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
        EntryIcon::Svg(icons::ALL_PROFILES.into()),
        i18n::t(Key::AgentsAllProfiles),
        AgentsIntent::OpenAllProfilesMenu,
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

/// The header of a page: the icon and the title, then Refresh and "⋯",
/// which sends `menu`, at the right end. The title ends with "…" when it
/// does not fit.
pub(super) fn header<'a>(
    icon: EntryIcon,
    title: impl Into<String>,
    menu: AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    header_with(icon, title, None, None, menu, palette)
}

/// The header of a page as [`header`] draws it, with `mark`, a short tag
/// right after the title, and `extra`, a button before Refresh, when they
/// are given. The title gives way first, the tag stays whole.
pub(super) fn header_with<'a>(
    icon: EntryIcon,
    title: impl Into<String>,
    mark: Option<String>,
    extra: Option<Element<'a, AgentsIntent>>,
    menu: AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let strong = palette.text_strong();
    let logo = entry_icon::view(icon, LOGO_SIZE, strong, palette);
    let reserved = mark
        .as_deref()
        .map_or(0.0, |mark| tag_width(mark) + BUTTON_GAP);
    let name = EllipsisText::new(
        title,
        typography::TITLE.size(),
        fonts::UI_SEMIBOLD,
        strong,
    )
    .shrink(reserved);
    let mut heading = row![name].spacing(BUTTON_GAP).align_y(Alignment::Center);
    if let Some(mark) = mark {
        heading = heading.push(tag(mark, size::CONTROL_MD, palette));
    }

    let refresh = IconButton::new(
        svg::Handle::from_memory(icons::REFRESH),
        i18n::t(Key::AgentsRefreshAll),
        palette,
    )
    .on_press(AgentsIntent::Refresh);
    let more =
        IconButton::new(glyph::dots(), i18n::t(Key::MoreActions), palette)
            .on_press(menu);
    let mut buttons = row![].spacing(BUTTON_GAP).align_y(Alignment::Center);
    if let Some(extra) = extra {
        buttons = buttons.push(extra);
    }
    let buttons = buttons.push(refresh).push(more);

    HeadingRow::new(heading, buttons)
        .lead(logo)
        .spacing(HEADER_GAP)
        .into()
}

/// The accounts: Account, Provider and Now. A press on a row or on the
/// name opens the account; a right press opens its menu.
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
    let with_menu = accounts.clone();

    let mut table = DataTable::new(columns, palette);
    for row in rows {
        let now = number_cell(row.now.text(), row.now.tone(), row.is_hidden);
        table = table.row_without_menu(vec![
            TableCell::link(row.name),
            TableCell::plain(row.provider),
            now,
        ]);
    }
    let table = table
        .on_row_press(move |index| select_account(&accounts, index))
        .on_link_press(move |index, _, _| select_account(&linked, index))
        .on_row_right_press(move |index, _| account_menu(&with_menu, index));

    blocks::rows_tall(table, count)
}

/// The profiles: Profile, Agent, Version and Today. A press on a row or
/// on the name opens the profile's page; a right press opens its menu.
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
    let with_menu = profiles.clone();

    let mut table = DataTable::new(columns, palette);
    for row in rows {
        table = table.row_without_menu(vec![
            TableCell::link(row.name),
            TableCell::plain(row.agent),
            TableCell::muted_mono(row.version),
            number_cell(row.today, row.tone, row.is_hidden),
        ]);
    }
    let table = table
        .on_row_press(move |index| select_profile(&profiles, index))
        .on_link_press(move |index, _, _| select_profile(&linked, index))
        .on_row_right_press(move |index, _| profile_menu(&with_menu, index));

    blocks::rows_tall(table, count)
}

/// A column of a page table named `key`, `share` parts of the width and
/// at least `min_width` wide, that does not sort.
fn page_column(key: Key, share: u16, min_width: u16) -> TableColumn {
    TableColumn::new(i18n::t(key), share)
        .min_width(min_width)
        .sortable(false)
}

/// A number with its word, a state that needs the user as a badge, or
/// "hidden" as a plain badge.
fn number_cell(
    number: String,
    tone: SessionTone,
    is_hidden: bool,
) -> TableCell {
    if is_hidden {
        return TableCell::badge(number, StatusTone::Neutral);
    }

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

/// Open the menu of the account of the row at `index`; nothing opens
/// when there is no such row.
fn account_menu(accounts: &[Account], index: usize) -> AgentsIntent {
    accounts
        .get(index)
        .cloned()
        .map_or(AgentsIntent::CloseMenu, AgentsIntent::OpenAccountMenu)
}

/// Open the menu of the profile of the row at `index`; nothing opens
/// when there is no such row.
fn profile_menu(profiles: &[Profile], index: usize) -> AgentsIntent {
    profiles
        .get(index)
        .cloned()
        .map_or(AgentsIntent::CloseMenu, AgentsIntent::OpenProfileMenu)
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
    use otty_agents::AgentId;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{AccountNumber, NeedFixViewModel};
    use crate::need::NeedAction;

    fn codex() -> Profile {
        Profile::default_for(
            AgentId::new("codex"),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        )
    }

    fn chatgpt() -> Account {
        crate::testing::chatgpt_login()
    }

    fn log_in_need() -> NeedViewModel {
        NeedViewModel {
            place: String::from("Codex"),
            plugin: None,
            title: String::from("needs login"),
            line: String::from("Nobody is logged in."),
            fix: Some(NeedFixViewModel {
                label: String::from("Log in"),
                action: Some(NeedAction::OpenPage(String::from(
                    "https://example.com/login",
                ))),
            }),
            extra: None,
        }
    }

    fn accounts_page() -> AllAccountsViewModel {
        AllAccountsViewModel {
            rows: vec![AccountTableRowViewModel {
                account: chatgpt(),
                name: String::from("OpenAI · you@example.com"),
                provider: String::from("OpenAI"),
                now: AccountNumber::word(
                    String::from("42 % used"),
                    SessionTone::Muted,
                ),
                is_hidden: false,
            }],
            needs: Vec::new(),
        }
    }

    fn profiles_page() -> AllProfilesViewModel {
        AllProfilesViewModel {
            rows: vec![ProfileTableRowViewModel {
                profile: codex(),
                name: String::from("Codex"),
                agent: String::from("Codex"),
                version: String::from("0.169.0"),
                today: String::from("needs login"),
                tone: SessionTone::Failed,
                is_hidden: false,
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
            .expect("the ProviderRef column should show");
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
            vec![AgentsIntent::FixNeed(NeedAction::OpenPage(String::from(
                "https://example.com/login"
            )))]
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

    /// What a press on the header button `from_end` places from the right
    /// end of the title line sends, on a page as wide as a form in the
    /// middle of a main area 1000 wide: 0 is "⋯", 1 Refresh.
    fn header_pressed(
        page: Element<'_, AgentsIntent>,
        title: &str,
        from_end: usize,
    ) -> Vec<AgentsIntent> {
        const WIDTH: f32 = 1000.0;
        let mut ui = simulator(container(page).width(WIDTH));
        let title = ui.find(title).expect("the title should show").bounds();
        let right_end = (WIDTH + size::FORM) / 2.0;
        let button = size::CONTROL_MD;
        let x = right_end
            - button / 2.0
            - from_end as f32
                * (button + super::super::profile_page::BUTTON_GAP);

        ui.point_at(iced::Point::new(x, title.center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    /// "⋯" ends the line of the title, Refresh before it: "⋯" opens the
    /// menu of the page (Show hidden, Show Found by OTTY).
    #[test]
    fn the_header_ends_with_refresh_and_the_menu_of_the_page() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let title = i18n::t(Key::AgentsAllAccounts);

        let menu =
            header_pressed(all_accounts(accounts_page(), palette), title, 0);
        let refresh =
            header_pressed(all_accounts(accounts_page(), palette), title, 1);
        let profiles = header_pressed(
            all_profiles(profiles_page(), palette),
            i18n::t(Key::AgentsAllProfiles),
            0,
        );

        assert_eq!(menu, vec![AgentsIntent::OpenAllAccountsMenu]);
        assert_eq!(refresh, vec![AgentsIntent::Refresh]);
        assert_eq!(profiles, vec![AgentsIntent::OpenAllProfilesMenu]);
    }

    /// Spec 2026-11-19, Decision 7: a right click on a row opens the menu
    /// of its account or profile.
    #[test]
    fn a_right_click_on_a_row_opens_its_menu() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let right_click_on = |page: Element<'_, AgentsIntent>, label: &str| {
            let mut ui = simulator(page);
            let row = ui.find(label).expect("the row should show");
            ui.point_at(row.bounds().center());
            let _ = ui.simulate(crate::testing::right_click());
            ui.into_messages().collect::<Vec<_>>()
        };

        let account =
            right_click_on(all_accounts(accounts_page(), palette), "42 % used");
        let profile =
            right_click_on(all_profiles(profiles_page(), palette), "0.169.0");

        assert_eq!(account, vec![AgentsIntent::OpenAccountMenu(chatgpt())]);
        assert_eq!(profile, vec![AgentsIntent::OpenProfileMenu(codex())]);
    }

    #[test]
    fn a_hidden_row_is_marked_hidden() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let mut page = accounts_page();
        page.rows[0].now =
            AccountNumber::word(String::from("hidden"), SessionTone::Muted);
        page.rows[0].is_hidden = true;

        let mut ui = simulator(all_accounts(page, palette));

        ui.find("hidden").expect("the row should say it is hidden");
    }
}
