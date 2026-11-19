//! The statistics popup of a session or of a project (spec 2026-11-19,
//! Decision 6): the spinner while they are read, the reason when they
//! could not be, else the facts, the usage windows of a Codex session and
//! the usage block of the session or of the project.

use iced::widget::{Column, Id, column, container};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
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

/// Render the modal statistics dialog, as wide as a form; it scrolls when
/// it is taller than the window. Close and its cross close it; the frame
/// closes it on Esc.
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

/// The facts of a session, its usage windows for Codex, and its usage
/// block.
fn session_body<'a>(
    vm: SessionStatsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut body = Column::new().push(blocks::facts(vm.facts, palette));
    if !vm.limits.is_empty() {
        body = body.push(blocks::titled(
            i18n::t(Key::AgentsStatsLimits),
            blocks::facts(vm.limits, palette),
            palette,
        ));
    }

    body.push(usage_block::view(vm.usage, palette))
        .spacing(spacing::SPACE_5)
        .width(Length::Fill)
        .into()
}

/// The facts of a project and its usage block.
fn project_body<'a>(
    vm: ProjectStatsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    column![
        blocks::facts(vm.facts, palette),
        usage_block::view(vm.usage, palette),
    ]
    .spacing(spacing::SPACE_5)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{
        FactViewModel, ProjectStatsViewModel, SessionStatsViewModel,
        StatsBodyViewModel, UsageBlockViewModel, UsageListsViewModel,
    };
    use crate::testing::usage_block;
    use crate::usage::{UsageList, UsagePeriod, UsageTab};

    fn dialog(body: StatsBodyViewModel) -> StatsDialogViewModel {
        StatsDialogViewModel {
            title: String::from("Fix the tabs"),
            body,
        }
    }

    /// The usage block of a popup with `tabs`, its `open` list and Money
    /// off for `money_reason`.
    fn popup_usage(
        tabs: &[UsageTab],
        open: Option<UsageList>,
        money_reason: Option<&str>,
    ) -> UsageBlockViewModel {
        UsageBlockViewModel {
            tabs: tabs.to_vec(),
            lists: Some(UsageListsViewModel {
                open,
                money_reason: money_reason.map(String::from),
            }),
            ..usage_block()
        }
    }

    fn session(usage: UsageBlockViewModel) -> StatsDialogViewModel {
        dialog(StatsBodyViewModel::Session(SessionStatsViewModel {
            facts: vec![
                FactViewModel::new("Started", "Nov 16, 2026, 10:00".into()),
                FactViewModel::new(
                    "Account",
                    String::from("OpenAI · you@example.com"),
                ),
            ],
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

    /// Spec 2026-11-19, Decision 6: the session popup has its facts with
    /// the Account line, its usage windows and the usage block with By
    /// model | By account, and no By project or By session.
    #[test]
    fn a_session_shows_its_facts_limits_and_usage_by_model_or_account() {
        let vm = session(popup_usage(&UsageTab::SESSION_POPUP, None, None));

        let labels = [
            "Started",
            "Nov 16, 2026, 10:00",
            "Account",
            "OpenAI · you@example.com",
            i18n::t(Key::AgentsStatsLimits),
            "12 % → 31 %",
            i18n::t(Key::AgentsUsage),
            i18n::t(Key::AgentsTopByModel),
            i18n::t(Key::AgentsByAccount),
            "gpt-5.5-codex",
        ];
        assert_eq!(shown(vm.clone(), &labels), labels.map(String::from));
        assert!(shown(vm, &[i18n::t(Key::AgentsTopByProject)]).is_empty());
    }

    #[test]
    fn a_project_shows_its_facts_and_its_usage_by_profile() {
        let vm = dialog(StatsBodyViewModel::Project(ProjectStatsViewModel {
            facts: vec![FactViewModel::new("Sessions", String::from("7"))],
            usage: UsageBlockViewModel {
                tab: UsageTab::ByProfile,
                ..popup_usage(&UsageTab::PROJECT_POPUP, None, None)
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
        let closed = session(popup_usage(&UsageTab::SESSION_POPUP, None, None));
        let mut closed_ui = ui(closed, &themes);
        closed_ui
            .click(UsagePeriod::SevenDays.label())
            .expect("the period field shows");
        let toggled: Vec<AgentsIntent> = closed_ui.into_messages().collect();

        let open = session(popup_usage(
            &UsageTab::SESSION_POPUP,
            Some(UsageList::Period),
            None,
        ));
        let mut open_ui = ui(open, &themes);
        open_ui
            .click(UsagePeriod::ThirtyDays.label())
            .expect("the list shows");
        let picked: Vec<AgentsIntent> = open_ui.into_messages().collect();

        assert_eq!(toggled, [AgentsIntent::OpenPeriodMenu]);
        assert_eq!(picked, [AgentsIntent::PickPeriod(UsagePeriod::ThirtyDays)]);
    }

    #[test]
    fn money_off_in_the_popup_shows_its_reason_under_the_title() {
        let reason = "Paid by a subscription";
        let vm =
            session(popup_usage(&UsageTab::SESSION_POPUP, None, Some(reason)));

        let line = i18n::filled(Key::TplAgentsMoneyOff, &[("reason", reason)]);
        let found = shown(vm, &[&line]);

        assert_eq!(found, [line]);
    }
}
