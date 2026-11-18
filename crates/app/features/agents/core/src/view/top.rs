//! The top section of a profile's dashboard, "Top, last 30 days": its
//! title with the tabs By project and By model at the end of the line, and
//! the table of the chosen tab, or a muted line when the tab has nothing to
//! list (spec Decision 13). The body is always as tall as the most rows a
//! tab lists, so the table never scrolls and a tab switch moves nothing.

use iced::widget::{Space, column, row};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::{DataTable, TableCell};
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::spacing;

use super::super::event::AgentsIntent;
use super::super::model::{
    TOP_ROWS, TopModelRowViewModel, TopProjectRowViewModel, TopRowsViewModel,
    TopViewModel,
};
use super::super::top_tab::TopTab;
use super::blocks;

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
    let body = blocks::rows_tall(body, TOP_ROWS);

    column![title_line, body].spacing(blocks::TITLE_GAP).into()
}

/// The projects with their sessions and tokens.
fn projects_table<'a>(
    projects: Vec<TopProjectRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut table = DataTable::new(
        vec![
            blocks::name_column(Key::AgentsColumnProject),
            blocks::number_column(Key::AgentsColumnSessions),
            blocks::number_column(Key::AgentsColumnTokens),
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

    table.into()
}

/// The models with their requests and tokens.
fn models_table<'a>(
    models: Vec<TopModelRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut table = DataTable::new(
        vec![
            blocks::name_column(Key::AgentsColumnModel),
            blocks::number_column(Key::AgentsColumnRequests),
            blocks::number_column(Key::AgentsColumnTokens),
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

    table.into()
}

#[cfg(test)]
mod tests {
    use iced::widget::container;
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::data_table;

    use super::*;
    use crate::model::{
        TOP_ROWS, TopModelRowViewModel, TopProjectRowViewModel,
        TopRowsViewModel,
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

    /// Where the text "below", put under the section of `top`, starts.
    fn top_of_what_follows(top: TopViewModel) -> f32 {
        let themes = ThemeManager::new();
        let section = view(top, themes.current().ui());
        let page = iced::widget::column![section, iced::widget::text("below")];
        let mut ui = simulator(container(page).width(600.0));

        ui.find("below").expect("the text should show").bounds().y
    }

    fn models(count: usize) -> TopViewModel {
        let rows = (0..count)
            .map(|index| TopModelRowViewModel {
                name: format!("model-{index}"),
                requests: String::from("1"),
                tokens: String::from("1K"),
            })
            .collect();
        TopViewModel {
            tab: TopTab::ByModel,
            rows: TopRowsViewModel::Models(rows),
        }
    }

    /// Owner, 2026-11-17: the table has a fixed height that holds every
    /// row it may list, so it never scrolls and switching tabs does not
    /// move what follows.
    #[test]
    fn the_section_is_as_tall_for_one_row_all_rows_and_none() {
        let one = top_of_what_follows(models(1));
        let all = top_of_what_follows(models(TOP_ROWS));
        let none = top_of_what_follows(models(0));

        assert_eq!(one, all);
        assert_eq!(none, all);
        assert!(all >= data_table::height_for_rows(TOP_ROWS));
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
