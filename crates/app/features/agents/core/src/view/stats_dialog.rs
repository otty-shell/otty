//! The statistics dialog of a session or of a project: the spinner while
//! they are read, the reason when they could not be, else the facts, the
//! tokens of each model, the usage windows of a Codex session and the
//! profiles of a project (spec Decision 14).

use iced::widget::{Column, Id, column, container};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::data_table::{DataTable, TableCell};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::spinner::Spinner;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::super::event::AgentsIntent;
use super::super::model::{
    ModelsTableViewModel, ProfilesTableViewModel, ProjectStatsViewModel,
    SessionStatsViewModel, StatsBodyViewModel, StatsDialogViewModel,
};
use super::blocks;

/// The id of the spinner of the dialog while the statistics are read.
pub(crate) const STATS_SPINNER_ID: &str = "agents_stats_spinner";

/// Props for the statistics dialog.
pub(crate) struct StatsDialogProps<'a> {
    pub(crate) vm: StatsDialogViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal statistics dialog, as wide as a form. Close and its
/// cross close it; the frame closes it on Esc.
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

/// The facts of a session, its tokens by model and, for Codex, its usage
/// windows.
fn session_body<'a>(
    vm: SessionStatsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut body = Column::new().push(blocks::facts(vm.facts, palette)).push(
        blocks::titled(
            i18n::t(Key::AgentsStatsTokensByModel),
            models_table(vm.models, palette),
            palette,
        ),
    );
    if !vm.limits.is_empty() {
        body = body.push(blocks::titled(
            i18n::t(Key::AgentsStatsLimits),
            blocks::facts(vm.limits, palette),
            palette,
        ));
    }

    body.spacing(spacing::SPACE_5).width(Length::Fill).into()
}

/// The facts of a project, its tokens by model and its profiles.
fn project_body<'a>(
    vm: ProjectStatsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    column![
        blocks::facts(vm.facts, palette),
        blocks::titled(
            i18n::t(Key::AgentsStatsTokensByModel),
            models_table(vm.models, palette),
            palette,
        ),
        blocks::titled(
            i18n::t(Key::AgentsStatsByProfile),
            profiles_table(vm.profiles, palette),
            palette,
        ),
    ]
    .spacing(spacing::SPACE_5)
    .width(Length::Fill)
    .into()
}

/// The tokens of each model: Model, In, Cached, Out, Total and, for an
/// API key profile, Cost; a muted line when the files have no tokens. The
/// table is as tall as its rows, so it never scrolls.
fn models_table<'a>(
    vm: ModelsTableViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    if vm.rows.is_empty() {
        return small_text(
            i18n::t(Key::AgentsStatsNoTokens),
            palette.text_muted(),
        )
        .into();
    }

    let mut columns = vec![
        blocks::name_column(Key::AgentsColumnModel),
        blocks::number_column(Key::AgentsColumnIn),
        blocks::number_column(Key::AgentsColumnCached),
        blocks::number_column(Key::AgentsColumnOut),
        blocks::number_column(Key::AgentsColumnTotal),
    ];
    if vm.has_cost {
        columns.push(blocks::number_column(Key::AgentsColumnCost));
    }
    let count = vm.rows.len();
    let mut table = DataTable::new(columns, palette);
    for row in vm.rows {
        let mut cells = vec![
            TableCell::plain(row.name),
            TableCell::muted(row.input),
            TableCell::muted(row.cached),
            TableCell::muted(row.output),
            TableCell::number(row.total),
        ];
        if vm.has_cost {
            cells.push(TableCell::number(row.cost));
        }
        table = table.row_without_menu(cells);
    }

    blocks::rows_tall(table, count)
}

