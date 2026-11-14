//! The menu of a table row, opened by a right click: the same actions as
//! the inspector buttons, at the pointer.

use iced::{Element, Size};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::context_menu;
use otty_ui_kit::components::menu::{Menu, MenuItem};

use super::super::event::ActionsIntent;
use super::super::model::RowMenuViewModel;

/// Props for the row menu.
pub(crate) struct RowMenuProps<'a> {
    pub(crate) vm: RowMenuViewModel,
    /// The window size the menu is kept inside.
    pub(crate) area: Size,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the row menu at the pointer, kept inside the window. A press of
/// any button away from it closes it.
pub(crate) fn view(props: RowMenuProps<'_>) -> Element<'_, ActionsIntent> {
    let mut menu = Menu::new(props.theme.theme.ui());
    for item in props.vm.items {
        let mut entry = MenuItem::new(item.label).danger(item.is_danger);
        if item.is_enabled {
            entry = entry.on_press(ActionsIntent::Act(item.action));
        }
        menu = menu.item(entry);
    }

    context_menu(
        menu,
        props.vm.position,
        props.area,
        ActionsIntent::CloseRowMenu,
    )
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_testing::mouse::middle_click;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::actions::model::ActionButton;
    use crate::actions::types::RowAction;

    fn item(action: RowAction, is_enabled: bool) -> ActionButton {
        ActionButton {
            action,
            label: i18n::t(action.label_key()),
            is_danger: action == RowAction::Remove,
            is_enabled,
        }
    }

    fn menu(items: Vec<ActionButton>) -> RowMenuViewModel {
        RowMenuViewModel {
            position: Point::new(100.0, 100.0),
            items,
        }
    }

    #[test]
    fn a_press_on_an_item_sends_its_action() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(RowMenuProps {
            vm: menu(vec![
                item(RowAction::Start, true),
                item(RowAction::Remove, true),
            ]),
            area: Size::new(800.0, 600.0),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ContainersActionRemove))
            .expect("remove should show");

        let intents: Vec<ActionsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![ActionsIntent::Act(RowAction::Remove)]);
    }

    #[test]
    fn a_disabled_item_sends_nothing() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(RowMenuProps {
            vm: menu(vec![item(RowAction::Stop, false)]),
            area: Size::new(800.0, 600.0),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ContainersActionStop))
            .expect("stop should show");

        let intents: Vec<ActionsIntent> = ui.into_messages().collect();
        assert!(intents.is_empty());
    }

    #[test]
    fn a_press_outside_the_menu_closes_it() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(RowMenuProps {
            vm: menu(vec![item(RowAction::Start, true)]),
            area: Size::new(800.0, 600.0),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.point_at(Point::new(700.0, 500.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let intents: Vec<ActionsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![ActionsIntent::CloseRowMenu]);
    }

    /// A middle press away from the menu closes it, as a left press does.
    #[test]
    fn a_middle_press_outside_the_menu_closes_it() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(RowMenuProps {
            vm: menu(vec![item(RowAction::Start, true)]),
            area: Size::new(800.0, 600.0),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.point_at(Point::new(700.0, 500.0));
        let _ = ui.simulate(middle_click());

        let intents: Vec<ActionsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![ActionsIntent::CloseRowMenu]);
    }
}
