//! The Agents mode left panel under the switcher: the Accounts section,
//! the Profiles section and the Projects section, sharing the panel height
//! the way Sessions and Launches do in Terminal mode. Accounts and
//! Profiles start with their All row; Projects is as it was.

use iced::widget::{
    Column, column, container, mouse_area, responsive, scrollable, svg, text,
};
use iced::{Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::icon_button::{
    IconButton, IconButtonSize, IconTone,
};
use otty_ui_kit::components::panel_height::{
    SectionHeight, share_panel_height,
};
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::components::{divider, scrollbar};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    AccountRowViewModel, AccountsSectionViewModel, AgentsSectionViewModel,
    AllRowViewModel, LeftPanelViewModel, ProfileRowViewModel,
};
use super::projects_section;

/// Props for the left panel.
pub(crate) struct LeftPanelProps<'a> {
    pub(crate) vm: LeftPanelViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the left panel: the Accounts section, a line, the Profiles
/// section, a line, then the Projects section. Each open section takes the
/// height of its rows while all fit and scrolls inside its share when they
/// do not (see [`share_panel_height`]); the Projects section takes all the
/// height left under the others.
///
/// The panel height is known only during layout, so this uses
/// `iced::widget::responsive`.
pub(crate) fn view(props: LeftPanelProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    responsive(move |panel| {
        let accounts = SectionHeight::panel_section(
            accounts_content_height(&vm.accounts),
            vm.accounts.is_open,
        );
        let agents = SectionHeight::panel_section(
            agents_content_height(&vm.agents),
            vm.agents.is_open,
        );
        let projects = SectionHeight::panel_section(
            projects_section::content_height(&vm.projects),
            vm.projects.is_open,
        );
        let available = (panel.height - 2.0 * size::LINE).max(0.0);
        let heights =
            share_panel_height(available, &[accounts, agents, projects]);
        let accounts_share = heights.first().copied().unwrap_or(0.0);
        let agents_share = heights.get(1).copied().unwrap_or(0.0);

        column![
            container(accounts_section(vm.accounts.clone(), palette))
                .width(Length::Fill)
                .height(accounts_share)
                .clip(true),
            divider::horizontal(palette),
            container(agents_section(vm.agents.clone(), palette))
                .width(Length::Fill)
                .height(agents_share)
                .clip(true),
            divider::horizontal(palette),
            container(projects_section::view(vm.projects.clone(), palette))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    })
    .into()
}

/// A muted line in place of rows, such as "Looking for agents…".
pub(super) fn note_text<'a>(
    content: impl text::IntoFragment<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    container(small_text(content, palette.text_muted()))
        .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3]))
        .into()
}

/// Height of a [`note_text`] line.
pub(super) fn note_height() -> f32 {
    typography::SMALL.line_height() + 2.0 * spacing::SPACE_1
}

/// The Accounts section: Refresh in the header, All accounts and a row
/// for each account. Its `+` comes with Add account.
fn accounts_section<'a>(
    vm: AccountsSectionViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let count = vm.accounts.len();
    let mut rows = Column::new().width(Length::Fill);
    if let Some(all) = vm.all {
        rows = rows.push(all_row(
            all,
            Key::AgentsAllAccounts,
            icons::ALL_ACCOUNTS,
            AgentsIntent::SelectAllAccounts,
            palette,
        ));
    }
    for account in vm.accounts {
        rows = rows.push(account_row(account, palette));
    }
    if let Some(note) = vm.note {
        rows = rows.push(note_text(note, palette));
    }

    let mut section =
        PanelSection::new(i18n::t(Key::AgentsSectionAccounts), palette)
            .open(vm.is_open)
            .action(refresh_button(palette))
            .body(section_body(rows, palette))
            .on_toggle(AgentsIntent::ToggleAccountsSection);
    if !vm.is_open {
        section = section.count(count);
    }

    section.into()
}

/// The Profiles section: Refresh and `+` in the header, All profiles and
/// a row for each profile.
fn agents_section<'a>(
    vm: AgentsSectionViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let add = IconButton::new(
        svg::Handle::from_memory(icons::PLUS),
        i18n::t(Key::AgentsAddProfile),
        palette,
    )
    .size(IconButtonSize::Small)
    .tone(IconTone::Accent)
    .on_press_maybe(vm.can_add.then_some(AgentsIntent::OpenAddProfile));

    let count = vm.profiles.len();
    let mut rows = Column::new().width(Length::Fill);
    if let Some(all) = vm.all {
        rows = rows.push(all_row(
            all,
            Key::AgentsAllProfiles,
            icons::ALL_PROFILES,
            AgentsIntent::SelectAllProfiles,
            palette,
        ));
    }
    for profile in vm.profiles {
        rows = rows.push(profile_row(profile, palette));
    }
    if let Some(note) = vm.note {
        rows = rows.push(note_text(note, palette));
    }

    let mut section =
        PanelSection::new(i18n::t(Key::AgentsSectionProfiles), palette)
            .open(vm.is_open)
            .action(refresh_button(palette))
            .action(add)
            .body(section_body(rows, palette))
            .on_toggle(AgentsIntent::ToggleAgentsSection);
    if !vm.is_open {
        section = section.count(count);
    }

    section.into()
}

