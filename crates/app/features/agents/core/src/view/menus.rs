//! The menus of Agents mode, opened at the pointer: the funnel menu of the
//! projects, the profiles of New session, the menu of a session and the
//! menu of a profile.

use iced::Element;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::context_menu;
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::palette::UiPalette;

use super::super::event::{AgentsIntent, SessionAction};
use super::super::model::{
    FilterMenuViewModel, MenuViewModel, NewSessionMenuViewModel,
    ProfileMenuViewModel, SessionMenuViewModel,
};
use super::super::project_filter::ProjectFilterChange;

/// The key that continues the selected session, shown after Continue.
const ENTER_KEY: &str = "Enter";

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
        MenuViewModel::Session(vm) => {
            let place = vm.place;
            (session_menu(&vm, palette), place)
        },
        MenuViewModel::Profile(vm) => {
            let place = vm.place;
            (profile_menu(vm, palette), place)
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

/// The menu of a session: Continue (Enter does it too), Continue in a
/// split, Copy resume command, Copy session ID, then Open folder in
/// terminal. An item that can do nothing is drawn faded and sends nothing.
fn session_menu(
    vm: &SessionMenuViewModel,
    palette: &UiPalette,
) -> Menu<AgentsIntent> {
    let item = |key: Key, action: SessionAction, is_enabled: bool| {
        let item = MenuItem::new(i18n::t(key));
        if !is_enabled {
            return item;
        }

        item.on_press(AgentsIntent::SessionMenu(action))
    };

    Menu::new(palette)
        .item(
            item(
                Key::AgentsContinue,
                SessionAction::Continue,
                vm.can_continue,
            )
            .shortcut(ENTER_KEY),
        )
        .item(item(
            Key::AgentsContinueInSplit,
            SessionAction::ContinueInSplit,
            vm.can_continue,
        ))
        .item(item(
            Key::AgentsCopyResumeCommand,
            SessionAction::CopyResumeCommand,
            vm.can_copy_command,
        ))
        .item(item(
            Key::AgentsCopySessionId,
            SessionAction::CopySessionId,
            true,
        ))
        .separator()
        .item(item(
            Key::AgentsOpenInTerminal,
            SessionAction::OpenFolder,
            vm.can_open_folder,
        ))
}

/// The menu of a profile's dashboard: Hide, and Remove for an added
/// profile.
fn profile_menu(
    vm: ProfileMenuViewModel,
    palette: &UiPalette,
) -> Menu<AgentsIntent> {
    let mut menu = Menu::new(palette).item(
        MenuItem::new(i18n::t(Key::AgentsHide))
            .on_press(AgentsIntent::HideProfile(vm.profile.clone())),
    );
    if vm.can_remove {
        menu = menu.item(
            MenuItem::new(i18n::t(Key::ButtonRemove))
                .danger(true)
                .on_press(AgentsIntent::RemoveProfile(vm.profile)),
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

    fn session_menu_with(
        can_continue: bool,
        can_open_folder: bool,
    ) -> MenuViewModel {
        MenuViewModel::Session(SessionMenuViewModel {
            place: place(),
            can_continue,
            can_copy_command: true,
            can_open_folder,
        })
    }

    #[test]
    fn each_item_of_the_session_menu_sends_its_action() {
        let items = [
            (Key::AgentsContinue, SessionAction::Continue),
            (Key::AgentsContinueInSplit, SessionAction::ContinueInSplit),
            (
                Key::AgentsCopyResumeCommand,
                SessionAction::CopyResumeCommand,
            ),
            (Key::AgentsCopySessionId, SessionAction::CopySessionId),
            (Key::AgentsOpenInTerminal, SessionAction::OpenFolder),
        ];

        for (key, action) in items {
            let sent = pressed(session_menu_with(true, true), i18n::t(key));

            assert_eq!(sent, vec![AgentsIntent::SessionMenu(action)]);
        }
    }

    #[test]
    fn the_items_of_a_session_that_can_not_open_send_nothing() {
        let continued = pressed(
            session_menu_with(false, false),
            i18n::t(Key::AgentsContinue),
        );
        let split = pressed(
            session_menu_with(false, false),
            i18n::t(Key::AgentsContinueInSplit),
        );
        let folder = pressed(
            session_menu_with(false, false),
            i18n::t(Key::AgentsOpenInTerminal),
        );
        let id = pressed(
            session_menu_with(false, false),
            i18n::t(Key::AgentsCopySessionId),
        );

        assert!(continued.is_empty());
        assert!(split.is_empty());
        assert!(folder.is_empty());
        assert_eq!(
            id,
            vec![AgentsIntent::SessionMenu(SessionAction::CopySessionId)]
        );
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

    #[test]
    fn the_profile_menu_hides_the_profile_and_removes_an_added_one() {
        let work = Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Work",
            PathBuf::from("/home/user/.claude-work"),
        );
        let menu = |can_remove| {
            MenuViewModel::Profile(ProfileMenuViewModel {
                place: place(),
                profile: work.clone(),
                can_remove,
            })
        };

        let hidden = pressed(menu(true), i18n::t(Key::AgentsHide));
        let removed = pressed(menu(true), i18n::t(Key::ButtonRemove));
        let themes = ThemeManager::new();
        let mut default = simulator(view(MenuProps {
            vm: menu(false),
            theme: ThemeProps::new(themes.current()),
        }));

        assert_eq!(hidden, vec![AgentsIntent::HideProfile(work.clone())]);
        assert_eq!(removed, vec![AgentsIntent::RemoveProfile(work.clone())]);
        assert!(default.find(i18n::t(Key::ButtonRemove)).is_err());
    }
}
