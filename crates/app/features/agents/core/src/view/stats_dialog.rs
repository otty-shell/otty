//! The statistics popup of a session or of a project (spec 2026-11-19,
//! Decision 6): the spinner while they are read, the reason when they
//! could not be, else the facts, the usage windows of a session whose agent
//! writes them and the usage block of the session or of the project. Each
//! part is a kit card, as on the pages (issue 25 of spec
//! 2026-11-23-accounts-apart-from-profiles).

use iced::widget::{Column, Id, column, container};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::card::Card;
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::spinner::Spinner;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::super::event::AgentsIntent;
use super::super::model::{
    ProjectStatsViewModel, SessionStatsViewModel, StatsBodyViewModel,
    StatsDialogViewModel,
};
use super::{blocks, usage_block};

/// The id of the spinner of the dialog while the statistics are read.
pub(crate) const STATS_SPINNER_ID: &str = "agents_stats_spinner";

/// Props for the statistics dialog.
pub(crate) struct StatsDialogProps<'a> {
    pub(crate) vm: StatsDialogViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal statistics dialog, as wide as a form; its body scrolls
/// when it is taller than the window. Close and its cross close it; the
/// frame closes it on Esc.
pub(crate) fn view(props: StatsDialogProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let body = match vm.body {
        StatsBodyViewModel::Reading => reading(palette),
        StatsBodyViewModel::Failed(reason) => {
            small_text(reason, palette.text_danger()).into()
        },
        StatsBodyViewModel::Session(session) => session_body(session, palette),
        StatsBodyViewModel::Project(project) => project_body(project, palette),
    };
    let close = Button::new(i18n::t(Key::ButtonClose), palette)
        .kind(ButtonKind::Secondary)
        .on_press(AgentsIntent::CloseStats);

    Dialog::new(vm.title, palette)
        .width(size::DIALOG_WIDE)
        .on_close(AgentsIntent::CloseStats, i18n::t(Key::ButtonClose))
        .body(body)
        .action(close)
        .into()
}

/// The kit spinner with a note under it, in the middle of the dialog.
fn reading<'a>(palette: &UiPalette) -> Element<'a, AgentsIntent> {
    let spinner =
        container(Spinner::new(palette)).id(Id::new(STATS_SPINNER_ID));

    column![
        spinner,
        small_text(i18n::t(Key::AgentsStatsReading), palette.text_muted()),
    ]
    .spacing(spacing::SPACE_3)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .into()
}

/// The cards of a session: Session info with its facts, Limits with its
/// usage windows when its agent writes them, and its usage block.
fn session_body<'a>(
    vm: SessionStatsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let info = Card::new(blocks::facts(vm.facts, palette), palette)
        .title(i18n::t(Key::AgentsSessionInfo));

    let mut body = Column::new().push(info);
    if !vm.limits.is_empty() {
        body = body.push(
            Card::new(blocks::facts(vm.limits, palette), palette)
                .title(i18n::t(Key::AgentsStatsLimits)),
        );
    }

    body.push(usage_block::card(vm.usage, palette))
        .spacing(blocks::BLOCK_GAP)
        .width(Length::Fill)
        .into()
}

