//! The top section of a profile's dashboard, "Top, last 30 days": its
//! title with the tabs By project and By model at the end of the line, and
//! the table of the chosen tab, or a muted line when the tab has nothing to
//! list (spec Decision 13).

use iced::widget::{Space, column, container, row};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::{
    ColumnAlign, DataTable, TableCell, TableColumn,
};
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::super::event::AgentsIntent;
use super::super::model::{
    TopModelRowViewModel, TopProjectRowViewModel, TopRowsViewModel,
    TopViewModel,
};
use super::super::top_tab::TopTab;
use super::blocks;

/// Gap between the title line and the table (`gap: 10px`).
const TITLE_GAP: f32 = 10.0;
/// Least width of the name column of a table.
const NAME_MIN_WIDTH: u16 = 120;
/// Least width of a number column of a table.
const NUMBER_MIN_WIDTH: u16 = 64;

/// Render the top section.
pub(super) fn view<'a>(
    top: TopViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let tabs =
        segmented(&TopTab::ALL, &top.tab, AgentsIntent::ShowTop, palette);
    let title_line = row![
        blocks::title(i18n::t(Key::AgentsTopTitle), palette),
        Space::new().width(Length::Fill),
        tabs,
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);

    let body = match top.rows {
        TopRowsViewModel::Projects(projects) if !projects.is_empty() => {
            projects_table(projects, palette)
        },
        TopRowsViewModel::Models(models) if !models.is_empty() => {
            models_table(models, palette)
        },
        _ => blocks::small_text(
            i18n::t(Key::AgentsTopEmpty),
            palette.text_muted(),
        ),
    };

    column![title_line, body].spacing(TITLE_GAP).into()
}

/// The projects with their sessions and tokens.
fn projects_table<'a>(
    projects: Vec<TopProjectRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let rows = projects.len();
    let mut table = DataTable::new(
        vec![
            name_column(Key::AgentsColumnProject),
            number_column(Key::AgentsColumnSessions),
            number_column(Key::AgentsColumnTokens),
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

    as_high_as_its_rows(table, rows)
}

/// The models with their requests and tokens.
fn models_table<'a>(
    models: Vec<TopModelRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let rows = models.len();
    let mut table = DataTable::new(
        vec![
            name_column(Key::AgentsColumnModel),
            number_column(Key::AgentsColumnRequests),
            number_column(Key::AgentsColumnTokens),
        ],
        palette,
    );
    for model in models {
        table = table.row_without_menu(vec![
            TableCell::plain(model.name),
            TableCell::muted(model.requests),
            TableCell::number(model.tokens),
        ]);
    }

    as_high_as_its_rows(table, rows)
}

/// The first column of a table: the project or the model.
fn name_column(key: Key) -> TableColumn {
    TableColumn::new(i18n::t(key), 3)
        .min_width(NAME_MIN_WIDTH)
        .sortable(false)
}

/// A column of numbers, at the right of the table.
fn number_column(key: Key) -> TableColumn {
    TableColumn::new(i18n::t(key), 1)
        .min_width(NUMBER_MIN_WIDTH)
        .align(ColumnAlign::End)
        .sortable(false)
}

/// `table` as high as its `rows`: the page scrolls, not the table.
fn as_high_as_its_rows<'a>(
    table: DataTable<'a, AgentsIntent>,
    rows: usize,
) -> Element<'a, AgentsIntent> {
    let height =
        size::TABLE_HEADER + size::LINE + rows as f32 * size::TABLE_ROW;

    container(table).width(Length::Fill).height(height).into()
}

#[cfg(test)]
mod tests {
    use iced::widget::container;
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{
        TopModelRowViewModel, TopProjectRowViewModel, TopRowsViewModel,
    };
    use crate::top_tab::TopTab;

    fn by_project() -> TopViewModel {
        TopViewModel {
            tab: TopTab::ByProject,
            rows: TopRowsViewModel::Projects(vec![TopProjectRowViewModel {
                name: String::from("otty"),
                sessions: String::from("9"),
                tokens: String::from("18.4M"),
            }]),
        }
    }

    fn by_model() -> TopViewModel {
        TopViewModel {
            tab: TopTab::ByModel,
            rows: TopRowsViewModel::Models(vec![TopModelRowViewModel {
                name: String::from("claude-opus-4-5"),
                requests: String::from("1 202"),
                tokens: String::from("12.1M"),
            }]),
        }
    }

    /// Which of `texts` show in the section of `top`, in order.
    fn shown(top: TopViewModel, texts: &[&str]) -> Vec<bool> {
        let themes = ThemeManager::new();
        let section = view(top, themes.current().ui());
        let mut ui = simulator(container(section).width(600.0));

        texts.iter().map(|text| ui.find(*text).is_ok()).collect()
    }

    /// What a press on `label` in the section of `top` sends.
    fn pressed(top: TopViewModel, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let section = view(top, themes.current().ui());
        let mut ui = simulator(container(section).width(600.0));

        ui.click(label).expect("the text should show");

        ui.into_messages().collect()
    }

    #[test]
    fn by_project_shows_the_title_the_tabs_and_the_projects() {
        let texts = [
            i18n::t(Key::AgentsTopTitle),
            i18n::t(Key::AgentsTopByProject),
            i18n::t(Key::AgentsTopByModel),
            i18n::t(Key::AgentsColumnProject),
            i18n::t(Key::AgentsColumnSessions),
            "otty",
            "18.4M",
            i18n::t(Key::AgentsColumnModel),
        ];

        let found = shown(by_project(), &texts);

        assert_eq!(found, [true, true, true, true, true, true, true, false]);
    }

    #[test]
    fn by_model_shows_the_models_with_their_requests() {
        let texts = [
            i18n::t(Key::AgentsTopByProject),
            i18n::t(Key::AgentsColumnModel),
            i18n::t(Key::AgentsColumnRequests),
            "claude-opus-4-5",
            "1 202",
            "12.1M",
            i18n::t(Key::AgentsColumnProject),
        ];

        let found = shown(by_model(), &texts);

        assert_eq!(found, [true, true, true, true, true, true, false]);
    }

    #[test]
    fn pressing_a_tab_chooses_it() {
        assert_eq!(
            pressed(by_project(), i18n::t(Key::AgentsTopByModel)),
            vec![AgentsIntent::ShowTop(TopTab::ByModel)]
        );
        assert_eq!(
            pressed(by_model(), i18n::t(Key::AgentsTopByProject)),
            vec![AgentsIntent::ShowTop(TopTab::ByProject)]
        );
    }

    #[test]
    fn a_tab_with_nothing_to_list_says_so_in_place_of_the_table() {
        let empty = TopViewModel {
            tab: TopTab::ByModel,
            rows: TopRowsViewModel::Models(Vec::new()),
        };

        let found = shown(
            empty,
            &[
                i18n::t(Key::AgentsTopByProject),
                i18n::t(Key::AgentsTopEmpty),
                i18n::t(Key::AgentsColumnModel),
            ],
        );

        assert_eq!(found, [true, true, false]);
    }
}
