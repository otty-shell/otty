//! The Agents mode left panel under the switcher: the Agents section over
//! the Projects section, sharing the panel height the way Sessions and
//! Launches do in Terminal mode.

use iced::widget::{
    Column, column, container, responsive, row, scrollable, svg, text,
};
use iced::{Alignment, Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::icon_button::{
    IconButton, IconButtonSize, IconTone,
};
use otty_ui_kit::components::panel_height::{
    SectionHeight, share_panel_height,
};
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::components::{divider, scrollbar};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    AgentsSectionViewModel, HiddenProfilesViewModel, LeftPanelViewModel,
    ProfileRowViewModel,
};
use super::projects_section;

/// Props for the left panel.
pub(crate) struct LeftPanelProps<'a> {
    pub(crate) vm: LeftPanelViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the left panel: the Agents section, a line, then the Projects
/// section. Each open section takes the height of its rows while both fit
/// and scrolls inside its share when they do not (see
/// [`share_panel_height`]); the Projects section takes all the height left
/// under the Agents section.
///
/// The panel height is known only during layout, so this uses
/// `iced::widget::responsive`.
pub(crate) fn view(props: LeftPanelProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    responsive(move |panel| {
        let agents = SectionHeight::panel_section(
            agents_content_height(&vm.agents),
            vm.agents.is_open,
        );
        let projects = SectionHeight::panel_section(
            projects_section::content_height(&vm.projects),
            vm.projects.is_open,
        );
        let available = (panel.height - size::LINE).max(0.0);
        let heights = share_panel_height(available, &[agents, projects]);
        let agents_share = heights.first().copied().unwrap_or(0.0);

        column![
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

/// The Agents section: Refresh and `+` in the header, a row for each
/// profile, then the Hidden profiles row.
fn agents_section<'a>(
    vm: AgentsSectionViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let refresh = IconButton::new(
        svg::Handle::from_memory(icons::REFRESH),
        i18n::t(Key::AgentsRefreshAll),
        palette,
    )
    .size(IconButtonSize::Small)
    .on_press(AgentsIntent::Refresh);
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
    for profile in vm.profiles {
        rows = rows.push(profile_row(profile, palette));
    }
    if let Some(hidden) = vm.hidden {
        rows = rows.push(hidden_profiles(hidden, palette));
    }
    if let Some(note) = vm.note {
        rows = rows.push(note_text(note, palette));
    }
    let body = scrollable(rows)
        .width(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette));

    let mut section =
        PanelSection::new(i18n::t(Key::AgentsSectionAgents), palette)
            .open(vm.is_open)
            .action(refresh)
            .action(add)
            .body(body)
            .on_toggle(AgentsIntent::ToggleAgentsSection);
    if !vm.is_open {
        section = section.count(count);
    }

    section.into()
}

/// Height of the body of the Agents section: the profile rows, the Hidden
/// profiles row with the hidden profiles while it is open, and the note.
///
/// Counted from the rows because iced cannot measure a widget before the
/// left panel shares its height between the sections.
fn agents_content_height(vm: &AgentsSectionViewModel) -> f32 {
    let hidden = vm.hidden.as_ref().map_or(0, |hidden| {
        if hidden.is_open {
            1 + hidden.profiles.len()
        } else {
            1
        }
    });
    let rows = (vm.profiles.len() + hidden) as f32 * size::ROW;

    match vm.note {
        Some(_) => rows + note_height(),
        None => rows,
    }
}

/// A profile: the logo of its agent, its name and its short number.
fn profile_row<'a>(
    vm: ProfileRowViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let icon = svg::Handle::from_memory(icons::agent_icon(vm.icon));
    let mut row = TreeRow::new(vm.name, palette)
        .icon(icon, None)
        .count_tone(vm.tone)
        .selected(vm.is_selected)
        .on_press(AgentsIntent::SelectProfile(vm.profile));
    if let Some(number) = vm.number {
        row = row.state(number);
    }

    row.into()
}

/// The Hidden profiles row, and while it is open the hidden profiles, each
/// with Show.
fn hidden_profiles<'a>(
    vm: HiddenProfilesViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut list = Column::new().push(
        TreeRow::new(vm.label, palette)
            .caret(vm.is_open)
            .on_press(AgentsIntent::ToggleHiddenProfiles),
    );
    if !vm.is_open {
        return list.into();
    }

    for hidden in vm.profiles {
        let icon = svg::Handle::from_memory(icons::agent_icon(hidden.icon));
        let name = hidden.profile.name().to_string();
        list = list.push(
            row![
                container(
                    TreeRow::new(name, palette).depth(1).icon(icon, None)
                )
                .width(Length::Fill),
                Button::new(i18n::t(Key::AgentsShowProfile), palette)
                    .kind(ButtonKind::Ghost)
                    .size(ButtonSize::Small)
                    .on_press(AgentsIntent::ShowProfile(hidden.profile)),
            ]
            .height(size::ROW)
            .align_y(Alignment::Center),
        );
    }

    list.into()
}

