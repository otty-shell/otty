//! The right column of a profile's dashboard: the Profile facts, the top
//! projects or the money by model, and where the numbers come from.

use iced::widget::{Column, column, container, row, text};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::{
    ColumnAlign, DataTable, TableCell, TableColumn,
};
use otty_ui_kit::components::key_value_list::KeyValueList;
use otty_ui_kit::components::status_badge::StatusBadge;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{SourceRowViewModel, TopTableViewModel};
use super::blocks;

/// Gap between the blocks of the column (`gap: 28px`).
const BLOCK_GAP: f32 = 28.0;
/// Gap between a block title and its content (`gap: 10px`).
const TITLE_GAP: f32 = 10.0;

/// Render the right column.
pub(super) fn view<'a>(
    facts: Vec<(&'static str, String)>,
    top: Option<TopTableViewModel>,
    sources: Vec<SourceRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut list = KeyValueList::new(palette);
    for (label, value) in facts {
        list = list.entry(label, value);
    }

    let mut blocks = Column::new().spacing(BLOCK_GAP).push(
        column![
            blocks::title(i18n::t(Key::AgentsProfileTitle), None, palette),
            list
        ]
        .spacing(TITLE_GAP),
    );
    if let Some(top) = top {
        blocks = blocks.push(top_table(top, palette));
    }
    if !sources.is_empty() {
        blocks = blocks.push(sources_list(sources, palette));
    }

    blocks.width(Length::Fill).into()
}

/// The top projects or the money by model, with its title. The table is
/// as high as its rows: the page scrolls, not the table.
fn top_table<'a>(
    top: TopTableViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let number = |key: Key| {
        TableColumn::new(i18n::t(key), 1)
            .min_width(64)
            .align(ColumnAlign::End)
            .sortable(false)
    };
    let name = |key: Key| {
        TableColumn::new(i18n::t(key), 3)
            .min_width(120)
            .sortable(false)
    };

    let (title, table, rows) = match top {
        TopTableViewModel::Projects(projects) => {
            let rows = projects.len();
            let mut table = DataTable::new(
                vec![
                    name(Key::AgentsColumnProject),
                    number(Key::AgentsColumnSessions),
                    number(Key::AgentsColumnTokens),
                ],
                palette,
            );
            for project in projects {
                table = table.row_without_menu(vec![
                    TableCell::plain(project.name),
                    TableCell::muted(project.sessions),
                    TableCell::number(project.tokens),
                ]);
            }
            (Key::AgentsTopProjects, table, rows)
        },
        TopTableViewModel::Models(models) => {
            let rows = models.len();
            let mut table = DataTable::new(
                vec![
                    name(Key::AgentsColumnModel),
                    number(Key::AgentsColumnInput),
                    number(Key::AgentsColumnOutput),
                    number(Key::AgentsColumnCost),
                ],
                palette,
            );
            for model in models {
                table = table.row_without_menu(vec![
                    TableCell::mono(model.model),
                    TableCell::muted(model.input),
                    TableCell::muted(model.output),
                    TableCell::number(model.cost),
                ]);
            }
            (Key::AgentsByModel, table, rows)
        },
    };

    let height =
        size::TABLE_HEADER + size::LINE + rows as f32 * size::TABLE_ROW;
    column![
        blocks::title(i18n::t(title), None, palette),
        container(table).width(Length::Fill).height(height),
    ]
    .spacing(TITLE_GAP)
    .into()
}

/// Where the numbers come from: each source with its number, its name, its
/// state, and why it failed under it.
fn sources_list<'a>(
    sources: Vec<SourceRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let body = typography::BODY;

    let mut list = Column::new().spacing(spacing::SPACE_3).push(blocks::title(
        i18n::t(Key::AgentsSourcesTitle),
        None,
        palette,
    ));
    for (index, source) in sources.into_iter().enumerate() {
        let line = row![
            text(format!("{}", index + 1))
                .size(body.size())
                .font(fonts::UI)
                .color(palette.text_muted()),
            text(source.name)
                .size(body.size())
                .font(fonts::UI)
                .color(palette.text_body())
                .width(Length::Fill),
            StatusBadge::new(source.state, source.tone, palette),
        ]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center);

        let mut entry = column![line].spacing(spacing::SPACE_1);
        if let Some(reason) = source.reason {
            entry =
                entry.push(blocks::small_text(reason, palette.text_muted()));
        }
        list = list.push(entry);
    }

    list.into()
}

#[cfg(test)]
mod tests {
    use iced::widget::container;
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::*;
    use crate::model::{ModelRowViewModel, TopProjectRowViewModel};

    fn sources() -> Vec<SourceRowViewModel> {
        vec![
            SourceRowViewModel {
                name: String::from("usage request"),
                state: i18n::t(Key::AgentsSourceLoginExpired),
                tone: StatusTone::Warn,
                reason: None,
            },
            SourceRowViewModel {
                name: String::from("app-server"),
                state: i18n::t(Key::AgentsSourceNotAvailable),
                tone: StatusTone::Neutral,
                reason: Some(String::from("no program")),
            },
        ]
    }

    fn shown(top: Option<TopTableViewModel>, texts: &[&str], missing: &[&str]) {
        let themes = ThemeManager::new();
        let facts = vec![
            (
                i18n::t(Key::AgentsFieldAccount),
                String::from("user@example.com"),
            ),
            (i18n::t(Key::AgentsFieldConfigDir), String::from("~/.codex")),
        ];
        let column = view(facts, top, sources(), themes.current().ui());
        let mut ui = simulator(container(column).width(480.0));

        for text in texts {
            ui.find(*text).expect("the text should show");
        }
        for text in missing {
            assert!(ui.find(*text).is_err(), "{text} should not show");
        }
    }

    #[test]
    fn the_column_shows_the_facts_the_top_projects_and_the_sources() {
        let top = TopTableViewModel::Projects(vec![TopProjectRowViewModel {
            name: String::from("otty"),
            sessions: String::from("9"),
            tokens: String::from("18.4M"),
        }]);

        shown(
            Some(top),
            &[
                i18n::t(Key::AgentsProfileTitle),
                i18n::t(Key::AgentsFieldAccount),
                "user@example.com",
                "~/.codex",
                i18n::t(Key::AgentsTopProjects),
                "otty",
                "18.4M",
                i18n::t(Key::AgentsSourcesTitle),
                "usage request",
                i18n::t(Key::AgentsSourceLoginExpired),
                "app-server",
                i18n::t(Key::AgentsSourceNotAvailable),
                "no program",
            ],
            &[i18n::t(Key::AgentsByModel)],
        );
    }

    #[test]
    fn an_api_key_shows_the_money_by_model_and_no_table_is_left_out() {
        let top = TopTableViewModel::Models(vec![ModelRowViewModel {
            model: String::from("claude-opus-5-5"),
            input: String::from("9.8M"),
            output: String::from("1.4M"),
            cost: String::from("$118.20"),
        }]);

        shown(
            Some(top),
            &[i18n::t(Key::AgentsByModel), "claude-opus-5-5", "$118.20"],
            &[i18n::t(Key::AgentsTopProjects)],
        );
        shown(
            None,
            &[i18n::t(Key::AgentsProfileTitle)],
            &[i18n::t(Key::AgentsTopProjects), i18n::t(Key::AgentsByModel)],
        );
    }
}
