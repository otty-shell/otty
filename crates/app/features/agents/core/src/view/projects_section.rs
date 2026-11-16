//! The Projects section of the Agents mode left panel: the search button
//! and New session in the header, the search field with its funnel, chips
//! and note while it is open, then a row for each project.

use iced::widget::{Column, Row, column, row, scrollable, svg};
use iced::{Alignment, Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::chip::Chip;
use otty_ui_kit::components::icon_button::{
    IconButton, IconButtonSize, IconTone,
};
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::search_field::SearchField;
use otty_ui_kit::components::session_row::SessionTone;
use otty_ui_kit::components::split_button::SplitButton;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::super::PROJECT_SEARCH;
use super::super::event::AgentsIntent;
use super::super::model::{
    ProjectRowViewModel, ProjectSearchViewModel, ProjectsSectionViewModel,
};
use super::left_panel::{note_height, note_text};

/// Gap between the search field and the funnel, and between the chips
/// (`gap: 4px`).
const SEARCH_GAP: f32 = spacing::SPACE_1;
/// Space above the search block (`padding: 2px 4px 6px 4px`).
const SEARCH_TOP: f32 = 2.0;

/// Render the Projects section. The search button opens or closes the
/// search field; the plus of New session starts a session in the selected
/// project and its chevron lists the profiles.
pub(crate) fn view<'a>(
    vm: ProjectsSectionViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let is_search_open = vm.search.is_some();
    let search_tone = if is_search_open {
        IconTone::Accent
    } else {
        IconTone::Default
    };
    let search_button = IconButton::new(
        svg::Handle::from_memory(icons::SEARCH),
        i18n::t(Key::AgentsSearchProjects),
        palette,
    )
    .size(IconButtonSize::Small)
    .tone(search_tone)
    .on_press(AgentsIntent::ToggleProjectSearch);
    let mut new_session = SplitButton::new(
        svg::Handle::from_memory(icons::PLUS),
        i18n::t(Key::AgentsNewSession),
        i18n::t(Key::AgentsChooseAgent),
        palette,
    )
    .menu_open(vm.is_menu_open);
    if vm.can_start {
        new_session = new_session
            .on_press(AgentsIntent::NewSession)
            .on_menu(AgentsIntent::OpenNewSessionMenu);
    }

    let mut rows = Column::new().width(Length::Fill);
    for project in vm.projects {
        rows = rows.push(project_row(project, palette));
    }
    if let Some(note) = vm.note {
        rows = rows.push(note_text(note, palette));
    }
    let rows = scrollable(rows)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette));
    let mut body = Column::new().width(Length::Fill);
    if let Some(search) = vm.search {
        body = body.push(search_block(search, palette));
    }
    let body = body.push(rows);

    let mut section = PanelSection::new(i18n::t(Key::AgentsProjects), palette)
        .open(vm.is_open)
        .grow(true)
        .action(search_button)
        .action(new_session)
        .body(body)
        .on_toggle(AgentsIntent::ToggleProjectsSection);
    if !vm.is_open {
        section = section.count(vm.count);
    }

    section.into()
}

/// Height of the body of the Projects section: the search block while the
/// field is open, the project rows and the note.
///
/// Counted from the rows because iced cannot measure a widget before the
/// left panel shares its height between the sections. The chips are
/// counted as one line.
pub(crate) fn content_height(vm: &ProjectsSectionViewModel<'_>) -> f32 {
    let search = vm.search.as_ref().map_or(0.0, search_height);
    let rows = vm.projects.len() as f32 * size::ROW;
    let note = vm.note.map_or(0.0, |_| note_height());

    search + rows + note
}

/// A project: its folder name and its sessions count, or that its folder
/// is gone.
fn project_row<'a>(
    vm: ProjectRowViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let row = TreeRow::new(vm.name, palette)
        .selected(vm.is_selected)
        .on_press(AgentsIntent::SelectProject(vm.folder));

    if vm.is_missing {
        return row
            .state(i18n::t(Key::AgentsProjectMissing))
            .count_tone(SessionTone::Failed)
            .into();
    }
    row.count(vm.sessions).into()
}

