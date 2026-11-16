//! The menus of the Agents mode left panel, opened at the pointer: the
//! funnel menu of the projects and the profiles of New session.

use iced::Element;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::context_menu;
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::{
    FilterMenuViewModel, MenuViewModel, NewSessionMenuViewModel,
};
use super::super::project_filter::ProjectFilterChange;

/// Props for the open menu.
pub(crate) struct MenuProps<'a> {
    pub(crate) vm: MenuViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the open menu at the pointer, kept inside the main area. A press
/// of any button away from it closes it.
pub(crate) fn view(props: MenuProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();

    let (menu, place) = match props.vm {
        MenuViewModel::ProjectFilter(vm) => {
            let place = vm.place;
            (filter_menu(vm, palette), place)
        },
        MenuViewModel::NewSession(vm) => {
            let place = vm.place;
            (new_session_menu(vm, palette), place)
        },
    };

    context_menu(menu, place.cursor(), place.area(), AgentsIntent::CloseMenu)
}

/// The funnel menu: all agents or one, the projects of the last 30 days,
/// the projects whose folder is gone, the sort by name and Clear all
/// filters. A choice that is on is drawn lit; pressing it turns it off.
fn filter_menu(
    vm: FilterMenuViewModel,
    palette: &UiPalette,
) -> Menu<AgentsIntent> {
    let filter = vm.filter;
    let change = AgentsIntent::ProjectFilter;

    let mut menu = Menu::new(palette)
        .heading(i18n::t(Key::AgentsFilterAgent))
        .item(
            MenuItem::new(i18n::t(Key::AgentsFilterAllAgents))
                .active(filter.agent().is_none())
                .on_press(change(ProjectFilterChange::Agent(None))),
        );
    for (agent, name) in vm.agents {
        menu = menu.item(
            MenuItem::new(name)
                .active(filter.agent() == Some(agent))
                .on_press(change(ProjectFilterChange::Agent(Some(agent)))),
        );
    }

    let mut clear_all = MenuItem::new(i18n::t(Key::AgentsFilterClearAll));
    if filter.is_any_on() {
        clear_all = clear_all.on_press(change(ProjectFilterChange::ClearAll));
    }

    menu.heading(i18n::t(Key::AgentsFilterShow))
        .item(
            MenuItem::new(i18n::t(Key::AgentsFilterRecent))
                .active(filter.is_recent_only())
                .on_press(change(ProjectFilterChange::RecentOnly(
                    !filter.is_recent_only(),
                ))),
        )
        .item(
            MenuItem::new(i18n::t(Key::AgentsFilterMissing))
                .active(filter.is_missing_only())
                .on_press(change(ProjectFilterChange::MissingOnly(
                    !filter.is_missing_only(),
                ))),
        )
        .separator()
        .item(
            MenuItem::new(i18n::t(Key::AgentsFilterSortByName))
                .active(filter.is_sorted_by_name())
                .on_press(change(ProjectFilterChange::SortByName(
                    !filter.is_sorted_by_name(),
                ))),
        )
        .item(clear_all)
}

/// The profiles a new session can start with; a press starts it.
fn new_session_menu(
    vm: NewSessionMenuViewModel,
    palette: &UiPalette,
) -> Menu<AgentsIntent> {
    let mut menu = Menu::new(palette);
    for profile in vm.profiles {
        let name = profile.name().to_string();
        menu = menu.item(
            MenuItem::new(name).on_press(AgentsIntent::NewSessionWith(profile)),
        );
    }

    menu
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_agents::{AgentId, Profile};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::menus::MenuPlace;
    use crate::project_filter::ProjectFilter;

    const CODEX: AgentId = AgentId::new("codex");

    fn place() -> MenuPlace {
        MenuPlace::new(Point::new(40.0, 40.0), Size::new(800.0, 600.0))
    }

    fn filter_menu_with(filter: ProjectFilter) -> MenuViewModel {
        MenuViewModel::ProjectFilter(FilterMenuViewModel {
            place: place(),
            agents: vec![
                (AgentId::new("claude-code"), "Claude Code"),
                (CODEX, "Codex"),
            ],
            filter,
        })
    }

    fn pressed(vm: MenuViewModel, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MenuProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(label).expect("the item should show");

        ui.into_messages().collect()
    }

    #[test]
    fn the_funnel_menu_picks_an_agent_and_turns_choices_on() {
        let codex =
            pressed(filter_menu_with(ProjectFilter::default()), "Codex");
        let all = pressed(
            filter_menu_with(ProjectFilter::default()),
            i18n::t(Key::AgentsFilterAllAgents),
        );
        let recent = pressed(
            filter_menu_with(ProjectFilter::default()),
            i18n::t(Key::AgentsFilterRecent),
        );
        let missing = pressed(
            filter_menu_with(ProjectFilter::default()),
            i18n::t(Key::AgentsFilterMissing),
        );
        let sorted = pressed(
            filter_menu_with(ProjectFilter::default()),
            i18n::t(Key::AgentsFilterSortByName),
        );

        assert_eq!(
            codex,
            vec![AgentsIntent::ProjectFilter(ProjectFilterChange::Agent(
                Some(CODEX)
            ))]
        );
        assert_eq!(
            all,
            vec![AgentsIntent::ProjectFilter(ProjectFilterChange::Agent(
                None
            ))]
        );
        assert_eq!(
            recent,
            vec![AgentsIntent::ProjectFilter(
                ProjectFilterChange::RecentOnly(true)
            )]
        );
        assert_eq!(
            missing,
            vec![AgentsIntent::ProjectFilter(
                ProjectFilterChange::MissingOnly(true)
            )]
        );
        assert_eq!(
            sorted,
            vec![AgentsIntent::ProjectFilter(
                ProjectFilterChange::SortByName(true)
            )]
        );
    }

    #[test]
    fn a_choice_that_is_on_turns_off_and_clear_all_works_only_then() {
        let mut on = ProjectFilter::default();
        on.apply(ProjectFilterChange::RecentOnly(true));

        let recent = pressed(
            filter_menu_with(on.clone()),
            i18n::t(Key::AgentsFilterRecent),
        );
        let cleared =
            pressed(filter_menu_with(on), i18n::t(Key::AgentsFilterClearAll));
        let nothing_to_clear = pressed(
            filter_menu_with(ProjectFilter::default()),
            i18n::t(Key::AgentsFilterClearAll),
        );

        assert_eq!(
            recent,
            vec![AgentsIntent::ProjectFilter(
                ProjectFilterChange::RecentOnly(false)
            )]
        );
        assert_eq!(
            cleared,
            vec![AgentsIntent::ProjectFilter(ProjectFilterChange::ClearAll)]
        );
        assert!(nothing_to_clear.is_empty());
    }

    #[test]
    fn a_profile_of_the_new_session_menu_starts_the_session() {
        let codex = Profile::default_for(
            CODEX,
            "Codex",
            PathBuf::from("/home/user/.codex"),
        );
        let vm = MenuViewModel::NewSession(NewSessionMenuViewModel {
            place: place(),
            profiles: vec![codex.clone()],
        });

        let started = pressed(vm, "Codex");

        assert_eq!(started, vec![AgentsIntent::NewSessionWith(codex)]);
    }
}