/// A muted line in place of rows, such as "Looking for agents…".
pub(super) fn note_text<'a>(
    content: impl text::IntoFragment<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let small = typography::SMALL;

    container(
        text(content)
            .size(small.size())
            .line_height(iced::Pixels(small.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted()),
    )
    .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3]))
    .into()
}

/// Height of a [`note_text`] line.
pub(super) fn note_height() -> f32 {
    typography::SMALL.line_height() + 2.0 * spacing::SPACE_1
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_agents::{AgentId, Profile};
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::session_row::SessionTone;

    use super::*;
    use crate::model::{HiddenProfileViewModel, ProjectsSectionViewModel};

    fn codex() -> Profile {
        Profile::default_for(
            AgentId::new("codex"),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        )
    }

    fn old_codex() -> Profile {
        Profile::added(
            AgentId::new("codex"),
            "Codex · Old",
            PathBuf::from("/home/user/.codex-old"),
        )
        .with_hidden(true)
    }

    fn agents() -> AgentsSectionViewModel {
        AgentsSectionViewModel {
            is_open: true,
            profiles: vec![ProfileRowViewModel {
                profile: codex(),
                is_selected: false,
                icon: "codex",
                name: String::from("Codex"),
                number: Some(String::from("3 760 left")),
                tone: SessionTone::Muted,
            }],
            hidden: Some(HiddenProfilesViewModel {
                label: String::from("Hidden profiles (1)"),
                is_open: true,
                profiles: vec![HiddenProfileViewModel {
                    icon: "codex",
                    profile: old_codex(),
                }],
            }),
            can_add: true,
            note: None,
        }
    }

    fn panel(agents: AgentsSectionViewModel) -> LeftPanelViewModel<'static> {
        LeftPanelViewModel {
            agents,
            projects: ProjectsSectionViewModel {
                is_open: true,
                count: 0,
                search: None,
                projects: Vec::new(),
                note: None,
                can_start: false,
                is_menu_open: false,
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

    /// What a press on the header button of the Agents section
    /// `from_end` places from the right end sends: 0 is `+`, 1 Refresh.
    fn header_pressed(
        vm: LeftPanelViewModel<'_>,
        from_end: usize,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(panel_view(vm, &themes));
        let title = ui
            .find(i18n::t(Key::AgentsSectionAgents))
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
            pressed(panel(agents()), i18n::t(Key::AgentsSectionAgents));

        assert_eq!(toggled, vec![AgentsIntent::ToggleAgentsSection]);
    }

    #[test]
    fn the_hidden_profiles_row_toggles_and_show_brings_one_back() {
        let toggled = pressed(panel(agents()), "Hidden profiles (1)");
        let shown = pressed(panel(agents()), i18n::t(Key::AgentsShowProfile));

        assert_eq!(toggled, vec![AgentsIntent::ToggleHiddenProfiles]);
        assert_eq!(shown, vec![AgentsIntent::ShowProfile(old_codex())]);
    }

    #[test]
    fn before_the_first_read_the_note_shows_and_add_is_off() {
        let loading = AgentsSectionViewModel {
            profiles: Vec::new(),
            hidden: None,
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
    fn the_body_height_counts_the_rows_and_the_note() {
        let closed_hidden = AgentsSectionViewModel {
            hidden: Some(HiddenProfilesViewModel {
                is_open: false,
                ..agents().hidden.expect("a hidden row")
            }),
            ..agents()
        };
        let loading = AgentsSectionViewModel {
            profiles: Vec::new(),
            hidden: None,
            note: Some("…"),
            ..agents()
        };

        assert_eq!(agents_content_height(&agents()), 3.0 * size::ROW);
        assert_eq!(agents_content_height(&closed_hidden), 2.0 * size::ROW);
        assert_eq!(agents_content_height(&loading), note_height());
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

        let title = at(i18n::t(Key::AgentsSectionAgents));
        let name = at("Codex");
        let number = at("3 760 left");
        let hidden = at("Hidden profiles (1)");
        let show = at(i18n::t(Key::AgentsShowProfile));

        let buttons = LEAST_WIDTH
            - HEADER_END
            - 2.0 * size::CONTROL_SM
            - spacing::SPACE_2;
        assert!(title.x + title.width <= buttons, "{title:?}");
        assert!(name.x + name.width <= number.x, "{name:?} {number:?}");
        for text in [number, hidden, show] {
            assert!(text.x + text.width <= LEAST_WIDTH, "{text:?}");
        }
    }
}
