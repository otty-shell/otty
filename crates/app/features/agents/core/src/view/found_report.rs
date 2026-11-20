//! The Found by OTTY report (spec Decision 2): a wide dialog with what OTTY
//! found, the profiles over the accounts, then what the accounts and the
//! profiles need from the user, each need with its one button. Done closes
//! it; the frame closes it on Esc.

use iced::widget::Column;
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::data_table::{
    ColumnAlign, DataTable, TableCell, TableColumn,
};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::status_badge::StatusTone;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::spacing;

use super::super::event::AgentsIntent;
use super::super::model::{
    FoundAccountRowViewModel, FoundProfileRowViewModel, FoundReportViewModel,
};
use super::{blocks, needs};

/// Width of the report (`width={760}` of the prototype).
const REPORT_WIDTH: f32 = 760.0;

/// Props for the Found by OTTY report.
pub(crate) struct FoundReportProps<'a> {
    pub(crate) vm: FoundReportViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal Found by OTTY report.
pub(crate) fn view(props: FoundReportProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut body = Column::new()
        .push(small_text(vm.intro, palette.text_muted()))
        .push(profiles_table(vm.profiles, palette))
        .push(accounts_table(vm.accounts, palette))
        .spacing(spacing::SPACE_4)
        .width(Length::Fill);
    if !vm.account_needs.is_empty() {
        body = body.push(blocks::titled(
            i18n::t(Key::AgentsAccountsNeedYou),
            needs::view(
                vm.account_needs,
                AgentsIntent::FoundReportNeed,
                palette,
            ),
            palette,
        ));
    }
    if !vm.profile_needs.is_empty() {
        body = body.push(blocks::titled(
            i18n::t(Key::AgentsProfilesNeedYou),
            needs::view(
                vm.profile_needs,
                AgentsIntent::FoundReportNeed,
                palette,
            ),
            palette,
        ));
    }

    let done = Button::new(i18n::t(Key::AgentsFoundDone), palette)
        .kind(ButtonKind::Primary)
        .on_press(AgentsIntent::CloseFoundReport);

    Dialog::new(vm.title, palette)
        .width(REPORT_WIDTH)
        .body(body)
        .action(done)
        .into()
}

/// The profiles: Agent, Profile, Version, Folder and Accounts, as tall as
/// its rows.
fn profiles_table<'a>(
    rows: Vec<FoundProfileRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let columns = vec![
        report_column(Key::AgentsAgent, 2, 110),
        report_column(Key::AgentsProfile, 2, 110),
        report_column(Key::AgentsColumnVersion, 1, 70),
        report_column(Key::AgentsColumnFolder, 3, 160),
        report_column(Key::AgentsColumnAccounts, 1, 70).align(ColumnAlign::End),
    ];
    let count = rows.len();

    let mut table = DataTable::new(columns, palette);
    for row in rows {
        table = table.row_without_menu(vec![
            TableCell::plain(row.agent),
            TableCell::plain(row.profile),
            TableCell::muted_mono(row.version),
            TableCell::muted_mono(row.folder),
            TableCell::number(row.accounts),
        ]);
    }

    blocks::rows_tall(table, count)
}

/// The accounts: Account, Found in, Reads and Now, as tall as its rows.
fn accounts_table<'a>(
    rows: Vec<FoundAccountRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let columns = vec![
        report_column(Key::AgentsFieldAccount, 3, 170),
        report_column(Key::AgentsColumnFoundIn, 2, 140),
        report_column(Key::AgentsColumnReads, 1, 80),
        report_column(Key::AgentsColumnNow, 2, 120).align(ColumnAlign::End),
    ];
    let count = rows.len();

    let mut table = DataTable::new(columns, palette);
    for row in rows {
        table = table.row_without_menu(vec![
            TableCell::plain(row.account),
            TableCell::muted_mono(row.found_in),
            TableCell::badge(row.reads, StatusTone::Ok),
            TableCell::number(row.now),
        ]);
    }

    blocks::rows_tall(table, count)
}

/// A column of the report named `key`, `share` parts of the width and at
/// least `min_width` wide, that does not sort.
fn report_column(key: Key, share: u16, min_width: u16) -> TableColumn {
    TableColumn::new(i18n::t(key), share)
        .min_width(min_width)
        .sortable(false)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_agents::{AgentId, Profile};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{NeedFixViewModel, NeedViewModel};
    use crate::need::NeedAction;

    fn codex() -> Profile {
        Profile::default_for(
            AgentId::new("codex"),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        )
    }

    fn report() -> FoundReportViewModel {
        FoundReportViewModel {
            title: "Found by OTTY",
            intro: String::from("Profiles found: 1. Accounts found: 1."),
            profiles: vec![FoundProfileRowViewModel {
                agent: String::from("Claude Code"),
                profile: String::from("Work"),
                version: String::from("2.1.328"),
                folder: String::from("~/.claude-work"),
                accounts: String::from("1"),
            }],
            accounts: vec![FoundAccountRowViewModel {
                account: String::from("Anthropic · dev@example.com"),
                found_in: String::from("~/.claude-work"),
                reads: "Limits",
                now: String::from("42 %"),
            }],
            account_needs: Vec::new(),
            profile_needs: vec![NeedViewModel {
                place: String::from("Codex"),
                title: String::from("needs login"),
                line: String::from("Nobody is logged in."),
                fix: Some(NeedFixViewModel {
                    label: String::from("Log in"),
                    action: Some(NeedAction::LogIn(codex())),
                }),
            }],
        }
    }

    #[test]
    fn the_report_shows_what_was_found_and_sends_its_buttons() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(FoundReportProps {
            vm: report(),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Found by OTTY").expect("the title should show");
        ui.find("~/.claude-work").expect("the folder should show");
        ui.find("2.1.328").expect("the version should show");
        ui.find("Anthropic · dev@example.com")
            .expect("the account should show");
        ui.find(i18n::t(Key::AgentsProfilesNeedYou))
            .expect("the profile needs should show");
        ui.find("Codex · needs login")
            .expect("the need should show");
        ui.click("Log in").expect("the need's button should show");
        ui.click(i18n::t(Key::AgentsFoundDone))
            .expect("Done should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                AgentsIntent::FoundReportNeed(NeedAction::LogIn(codex())),
                AgentsIntent::CloseFoundReport,
            ]
        );
    }
}