/// The small Refresh button of a section header: it reads everything
/// again.
fn refresh_button(palette: &UiPalette) -> IconButton<AgentsIntent> {
    IconButton::new(
        svg::Handle::from_memory(icons::REFRESH),
        i18n::t(Key::AgentsRefreshAll),
        palette,
    )
    .size(IconButtonSize::Small)
    .on_press(AgentsIntent::Refresh)
}

/// The rows of a section, scrolling inside its share of the panel.
fn section_body<'a>(
    rows: Column<'a, AgentsIntent>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    scrollable(rows)
        .width(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette))
        .into()
}

/// Height of the body of the Accounts section: All accounts, the account
/// rows and the note.
///
/// Counted from the rows because iced cannot measure a widget before the
/// left panel shares its height between the sections.
fn accounts_content_height(vm: &AccountsSectionViewModel) -> f32 {
    let all = vm.all.map_or(0.0, |_| size::ROW);
    let rows = vm.accounts.len() as f32 * size::ROW;
    let note = vm.note.map_or(0.0, |_| note_height());

    all + rows + note
}

/// Height of the body of the Profiles section: All profiles, the profile
/// rows and the note.
fn agents_content_height(vm: &AgentsSectionViewModel) -> f32 {
    let all = vm.all.map_or(0.0, |_| size::ROW);
    let rows = vm.profiles.len() as f32 * size::ROW;
    let note = vm.note.map_or(0.0, |_| note_height());

    all + rows + note
}

/// All accounts or All profiles: the icon, the title and how many rows
/// follow it. A press shows its page.
fn all_row<'a>(
    vm: AllRowViewModel,
    title: Key,
    icon: &'static [u8],
    on_press: AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    TreeRow::new(i18n::t(title), palette)
        .icon(svg::Handle::from_memory(icon), None)
        .count(vm.count)
        .selected(vm.is_selected)
        .on_press(on_press)
        .into()
}

/// An account: the logo of its provider, its name and its number. A press
/// shows it.
fn account_row<'a>(
    vm: AccountRowViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let icon = svg::Handle::from_memory(icons::provider_icon(vm.provider));
    let mut row = TreeRow::new(vm.name, palette)
        .icon(icon, None)
        .count_tone(vm.tone)
        .selected(vm.is_selected)
        .on_press(AgentsIntent::SelectAccount(vm.account));
    if let Some(number) = vm.number {
        row = row.state(number);
    }

    row.into()
}

