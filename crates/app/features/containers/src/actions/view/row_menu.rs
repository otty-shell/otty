//! The menu of a table row, opened by a right click: the same actions as
//! the inspector buttons, at the pointer.

use iced::widget::{Space, Stack, container, mouse_area, opaque};
use iced::{Element, Length, Padding, Size};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::geometry::{anchor_position, menu_height_for_items};
use otty_ui_kit::spacing;

use super::super::event::ActionsIntent;
use super::super::model::RowMenuViewModel;

/// Props for the row menu.
pub(crate) struct RowMenuProps<'a> {
    pub(crate) vm: RowMenuViewModel,
    /// The window size the menu is kept inside.
    pub(crate) area: Size,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the row menu over the whole window. A press of any button outside
/// it closes it.
pub(crate) fn view(props: RowMenuProps<'_>) -> Element<'_, ActionsIntent> {
    let palette = props.theme.theme.ui();

    let height = menu_height_for_items(props.vm.items.len());
    let anchor = anchor_position(
        props.vm.position,
        props.area,
        menu::MENU_WIDTH,
        height,
        spacing::SPACE_2,
    );

    let mut list = Menu::new(palette);
    for item in props.vm.items {
        let mut entry = MenuItem::new(item.label).danger(item.is_danger);
        if item.is_enabled {
            entry = entry.on_press(ActionsIntent::Act(item.action));
        }
        list = list.item(entry);
    }

    // `opaque` keeps a press on a disabled item from reaching the dismiss
    // layer under the menu.
    let positioned = container(opaque(list)).padding(Padding {
        top: anchor.y,
        right: 0.0,
        bottom: 0.0,
        left: anchor.x,
    });
    let dismiss = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(ActionsIntent::CloseRowMenu)
    .on_right_press(ActionsIntent::CloseRowMenu)
    .on_middle_press(ActionsIntent::CloseRowMenu);

    Stack::with_children(vec![dismiss.into(), positioned.into()])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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