/// The cards of a project: Project info with its facts, and its usage
/// block.
fn project_body<'a>(
    vm: ProjectStatsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let info = Card::new(blocks::facts(vm.facts, palette), palette)
        .title(i18n::t(Key::AgentsProjectInfo));

    column![info, usage_block::card(vm.usage, palette)]
        .spacing(blocks::BLOCK_GAP)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{
        FactViewModel, PeriodPicker, ProjectStatsViewModel,
        SessionStatsViewModel, StatsBodyViewModel, UsageBlockViewModel,
    };
    use crate::testing::usage_block;
    use crate::usage::{UsagePeriod, UsageTab};

    fn dialog(body: StatsBodyViewModel) -> StatsDialogViewModel {
        StatsDialogViewModel {
            title: String::from("Fix the tabs"),
            body,
        }
    }

    /// The usage block of a popup with `tabs`, its list of the periods
    /// open when `is_open`.
    fn popup_usage(tabs: &[UsageTab], is_open: bool) -> UsageBlockViewModel {
        UsageBlockViewModel {
            tabs: tabs.to_vec(),
            period_picker: PeriodPicker::List { is_open },
            ..usage_block()
        }
    }

    fn session(usage: UsageBlockViewModel) -> StatsDialogViewModel {
        dialog(StatsBodyViewModel::Session(SessionStatsViewModel {
            facts: vec![FactViewModel::new(
                "Started",
                "Nov 16, 2026, 10:00".into(),
            )],
            limits: vec![FactViewModel::new(
                "5 hours",
                String::from("12 % → 31 %"),
            )],
            usage,
        }))
    }

    fn ui(
        vm: StatsDialogViewModel,
        themes: &ThemeManager,
    ) -> iced_test::Simulator<'_, AgentsIntent> {
        iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1000.0, 1200.0),
            view(StatsDialogProps {
                vm,
                theme: ThemeProps::new(themes.current()),
            }),
        )
    }

    /// The labels of `vm` that show, out of `labels`.
    fn shown(vm: StatsDialogViewModel, labels: &[&str]) -> Vec<String> {
        let themes = ThemeManager::new();
        let mut ui = ui(vm, &themes);

        labels
            .iter()
            .filter(|label| ui.find(**label).is_ok())
            .map(|label| label.to_string())
            .collect()
    }

    #[test]
    fn while_reading_the_dialog_shows_the_spinner() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(StatsDialogProps {
            vm: dialog(StatsBodyViewModel::Reading),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(iced::widget::Id::new(STATS_SPINNER_ID))
            .expect("the spinner shows");
        ui.find("Fix the tabs").expect("the title shows");
        ui.find(i18n::t(Key::AgentsStatsReading))
            .expect("the note shows");
    }

    #[test]
    fn a_failure_shows_its_reason_and_close_closes_the_dialog() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(StatsDialogProps {
            vm: dialog(StatsBodyViewModel::Failed(String::from(
                "Could not read: no file",
            ))),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Could not read: no file")
            .expect("the reason shows");
        assert!(ui.find(iced::widget::Id::new(STATS_SPINNER_ID)).is_err());
        ui.click(i18n::t(Key::ButtonClose)).expect("Close shows");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![AgentsIntent::CloseStats]);
    }

    /// Spec 2026-11-19, Decision 6: the session popup has its facts, its
    /// usage windows and the usage block with By model, and no By project
    /// or By session. Spec 2026-11-23, Decision 1: no Account line and no
    /// By account.
    #[test]
    fn a_session_shows_its_facts_limits_and_usage_by_model() {
        let vm = session(popup_usage(&UsageTab::SESSION_POPUP, false));

        let labels = [
            "Started",
            "Nov 16, 2026, 10:00",
            i18n::t(Key::AgentsStatsLimits),
            "12 % → 31 %",
            i18n::t(Key::AgentsUsage),
            i18n::t(Key::AgentsTopByModel),
            "gpt-5.5-codex",
        ];
        assert_eq!(shown(vm.clone(), &labels), labels.map(String::from));
        let gone = [i18n::t(Key::AgentsTopByProject), "Account", "By account"];
        assert!(shown(vm, &gone).is_empty());
    }

    #[test]
    fn a_project_shows_its_facts_and_its_usage_by_profile() {
        let vm = dialog(StatsBodyViewModel::Project(ProjectStatsViewModel {
            facts: vec![FactViewModel::new("Sessions", String::from("7"))],
            usage: UsageBlockViewModel {
                tab: UsageTab::ByProfile,
                ..popup_usage(&UsageTab::PROJECT_POPUP, false)
            },
        }));

        let labels = [
            "Sessions",
            "7",
            i18n::t(Key::AgentsStatsByProfile),
            i18n::t(Key::AgentsProfile),
            "gpt-5.5-codex",
        ];
        assert_eq!(shown(vm, &labels), labels.map(String::from));
    }

    /// No menu of the mode shows over a dialog: the period list opens
    /// under its field in the popup, and a press picks.
    #[test]
    fn the_period_list_opens_under_its_field_and_a_press_picks() {
        let themes = ThemeManager::new();
        let closed = session(popup_usage(&UsageTab::SESSION_POPUP, false));
        let mut closed_ui = ui(closed, &themes);
        closed_ui
            .click(UsagePeriod::SevenDays.label())
            .expect("the period field shows");
        let toggled: Vec<AgentsIntent> = closed_ui.into_messages().collect();

        let open = session(popup_usage(&UsageTab::SESSION_POPUP, true));
        let mut open_ui = ui(open, &themes);
        open_ui
            .click(UsagePeriod::ThirtyDays.label())
            .expect("the list shows");
        let picked: Vec<AgentsIntent> = open_ui.into_messages().collect();

        assert_eq!(toggled, [AgentsIntent::OpenPeriodMenu]);
        assert_eq!(picked, [AgentsIntent::PickPeriod(UsagePeriod::ThirtyDays)]);
    }

    /// Issue 14 of spec 2026-11-23: the table of a popup has the Tokens
    /// and the Money columns and fits the dialog, so it never scrolls
    /// sideways.
    #[test]
    fn the_table_of_a_popup_has_tokens_and_money_and_fits_the_dialog() {
        let themes = ThemeManager::new();
        let vm = session(popup_usage(&UsageTab::SESSION_POPUP, false));
        let mut ui = ui(vm, &themes);
        let mut at =
            |label: &str| ui.find(label).expect("the column shows").bounds();

        let tokens = at(i18n::t(Key::AgentsTokens));
        let money = at(i18n::t(Key::AgentsNumberMoney));
        let share = at(i18n::t(Key::AgentsColumnShare));

        let dialog_end = (1000.0 + size::DIALOG_WIDE) / 2.0;
        assert!(tokens.x + tokens.width < money.x, "{tokens:?} {money:?}");
        assert!(
            share.x + share.width < dialog_end,
            "{share:?} ends inside the dialog at {dialog_end}"
        );
    }

    /// Side padding of a kit card.
    const CARD_PADDING: f32 = spacing::SPACE_5;
    /// The least room between the last line of a card and the title of the
    /// card under it: the bottom padding of the card and the gap.
    const CARD_AFTER: f32 = CARD_PADDING + spacing::SPACE_5 - 0.5;

    /// Issue 25 of spec 2026-11-23-accounts-apart-from-profiles: the parts
    /// of the session popup are kit cards, as on the pages: Session info
    /// with the facts, Limits and Usage, whose period sits at the right
    /// end of its title row.
    #[test]
    fn the_parts_of_a_session_popup_are_cards() {
        let themes = ThemeManager::new();
        let vm = session(popup_usage(&UsageTab::SESSION_POPUP, false));
        let mut ui = ui(vm, &themes);
        let mut at =
            |label: &str| ui.find(label).expect("the text shows").bounds();

        let body_left = at("Fix the tabs").x;
        let info = at(i18n::t(Key::AgentsSessionInfo));
        let started = at("Started");
        let limits = at(i18n::t(Key::AgentsStatsLimits));
        let window = at("5 hours");
        let usage = at(i18n::t(Key::AgentsUsage));
        let period = at(UsagePeriod::SevenDays.label());

        let inside = body_left + CARD_PADDING;
        for part in [info, started, limits, window, usage] {
            assert_eq!(part.x, inside, "{part:?} lies inside its card");
        }
        assert!(started.y >= info.y + info.height, "{started:?}");
        assert!(limits.y >= started.y + started.height + CARD_AFTER);
        assert!(usage.y >= window.y + window.height + CARD_AFTER);
        assert!(
            (period.center_y() - usage.center_y()).abs() < size::CONTROL_MD,
            "the period {period:?} is in the title row of {usage:?}"
        );
        let dialog_end = (1000.0 + size::DIALOG_WIDE) / 2.0;
        assert!(period.x + period.width < dialog_end - 2.0 * CARD_PADDING);
    }

    /// Issue 25: the parts of the project popup are kit cards: Project
    /// info with the facts, and Usage.
    #[test]
    fn the_parts_of_a_project_popup_are_cards() {
        let themes = ThemeManager::new();
        let mut ui = ui(project_popup(true), &themes);
        let mut at =
            |label: &str| ui.find(label).expect("the text shows").bounds();

        let body_left = at("Fix the tabs").x;
        let info = at(i18n::t(Key::AgentsProjectInfo));
        let sessions = at("Sessions");
        let usage = at(i18n::t(Key::AgentsUsage));
        let tab = at(i18n::t(Key::AgentsStatsByProfile));

        let inside = body_left + CARD_PADDING;
        for part in [info, sessions, usage] {
            assert_eq!(part.x, inside, "{part:?} lies inside its card");
        }
        assert!(usage.y >= sessions.y + sessions.height + CARD_AFTER);
        assert!(tab.x >= inside, "{tab:?} inside the card of Usage");
    }

    /// The least height of a whole line of text of a table cell.
    const LINE_LEAST: f32 = 12.0;

    /// The project popup with its usage block by profile, with the rows
    /// of the usage block or with none.
    fn project_popup(has_rows: bool) -> StatsDialogViewModel {
        let mut usage = UsageBlockViewModel {
            tab: UsageTab::ByProfile,
            ..popup_usage(&UsageTab::PROJECT_POPUP, false)
        };
        if !has_rows {
            usage.rows.clear();
        }

        dialog(StatsBodyViewModel::Project(ProjectStatsViewModel {
            facts: vec![FactViewModel::new("Sessions", String::from("7"))],
            usage,
        }))
    }

    /// The bottom of the table of the popup, the line under its last row
    /// included, found from the text `last` of that row; and the top of
    /// the text of Close under it.
    fn table_bottom_and_close(
        vm: StatsDialogViewModel,
        last: &str,
    ) -> (f32, f32) {
        let themes = ThemeManager::new();
        let mut ui = ui(vm, &themes);

        let last = ui.find(last).expect("the last row shows").bounds();
        let close = ui
            .find(i18n::t(Key::ButtonClose))
            .expect("Close shows")
            .bounds();

        assert!(last.height >= LINE_LEAST, "the row is whole: {last:?}");
        let bottom = last.center_y() + size::TABLE_ROW / 2.0 + size::LINE;

        (bottom, close.y)
    }

    /// Issue 19 of spec 2026-11-23: an empty table of the popup shows its
    /// whole row, and under it the same room as under a table with rows,
    /// before Close at the bottom of the card.
    #[test]
    fn an_empty_table_keeps_the_room_under_it_as_a_table_with_rows() {
        let (rows_bottom, rows_close) =
            table_bottom_and_close(project_popup(true), "gpt-5.4");
        let (empty_bottom, empty_close) = table_bottom_and_close(
            project_popup(false),
            i18n::t(Key::AgentsTopEmpty),
        );

        let rows_room = rows_close - rows_bottom;
        let empty_room = empty_close - empty_bottom;
        assert!(rows_room >= spacing::SPACE_5, "{rows_room}");
        assert!(
            (empty_room - rows_room).abs() < 1.0,
            "under the empty row {empty_room}, under the rows {rows_room}"
        );
    }
}