/// The search field with the funnel after it, the chips of the filters
/// that are on, and the note that says how many projects match.
fn search_block<'a>(
    vm: ProjectSearchViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let field = SearchField::new(
        vm.query,
        svg::Handle::from_memory(icons::SEARCH),
        palette,
    )
    .placeholder(i18n::t(Key::AgentsSearchPlaceholder))
    .id(PROJECT_SEARCH)
    .on_input(AgentsIntent::ProjectSearchChanged)
    .on_clear(
        AgentsIntent::ProjectSearchCleared,
        i18n::t(Key::AgentsSearchClear),
    );
    let funnel_tone = if vm.is_funnel_lit {
        IconTone::Accent
    } else {
        IconTone::Default
    };
    let funnel = IconButton::new(
        svg::Handle::from_memory(icons::FILTER),
        i18n::t(Key::AgentsFilterProjects),
        palette,
    )
    .size(IconButtonSize::Small)
    .tone(funnel_tone)
    .on_press(AgentsIntent::OpenFilterMenu);

    let mut block = column![
        row![field, funnel]
            .spacing(SEARCH_GAP)
            .align_y(Alignment::Center)
    ]
    .spacing(spacing::SPACE_2);
    if !vm.chips.is_empty() {
        let chips = vm.chips.into_iter().map(|chip| {
            Chip::new(chip.label, palette)
                .on_clear(
                    i18n::t(Key::AgentsChipRemove),
                    AgentsIntent::ProjectFilter(chip.clear),
                )
                .into()
        });
        block =
            block.push(Row::with_children(chips).spacing(SEARCH_GAP).wrap());
    }
    block = block.push(note_text(vm.note, palette));

    block
        .padding(Padding {
            top: SEARCH_TOP,
            right: spacing::SPACE_1,
            bottom: spacing::SPACE_2,
            left: spacing::SPACE_1,
        })
        .into()
}

