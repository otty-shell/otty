//! "Where the numbers come from" on a profile's dashboard: a kit
//! `DataTable` as tall as its rows, with the number each row gives, the
//! plain name of its source with what it does in the tooltip, its state
//! with why in the tooltip, and when its data is from (spec Decision 18).

use iced::Element;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::{DataTable, TableCell, TableColumn};
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::SourceRowViewModel;
use super::blocks;

/// Least width of the Number column: "Limits", "Prices". The four least
/// widths fit the dashboard of the smallest main area, so the table does
/// not scroll sideways there.
const NUMBER_MIN_WIDTH: u16 = 64;
/// Least width of the Source column; a longer name ends with "…".
const SOURCE_MIN_WIDTH: u16 = 170;
/// Least width of the State column: "not available" with its dot.
const STATE_MIN_WIDTH: u16 = 120;
/// Least width of the Updated column: "Nov 16, 2026, 10:00".
const UPDATED_MIN_WIDTH: u16 = 150;

/// Render the title and the table of the `sources`.
pub(super) fn view<'a>(
    sources: Vec<SourceRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let rows = sources.len();

    let mut table = DataTable::new(columns(), palette);
    for source in sources {
        table = table.row_without_menu(cells(source));
    }

    blocks::titled(
        i18n::t(Key::AgentsSourcesTitle),
        blocks::rows_tall(table, rows),
        palette,
    )
}

/// The columns: Number, Source, State and Updated.
fn columns() -> Vec<TableColumn> {
    vec![
        TableColumn::new(i18n::t(Key::AgentsColumnNumber), 2)
            .min_width(NUMBER_MIN_WIDTH)
            .sortable(false),
        TableColumn::new(i18n::t(Key::AgentsColumnSource), 5)
            .min_width(SOURCE_MIN_WIDTH)
            .sortable(false),
        TableColumn::new(i18n::t(Key::AgentsColumnState), 2)
            .min_width(STATE_MIN_WIDTH)
            .sortable(false),
        TableColumn::new(i18n::t(Key::AgentsColumnSourceUpdated), 3)
            .min_width(UPDATED_MIN_WIDTH)
            .sortable(false),
    ]
}