/// The sessions and tokens of each profile of a project, with Cost when
/// one of them pays with an API key.
fn profiles_table<'a>(
    vm: ProfilesTableViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut columns = vec![
        blocks::name_column(Key::AgentsColumnProfile),
        blocks::number_column(Key::AgentsSessions),
        blocks::number_column(Key::AgentsTokens),
    ];
    if vm.has_cost {
        columns.push(blocks::number_column(Key::AgentsColumnCost));
    }
    let count = vm.rows.len();
    let mut table = DataTable::new(columns, palette);
    for row in vm.rows {
        let mut cells = vec![
            TableCell::plain(row.name),
            TableCell::muted(row.sessions),
            TableCell::number(row.tokens),
        ];
        if vm.has_cost {
            cells.push(TableCell::number(row.cost));
        }
        table = table.row_without_menu(cells);
    }

    blocks::rows_tall(table, count)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{
        FactViewModel, ModelRowViewModel, ModelsTableViewModel,
        ProfileStatsRowViewModel, ProfilesTableViewModel,
        ProjectStatsViewModel, SessionStatsViewModel, StatsBodyViewModel,
    };

    fn dialog(body: StatsBodyViewModel) -> StatsDialogViewModel {
        StatsDialogViewModel {
            title: String::from("Fix the tabs"),
            body,
        }
    }

    fn row(name: &str, cost: &str) -> ModelRowViewModel {
        ModelRowViewModel {
            name: String::from(name),
            input: String::from("1.2M"),
            cached: String::from("800K"),
            output: String::from("12K"),
            total: String::from("2M"),
            cost: String::from(cost),
        }
    }

    /// The labels of `vm` that show, out of `labels`.
    fn shown(vm: StatsDialogViewModel, labels: &[&str]) -> Vec<String> {
        let themes = ThemeManager::new();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1000.0, 1200.0),
            view(StatsDialogProps {
                vm,
                theme: ThemeProps::new(themes.current()),
            }),
        );

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

    #[test]
    fn a_session_shows_its_facts_tokens_money_and_limits() {
        let vm = dialog(StatsBodyViewModel::Session(SessionStatsViewModel {
            facts: vec![
                FactViewModel::new("Started", "Nov 16, 2026, 10:00".into()),
                FactViewModel::new("Messages", String::from("4")),
            ],
            models: ModelsTableViewModel {
                has_cost: true,
                rows: vec![row("claude-opus-4-5", "$3.20")],
            },
            limits: vec![FactViewModel::new(
                "5 hours",
                String::from("12 % → 31 %"),
            )],
        }));

        let labels = [
            "Started",
            "Nov 16, 2026, 10:00",
            "4",
            i18n::t(Key::AgentsStatsTokensByModel),
            i18n::t(Key::AgentsColumnIn),
            i18n::t(Key::AgentsColumnCached),
            i18n::t(Key::AgentsColumnOut),
            i18n::t(Key::AgentsColumnTotal),
            i18n::t(Key::AgentsColumnCost),
            "claude-opus-4-5",
            "1.2M",
            "$3.20",
            i18n::t(Key::AgentsStatsLimits),
            "5 hours",
            "12 % → 31 %",
        ];
        assert_eq!(shown(vm, &labels), labels.map(String::from));
    }

    #[test]
    fn a_session_with_no_tokens_and_no_limits_says_so_and_has_no_cost() {
        let vm = dialog(StatsBodyViewModel::Session(SessionStatsViewModel {
            facts: Vec::new(),
            models: ModelsTableViewModel {
                has_cost: false,
                rows: Vec::new(),
            },
            limits: Vec::new(),
        }));

        let shown = shown(
            vm,
            &[
                i18n::t(Key::AgentsStatsNoTokens),
                i18n::t(Key::AgentsColumnCost),
                i18n::t(Key::AgentsStatsLimits),
            ],
        );

        assert_eq!(shown, [i18n::t(Key::AgentsStatsNoTokens)]);
    }

    #[test]
    fn a_project_shows_its_models_and_its_profiles() {
        let vm = dialog(StatsBodyViewModel::Project(ProjectStatsViewModel {
            facts: vec![FactViewModel::new("Sessions", String::from("7"))],
            models: ModelsTableViewModel {
                has_cost: false,
                rows: vec![row("gpt-5-codex", "")],
            },
            profiles: ProfilesTableViewModel {
                has_cost: true,
                rows: vec![ProfileStatsRowViewModel {
                    name: String::from("Codex · Work"),
                    sessions: String::from("5"),
                    tokens: String::from("3.4M"),
                    cost: String::from("$7.10"),
                }],
            },
        }));

        let labels = [
            "Sessions",
            "7",
            "gpt-5-codex",
            i18n::t(Key::AgentsStatsByProfile),
            "Codex · Work",
            "3.4M",
            "$7.10",
            i18n::t(Key::AgentsColumnCost),
        ];
        assert_eq!(shown(vm, &labels), labels.map(String::from));
    }
}