/// A profile: the logo of its agent, its name and its number. A press
/// shows its dashboard; a right press opens its menu at the pointer, the
/// same as "⋯" of the dashboard.
fn profile_row<'a>(
    vm: ProfileRowViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let icon = svg::Handle::from_memory(icons::agent_icon(vm.icon));
    let menu = AgentsIntent::OpenProfileMenu(vm.profile.clone());
    let mut row = TreeRow::new(vm.name, palette)
        .icon(icon, None)
        .count_tone(vm.tone)
        .selected(vm.is_selected)
        .on_press(AgentsIntent::SelectProfile(vm.profile));
    if let Some(number) = vm.number {
        row = row.state(number);
    }

    mouse_area(row).on_right_press(menu).into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_agents::{
        Account, AccountIdentity, AccountKind, AccountLogin, AgentId, FoundIn,
        Profile, Provider,
    };
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::session_row::SessionTone;

    use super::*;
    use crate::model::ProjectsSectionViewModel;
    use crate::testing::right_click;

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

    fn accounts() -> AccountsSectionViewModel {
        AccountsSectionViewModel {
            is_open: true,
            all: Some(AllRowViewModel {
                count: 1,
                is_selected: false,
            }),
            accounts: vec![AccountRowViewModel {
                account: chatgpt(),
                is_selected: false,
                provider: "OpenAI",
                name: String::from("OpenAI · you@example.com"),
                number: Some(String::from("42 % used")),
                tone: SessionTone::Muted,
            }],
            note: None,
        }
    }

    fn agents() -> AgentsSectionViewModel {
        AgentsSectionViewModel {
            is_open: true,
            all: Some(AllRowViewModel {
                count: 1,
                is_selected: false,
            }),
            profiles: vec![ProfileRowViewModel {
                profile: codex(),
                is_selected: false,
                icon: "codex",
                name: String::from("Codex"),
                number: Some(String::from("3 760 left")),
                tone: SessionTone::Muted,
            }],
            can_add: true,
            note: None,
        }
    }

    fn panel(agents: AgentsSectionViewModel) -> LeftPanelViewModel<'static> {
        LeftPanelViewModel {
            accounts: accounts(),
            agents,
            projects: ProjectsSectionViewModel {
                is_open: true,
                count: 0,
                search: None,
                projects: Vec::new(),
                note: None,
            },
        }
    }

    /// Width of the panel under test.
    const WIDTH: f32 = 280.0;
    /// From the right edge of the panel to the right edge of the last
    /// header button: the section padding and the header padding.
    const HEADER_END: f32 = spacing::SPACE_3 + spacing::SPACE_1;

    fn panel_view<'a>(
        vm: LeftPanelViewModel<'a>,
        themes: &'a ThemeManager,
    ) -> Element<'a, AgentsIntent> {
        container(view(LeftPanelProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }))
        .width(WIDTH)
        .into()
    }

    fn pressed(vm: LeftPanelViewModel<'_>, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(panel_view(vm, &themes));

        ui.click(label).expect("the label should show");

        ui.into_messages().collect()
    }

    /// What a press on the header button of the Profiles section
    /// `from_end` places from the right end sends: 0 is `+`, 1 Refresh.
    fn header_pressed(
        vm: LeftPanelViewModel<'_>,
        from_end: usize,
    ) -> Vec<AgentsIntent> {
        section_header_pressed(vm, Key::AgentsSectionProfiles, from_end)
    }

    /// What a press on the header button of the section titled `section`
    /// `from_end` places from the right end sends.
    fn section_header_pressed(
        vm: LeftPanelViewModel<'_>,
        section: Key,
        from_end: usize,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(panel_view(vm, &themes));
        let title = ui
            .find(i18n::t(section))
            .expect("the title should show")
            .bounds();
        let button = size::CONTROL_SM;
        let x = WIDTH
            - HEADER_END
            - button / 2.0
            - from_end as f32 * (button + spacing::SPACE_2);

        ui.point_at(iced::Point::new(x, title.center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn a_profile_row_shows_its_number_and_a_press_selects_it() {
        let themes = ThemeManager::new();
        let mut ui = simulator(panel_view(panel(agents()), &themes));

        ui.find("3 760 left").expect("the number should show");
        ui.click("Codex").expect("the profile should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![AgentsIntent::SelectProfile(codex())]);
    }

    /// Owner, issue 26: a right click on a profile row opens the same menu
    /// as "⋯" of its dashboard, at the pointer.
    #[test]
    fn a_right_click_on_a_profile_row_opens_its_menu() {
        let themes = ThemeManager::new();
        let mut ui = simulator(panel_view(panel(agents()), &themes));

        let row = ui.find("Codex").expect("the profile should show");
        ui.point_at(row.bounds().center());
        let _ = ui.simulate(right_click());

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![AgentsIntent::OpenProfileMenu(codex())]);
    }

    /// Spec Decision 1: the Accounts section starts with All accounts,
    /// then each account with its number and its word.
    #[test]
    fn all_accounts_and_an_account_row_show_their_pages() {
        let all = pressed(panel(agents()), i18n::t(Key::AgentsAllAccounts));
        let account = pressed(panel(agents()), "OpenAI · you@example.com");
        let themes = ThemeManager::new();
        let mut ui = simulator(panel_view(panel(agents()), &themes));

        ui.find("42 % used").expect("the number should show");
        assert_eq!(all, vec![AgentsIntent::SelectAllAccounts]);
        assert_eq!(account, vec![AgentsIntent::SelectAccount(chatgpt())]);
    }

    #[test]
    fn all_profiles_shows_its_page() {
        let all = pressed(panel(agents()), i18n::t(Key::AgentsAllProfiles));

        assert_eq!(all, vec![AgentsIntent::SelectAllProfiles]);
    }

    /// Until Add account is in (issue 09) the Accounts header has Refresh
    /// alone.
    #[test]
    fn the_accounts_header_has_refresh_alone_and_its_title_toggles_it() {
        let refreshed = section_header_pressed(
            panel(agents()),
            Key::AgentsSectionAccounts,
            0,
        );
        let beside = section_header_pressed(
            panel(agents()),
            Key::AgentsSectionAccounts,
            1,
        );
        let toggled =
            pressed(panel(agents()), i18n::t(Key::AgentsSectionAccounts));

        assert_eq!(refreshed, vec![AgentsIntent::Refresh]);
        assert_eq!(
            beside,
            vec![AgentsIntent::ToggleAccountsSection],
            "no second button: a press there lands on the header"
        );
        assert_eq!(toggled, vec![AgentsIntent::ToggleAccountsSection]);
    }

    #[test]
    fn the_header_buttons_refresh_and_add_a_profile() {
        let refreshed = header_pressed(panel(agents()), 1);
        let added = header_pressed(panel(agents()), 0);

        assert_eq!(refreshed, vec![AgentsIntent::Refresh]);
        assert_eq!(added, vec![AgentsIntent::OpenAddProfile]);
    }

    #[test]
    fn the_title_opens_and_closes_the_section() {
        let toggled =
            pressed(panel(agents()), i18n::t(Key::AgentsSectionProfiles));

        assert_eq!(toggled, vec![AgentsIntent::ToggleAgentsSection]);
    }

    #[test]
    fn before_the_first_read_the_note_shows_and_add_is_off() {
        let loading = AgentsSectionViewModel {
            all: None,
            profiles: Vec::new(),
            can_add: false,
            note: Some(i18n::t(Key::AgentsLoading)),
            ..agents()
        };

        let note = pressed(panel(loading.clone()), i18n::t(Key::AgentsLoading));
        let add = header_pressed(panel(loading), 0);

        assert!(note.is_empty());
        assert!(add.is_empty(), "add is off until the profiles are read");
    }

    #[test]
    fn a_closed_section_shows_no_rows() {
        let closed = AgentsSectionViewModel {
            is_open: false,
            ..agents()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(panel_view(panel(closed), &themes));

        assert!(ui.find("3 760 left").is_err());
    }

    #[test]
    fn the_body_height_counts_the_all_row_the_rows_and_the_note() {
        let loading = AgentsSectionViewModel {
            all: None,
            profiles: Vec::new(),
            note: Some("…"),
            ..agents()
        };
        let accounts_loading = AccountsSectionViewModel {
            all: None,
            accounts: Vec::new(),
            note: Some("…"),
            ..accounts()
        };

        assert_eq!(agents_content_height(&agents()), 2.0 * size::ROW);
        assert_eq!(agents_content_height(&loading), note_height());
        assert_eq!(accounts_content_height(&accounts()), 2.0 * size::ROW);
        assert_eq!(accounts_content_height(&accounts_loading), note_height());
    }

    /// The least width of the left panel: 16 % of the least window, 800.
    const LEAST_WIDTH: f32 = 128.0;

    #[test]
    fn at_the_least_panel_width_no_text_of_the_agents_section_overlaps() {
        let themes = ThemeManager::new();
        let panel = container(view(LeftPanelProps {
            vm: panel(agents()),
            theme: ThemeProps::new(themes.current()),
        }))
        .width(LEAST_WIDTH);
        let mut ui = simulator(panel);
        let mut at = |label: &str| {
            ui.find(label).expect("the text should show").bounds()
        };

        let title = at(i18n::t(Key::AgentsSectionProfiles));
        let name = at("Codex");
        let number = at("3 760 left");

        let buttons = LEAST_WIDTH
            - HEADER_END
            - 2.0 * size::CONTROL_SM
            - spacing::SPACE_2;
        assert!(title.x + title.width <= buttons, "{title:?}");
        assert!(name.x + name.width <= number.x, "{name:?} {number:?}");
        assert!(number.x + number.width <= LEAST_WIDTH, "{number:?}");
    }

    /// Only the icon name links an agent of `otty-agents` to its logo in
    /// the theme, so every known agent is checked to have a logo of its
    /// own, not the icon of the mode.
    #[test]
    fn every_known_agent_has_a_logo_of_its_own() {
        let agents = otty_agents::known_agents();
        let logos: Vec<&[u8]> = agents
            .iter()
            .map(|agent| icons::agent_icon(agent.descriptor().icon()))
            .collect();

        for (agent, logo) in agents.iter().zip(&logos) {
            let name = agent.descriptor().name();
            assert_ne!(*logo, icons::MODE_AGENTS, "{name} has no logo");
        }
        let mut distinct = logos.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), logos.len(), "two agents share a logo");
    }
}