/// The cells of the row of `source`: the number, on the first row of its
/// group only; the name, with what the source does as its tooltip; the
/// state, with why as its tooltip when there is one; and when its data is
/// from, when known.
fn cells(source: SourceRowViewModel) -> Vec<TableCell> {
    let name = TableCell::plain(source.name).with_tooltip(source.name_tip);
    let badge = TableCell::badge(source.state, source.tone);
    let state = match source.tip {
        Some(tip) => badge.with_tooltip(tip),
        None => badge,
    };

    vec![
        TableCell::plain(source.number.unwrap_or_default()),
        name,
        state,
        TableCell::muted(source.updated.unwrap_or_default()),
    ]
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use iced::widget::{container, text};
    use iced_test::simulator;
    use otty_app_testing::temp_dir::TempDir;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::data_table;
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::*;

    /// Width of the dashboard column in the tests.
    const WIDTH: f32 = 720.0;

    fn limits_rows() -> Vec<SourceRowViewModel> {
        vec![
            SourceRowViewModel {
                source: otty_agents::SourceId::AnthropicUsageApi,
                number: Some(i18n::t(Key::AgentsNumberLimits)),
                name: "Anthropic usage API (direct request)",
                name_tip: "Asks anthropic.com directly",
                state: i18n::t(Key::AgentsSourceLoginExpired),
                tone: StatusTone::Warn,
                tip: Some(String::from("Claude Code renews it when it runs")),
                updated: None,
            },
            SourceRowViewModel {
                source: otty_agents::SourceId::ClaudeUsageCommand,
                number: None,
                name: "Claude Code /usage (runs claude)",
                name_tip: "Runs claude in a hidden terminal",
                state: i18n::t(Key::AgentsSourceAnswered),
                tone: StatusTone::Ok,
                tip: None,
                updated: Some(String::from("Nov 16, 2026, 10:00")),
            },
            SourceRowViewModel {
                source: otty_agents::SourceId::SessionFiles,
                number: Some(i18n::t(Key::AgentsNumberTokens)),
                name: "Session files",
                name_tip: "The agent's session files on this computer",
                state: i18n::t(Key::AgentsSourceAnswered),
                tone: StatusTone::Ok,
                tip: None,
                updated: Some(String::from("Nov 16, 2026, 10:05")),
            },
        ]
    }

    /// Where the text "below", put under the table of `rows`, starts.
    fn top_of_what_follows(rows: Vec<SourceRowViewModel>) -> f32 {
        let themes = ThemeManager::new();
        let page = iced::widget::column![
            view(rows, themes.current().ui()),
            text("below")
        ];
        let mut ui = simulator(container(page).width(WIDTH));

        ui.find("below").expect("the text should show").bounds().y
    }

    #[test]
    fn the_table_has_its_title_and_four_columns() {
        let themes = ThemeManager::new();
        let table = view(limits_rows(), themes.current().ui());
        let mut ui = simulator(container(table).width(WIDTH));

        for title in [
            i18n::t(Key::AgentsSourcesTitle),
            i18n::t(Key::AgentsColumnNumber),
            i18n::t(Key::AgentsColumnSource),
            i18n::t(Key::AgentsColumnState),
            i18n::t(Key::AgentsColumnSourceUpdated),
        ] {
            assert!(ui.find(title).is_ok(), "{title} should show");
        }
    }

    #[test]
    fn each_row_shows_its_number_name_state_and_time() {
        let themes = ThemeManager::new();
        let table = view(limits_rows(), themes.current().ui());
        let mut ui = simulator(container(table).width(WIDTH));

        let limits = ui
            .find(i18n::t(Key::AgentsNumberLimits))
            .expect("the number shows")
            .bounds();
        let first = ui
            .find("Anthropic usage API (direct request)")
            .expect("the name shows")
            .bounds();
        let second = ui
            .find("Claude Code /usage (runs claude)")
            .expect("the name shows")
            .bounds();
        let tokens = ui
            .find(i18n::t(Key::AgentsNumberTokens))
            .expect("the number shows")
            .bounds();
        let session_files =
            ui.find("Session files").expect("the name shows").bounds();

        assert_eq!(limits.center_y(), first.center_y());
        assert!(limits.x < first.x, "the number is the first column");
        assert!(second.y > first.y);
        assert_eq!(tokens.center_y(), session_files.center_y());
        assert!(ui.find(i18n::t(Key::AgentsSourceLoginExpired)).is_ok());
        assert!(ui.find("Nov 16, 2026, 10:00").is_ok());
        assert!(ui.find("Nov 16, 2026, 10:05").is_ok());
        assert!(
            ui.find("Claude Code renews it when it runs").is_err(),
            "why a state failed is its tooltip"
        );
        assert!(
            ui.find("Asks anthropic.com directly").is_err(),
            "what a source does is the tooltip of its name"
        );
    }

    /// Owner, issue 42: the table is as tall as its rows, so it never
    /// scrolls.
    #[test]
    fn the_table_is_as_tall_as_its_rows() {
        let one = top_of_what_follows(limits_rows()[..1].to_vec());
        let three = top_of_what_follows(limits_rows());

        assert_eq!(
            three - one,
            data_table::height_for_rows(3) - data_table::height_for_rows(1)
        );
    }

    /// Says whether the table whose first name has `name_tip` as its
    /// tooltip draws the picture at `reference`, with the pointer over the
    /// name; the first call saves the picture.
    fn hovered_name_draws(name_tip: &'static str, reference: &Path) -> bool {
        let themes = ThemeManager::new();
        let mut rows = limits_rows();
        rows[0].name_tip = name_tip;
        let table = view(rows, themes.current().ui());
        let mut ui = simulator(container(table).width(WIDTH).height(400.0));
        let name = ui
            .find("Anthropic usage API (direct request)")
            .expect("the name shows")
            .bounds();
        let position = iced::Point::new(name.x + 10.0, name.center_y());
        ui.point_at(position);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved { position },
        )]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("the table should draw")
            .matches_hash(reference)
            .expect("the reference should be readable")
    }

    /// Owner, issue 42: the tooltip of a name says what its source does.
    #[test]
    fn the_name_of_a_source_shows_what_it_does_in_its_tooltip() {
        let folder = TempDir::new("agents-sources-name");
        let reference = folder.path().join("over");

        assert!(hovered_name_draws(
            "Asks anthropic.com directly",
            &reference
        ));
        assert!(
            !hovered_name_draws("Runs claude in a hidden terminal", &reference),
            "the tooltip of the name shows under the pointer"
        );
    }
}
