//! The Agents mode left panel under the switcher: the profiles with their
//! state, then the projects with their sessions count.

use iced::widget::{Column, container, row, scrollable, svg, text};
use iced::{Alignment, Color, Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::session_row::{SessionRow, SessionTone};
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    HiddenProfilesViewModel, LeftPanelViewModel, ProfileRowViewModel,
    ProjectRowViewModel,
};

/// Props for the left panel.
pub(crate) struct LeftPanelProps<'a> {
    pub(crate) vm: LeftPanelViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the left panel. A press on a profile shows its details in the
/// main area, a press on a project its sessions. Add profile and the hidden
/// profiles follow the profiles.
pub(crate) fn view(props: LeftPanelProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut list = Column::new().width(Length::Fill);
    if !vm.profiles.is_empty() {
        list = list.push(label(i18n::t(Key::AgentsProfiles), palette));
    }
    for profile in vm.profiles {
        list = list.push(profile_row(profile, palette));
    }
    if let Some(hidden) = vm.hidden {
        list = list.push(hidden_profiles(hidden, palette));
    }
    if vm.can_add {
        list = list.push(
            container(
                Button::new(i18n::t(Key::AgentsAddProfile), palette)
                    .kind(ButtonKind::Ghost)
                    .size(ButtonSize::Small)
                    .on_press(AgentsIntent::OpenAddProfile),
            )
            .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_1])),
        );
    }
    if !vm.projects.is_empty() {
        list = list.push(label(i18n::t(Key::AgentsProjects), palette));
    }
    for project in vm.projects {
        list = list.push(project_row(project, palette));
    }
    if let Some(note) = vm.note {
        list = list.push(small_text(note, palette.text_muted()));
    }

    let body = scrollable(list)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette));

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: spacing::SPACE_1,
            right: spacing::SPACE_3,
            bottom: spacing::SPACE_3,
            left: spacing::SPACE_3,
        })
        .into()
}

/// A profile: the icon of its agent, its name and its state, and why its
/// sessions could not be read under it.
fn profile_row<'a>(
    vm: ProfileRowViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let icon = svg::Handle::from_memory(icons::agent_icon(vm.icon));
    let row: Element<'a, AgentsIntent> = TreeRow::new(vm.name, palette)
        .icon(icon, None)
        .state(vm.state)
        .count_tone(vm.tone)
        .selected(vm.is_selected)
        .on_press(AgentsIntent::SelectProfile(vm.profile))
        .into();

    let Some(failure) = vm.failure else {
        return row;
    };

    Column::new()
        .push(row)
        .push(small_text(failure, palette.status_error()))
        .into()
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
            .align_y(Alignment::Center),
        );
    }

    list.into()
}

/// A project: its folder name, its path as a hint and its sessions count,
/// or that its folder is missing.
fn project_row<'a>(
    vm: ProjectRowViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let (state, tone) = if vm.is_missing {
        (
            i18n::t(Key::AgentsProjectMissing).to_string(),
            SessionTone::Failed,
        )
    } else {
        (vm.sessions.to_string(), SessionTone::Muted)
    };

    SessionRow::new(vm.name, palette)
        .folder(vm.path)
        .state(state, tone)
        .selected(vm.is_selected)
        .on_press(AgentsIntent::SelectProject(vm.folder))
        .into()
}

/// The muted label above the profiles or the projects.
fn label<'a>(
    content: &'a str,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let caption = typography::CAPTION;

    container(
        text(content)
            .size(caption.size())
            .line_height(iced::Pixels(caption.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted()),
    )
    .padding(Padding {
        top: spacing::SPACE_3,
        bottom: spacing::SPACE_1,
        ..Padding::from([0.0, spacing::SPACE_3])
    })
    .into()
}

fn small_text<'a>(
    content: impl text::IntoFragment<'a>,
    color: Color,
) -> Element<'a, AgentsIntent> {
    let small = typography::SMALL;

    container(
        text(content)
            .size(small.size())
            .line_height(iced::Pixels(small.line_height()))
            .font(fonts::UI)
            .color(color),
    )
    .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3]))
    .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_agents::{AgentId, Profile};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::HiddenProfileViewModel;

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

    fn pressed(vm: LeftPanelViewModel, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(LeftPanelProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(label).expect("the label should show");

        ui.into_messages().collect()
    }

    fn panel_with_codex() -> LeftPanelViewModel {
        LeftPanelViewModel {
            profiles: vec![ProfileRowViewModel {
                profile: codex(),
                is_selected: false,
                icon: "codex",
                name: String::from("Codex"),
                state: String::from("connected"),
                tone: SessionTone::Muted,
                failure: None,
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
            projects: Vec::new(),
            note: None,
        }
    }

    #[test]
    fn a_press_on_a_profile_selects_it_and_add_profile_opens_the_dialog() {
        let selected = pressed(panel_with_codex(), "Codex");
        let added = pressed(panel_with_codex(), i18n::t(Key::AgentsAddProfile));

        assert_eq!(selected, vec![AgentsIntent::SelectProfile(codex())]);
        assert_eq!(added, vec![AgentsIntent::OpenAddProfile]);
    }

    #[test]
    fn the_hidden_profiles_row_toggles_and_show_brings_one_back() {
        let toggled = pressed(panel_with_codex(), "Hidden profiles (1)");
        let shown =
            pressed(panel_with_codex(), i18n::t(Key::AgentsShowProfile));

        assert_eq!(toggled, vec![AgentsIntent::ToggleHiddenProfiles]);
        assert_eq!(shown, vec![AgentsIntent::ShowProfile(old_codex())]);
    }

    fn project(name: &str, is_missing: bool) -> ProjectRowViewModel {
        ProjectRowViewModel {
            folder: PathBuf::from(format!("/work/{name}")),
            name: String::from(name),
            path: format!("/work/{name}"),
            sessions: 3,
            is_missing,
            is_selected: false,
        }
    }

    #[test]
    fn the_panel_shows_the_profiles_and_a_press_on_a_project_selects_it() {
        let vm = LeftPanelViewModel {
            profiles: vec![ProfileRowViewModel {
                profile: codex(),
                is_selected: false,
                icon: "codex",
                name: String::from("Codex"),
                state: String::from("sessions not read"),
                tone: SessionTone::Failed,
                failure: Some(String::from("files: no folder")),
            }],
            hidden: None,
            can_add: false,
            projects: vec![project("otty", false), project("gone", true)],
            note: None,
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(LeftPanelProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Codex").expect("the profile should show");
        ui.find("files: no folder")
            .expect("the failure should show");
        ui.find(i18n::t(Key::AgentsProjectMissing))
            .expect("the missing project should say so");
        ui.click("otty").expect("the project should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![AgentsIntent::SelectProject(PathBuf::from("/work/otty"))]
        );
    }

    #[test]
    fn a_note_shows_when_there_is_nothing_to_list() {
        let vm = LeftPanelViewModel {
            profiles: Vec::new(),
            hidden: None,
            can_add: false,
            projects: Vec::new(),
            note: Some(i18n::t(Key::AgentsLoading)),
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(LeftPanelProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::AgentsLoading))
            .expect("the note should show");
        assert!(ui.find(i18n::t(Key::AgentsProfiles)).is_err());
    }
}