/// Height of the search block: the field, a line of chips while any
/// filter is on, the note, the gaps between them and the padding.
fn search_height(vm: &ProjectSearchViewModel<'_>) -> f32 {
    let chips = if vm.chips.is_empty() {
        0.0
    } else {
        size::CONTROL_SM + spacing::SPACE_2
    };

    SEARCH_TOP
        + size::CONTROL_MD
        + spacing::SPACE_2
        + chips
        + note_height()
        + spacing::SPACE_2
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::FilterChipViewModel;
    use crate::project_filter::ProjectFilterChange;

    fn project(name: &str, is_missing: bool) -> ProjectRowViewModel {
        ProjectRowViewModel {
            folder: PathBuf::from(format!("/work/{name}")),
            name: String::from(name),
            sessions: 3,
            is_missing,
            is_selected: false,
        }
    }

    fn section() -> ProjectsSectionViewModel<'static> {
        ProjectsSectionViewModel {
            is_open: true,
            count: 2,
            search: None,
            projects: vec![project("otty", false), project("gone", true)],
            note: None,
            can_start: true,
            is_menu_open: false,
        }
    }

    fn with_search() -> ProjectsSectionViewModel<'static> {
        ProjectsSectionViewModel {
            search: Some(ProjectSearchViewModel {
                query: "ott",
                chips: vec![FilterChipViewModel {
                    label: String::from("Last 30 days"),
                    clear: ProjectFilterChange::RecentOnly(false),
                }],
                is_funnel_lit: true,
                note: String::from("1 of 2 projects match"),
            }),
            ..section()
        }
    }

    /// Width of the section under test.
    const WIDTH: f32 = 280.0;
    /// The right edge of the last header button, and of the funnel.
    const RIGHT_END: f32 = WIDTH - spacing::SPACE_3 - spacing::SPACE_1;

    fn section_view<'a>(
        vm: ProjectsSectionViewModel<'a>,
        themes: &ThemeManager,
    ) -> Element<'a, AgentsIntent> {
        iced::widget::container(view(vm, themes.current().ui()))
            .width(WIDTH)
            .into()
    }

    fn pressed(
        vm: ProjectsSectionViewModel<'_>,
        label: &str,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(section_view(vm, &themes));

        ui.click(label).expect("the label should show");

        ui.into_messages().collect()
    }

    /// What a press `left` pixels left of [`RIGHT_END`], at the height of
    /// the text `beside`, sends.
    fn pressed_at(
        vm: ProjectsSectionViewModel<'_>,
        beside: &str,
        left: f32,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(section_view(vm, &themes));
        let row = ui.find(beside).expect("the text should show").bounds();

        ui.point_at(iced::Point::new(RIGHT_END - left, row.center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    /// The press on the cross of the chip `label`.
    fn chip_cross_pressed(
        vm: ProjectsSectionViewModel<'_>,
        label: &str,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(section_view(vm, &themes));
        let chip = ui.find(label).expect("the chip should show").bounds();
        let x = chip.x + chip.width + spacing::SPACE_2 + size::CONTROL_SM / 2.0;

        ui.point_at(iced::Point::new(x, chip.center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn the_rows_show_their_count_or_that_the_folder_is_gone() {
        let themes = ThemeManager::new();
        let mut ui = simulator(section_view(section(), &themes));

        ui.find("3").expect("the sessions count should show");
        ui.find(i18n::t(Key::AgentsProjectMissing))
            .expect("the missing folder should say so");
        ui.click("otty").expect("the project should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![AgentsIntent::SelectProject(PathBuf::from("/work/otty"))]
        );
    }

    #[test]
    fn the_header_opens_the_search_and_new_session_starts_one() {
        let title = i18n::t(Key::AgentsProjects);
        let split = size::CONTROL_SM + size::SPLIT_CHEVRON;
        let chevron = size::SPLIT_CHEVRON / 2.0;
        let plus = size::SPLIT_CHEVRON + size::CONTROL_SM / 2.0;
        let search = split + spacing::SPACE_2 + size::CONTROL_SM / 2.0;

        let searched = pressed_at(section(), title, search);
        let started = pressed_at(section(), title, plus);
        let listed = pressed_at(section(), title, chevron);
        let toggled = pressed(section(), title);
        let off = ProjectsSectionViewModel {
            can_start: false,
            ..section()
        };
        let nothing = pressed_at(off, title, plus);

        assert_eq!(searched, vec![AgentsIntent::ToggleProjectSearch]);
        assert_eq!(started, vec![AgentsIntent::NewSession]);
        assert_eq!(listed, vec![AgentsIntent::OpenNewSessionMenu]);
        assert_eq!(toggled, vec![AgentsIntent::ToggleProjectsSection]);
        assert!(nothing.is_empty(), "no project can start a session");
    }

    #[test]
    fn the_search_block_shows_the_text_the_chips_and_the_note() {
        let themes = ThemeManager::new();
        let mut ui = simulator(section_view(with_search(), &themes));

        ui.find("ott").expect("the text should show");
        ui.find("Last 30 days").expect("the chip should show");
        ui.find("1 of 2 projects match")
            .expect("the note should show");
    }

    #[test]
    fn the_cross_of_a_chip_and_the_funnel_send_their_intents() {
        let cleared = chip_cross_pressed(with_search(), "Last 30 days");
        let funnel = pressed_at(with_search(), "ott", size::CONTROL_SM / 2.0);

        assert_eq!(
            cleared,
            vec![AgentsIntent::ProjectFilter(
                ProjectFilterChange::RecentOnly(false)
            )]
        );
        assert_eq!(funnel, vec![AgentsIntent::OpenFilterMenu]);
    }

    #[test]
    fn the_cross_of_the_field_clears_it() {
        // The cross stands in the field, before the funnel and its gap,
        // inside the field's padding.
        let cross = size::CONTROL_SM + SEARCH_GAP + spacing::SPACE_3 + 7.0;
        let cleared = pressed_at(with_search(), "ott", cross);

        assert_eq!(cleared, vec![AgentsIntent::ProjectSearchCleared]);
    }

    #[test]
    fn the_body_height_counts_the_search_block_and_the_rows() {
        let plain = content_height(&section());
        let searched = content_height(&with_search());

        assert_eq!(plain, 2.0 * size::ROW);
        assert_eq!(
            searched - plain,
            SEARCH_TOP
                + size::CONTROL_MD
                + size::CONTROL_SM
                + 3.0 * spacing::SPACE_2
                + note_height()
        );
    }
}
