use iced::widget::{Space, Stack, container, mouse_area};
use iced::{Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::geometry::{anchor_position, menu_height_for_items};
use otty_ui_kit::spacing;

use crate::event::{ExplorerIntent, ExplorerMenuAction};
use crate::interaction::MenuTarget;
use crate::model::ContextMenuViewModel;

/// Height of one separator: the 1px line plus its 4px margins.
const SEPARATOR_HEIGHT: f32 = 9.0;

/// One entry of the menu: an item, or a line between groups of items.
enum Entry {
    Item(Key, ExplorerMenuAction),
    DangerItem(Key, ExplorerMenuAction),
    Separator,
}

/// Render the Files context menu with the dismiss layer over the main
/// area, kept inside the main area it opened in. Its items depend on the
/// row it opened on.
pub(crate) fn view<'a>(
    vm: ContextMenuViewModel<'_>,
    theme: ThemeProps<'a>,
) -> Element<'a, ExplorerIntent> {
    let palette = theme.theme.ui();
    let entries = entries(vm.target);

    let mut menu = Menu::new(palette);
    let mut item_count = 0;
    let mut separator_count = 0;
    for entry in entries {
        menu = match entry {
            Entry::Item(key, action) => {
                item_count += 1;
                menu.item(item(key, action, false))
            },
            Entry::DangerItem(key, action) => {
                item_count += 1;
                menu.item(item(key, action, true))
            },
            Entry::Separator => {
                separator_count += 1;
                menu.separator()
            },
        };
    }

    let height = menu_height_for_items(item_count)
        + SEPARATOR_HEIGHT * separator_count as f32;
    let anchor = anchor_position(
        vm.cursor,
        vm.area_size,
        menu::MENU_WIDTH,
        height,
        spacing::SPACE_2,
    );
    let positioned_menu = container(Element::from(menu)).padding(Padding {
        top: anchor.y,
        right: 0.0,
        bottom: 0.0,
        left: anchor.x,
    });

    // Dismiss layer: a press of any button away from the menu closes it.
    let dismiss_layer = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(ExplorerIntent::ContextMenuDismiss)
    .on_right_press(ExplorerIntent::ContextMenuDismiss)
    .on_middle_press(ExplorerIntent::ContextMenuDismiss);

    Stack::with_children(vec![dismiss_layer.into(), positioned_menu.into()])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The entries of the menu of `target`: a file opens, is renamed or
/// deleted; a folder also opens a terminal and takes new entries; the empty
/// area acts on the followed folder.
fn entries(target: &MenuTarget) -> Vec<Entry> {
    use ExplorerMenuAction::*;

    match target {
        MenuTarget::File(_) => vec![
            Entry::Item(Key::FilesOpen, Open),
            Entry::Item(Key::Rename, Rename),
            Entry::Separator,
            Entry::DangerItem(Key::Delete, Delete),
        ],
        MenuTarget::Folder(_) => vec![
            Entry::Item(Key::FilesOpen, Open),
            Entry::Item(Key::FilesOpenInTerminal, OpenInTerminal),
            Entry::Separator,
            Entry::Item(Key::FilesNewFile, NewFile),
            Entry::Item(Key::MenuNewFolder, NewFolder),
            Entry::Separator,
            Entry::Item(Key::Rename, Rename),
            Entry::Separator,
            Entry::DangerItem(Key::Delete, Delete),
        ],
        MenuTarget::Background => vec![
            Entry::Item(Key::FilesOpenInTerminal, OpenInTerminal),
            Entry::Separator,
            Entry::Item(Key::FilesNewFile, NewFile),
            Entry::Item(Key::MenuNewFolder, NewFolder),
        ],
    }
}

fn item(
    key: Key,
    action: ExplorerMenuAction,
    is_danger: bool,
) -> MenuItem<ExplorerIntent> {
    MenuItem::new(i18n::t(key))
        .danger(is_danger)
        .on_press(ExplorerIntent::ContextMenuAction(action))
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::{ThemeManager, ThemeProps};

    use super::view;
    use crate::event::{ExplorerIntent, ExplorerMenuAction};
    use crate::interaction::MenuTarget;
    use crate::model::ContextMenuViewModel;

    fn menu_of(target: &MenuTarget) -> ContextMenuViewModel<'_> {
        ContextMenuViewModel {
            target,
            cursor: Point::new(40.0, 40.0),
            area_size: Size::new(1024.0, 768.0),
        }
    }

    /// Click the item `key` in the menu of `target` and return what it
    /// sent.
    fn click(target: MenuTarget, key: Key) -> Vec<ExplorerIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(
            menu_of(&target),
            ThemeProps::new(themes.current()),
        ));

        ui.click(i18n::t(key)).expect("the item should show");

        ui.into_messages().collect()
    }

    fn sent_action(intents: &[ExplorerIntent]) -> Option<ExplorerMenuAction> {
        match intents {
            [ExplorerIntent::ContextMenuAction(action)] => Some(*action),
            _ => None,
        }
    }

    fn folder() -> MenuTarget {
        MenuTarget::Folder(vec![String::from("src")])
    }

    fn file() -> MenuTarget {
        MenuTarget::File(vec![String::from("main.rs")])
    }

    #[test]
    fn the_folder_menu_has_every_folder_item() {
        let items = [
            (Key::FilesOpen, ExplorerMenuAction::Open),
            (Key::FilesOpenInTerminal, ExplorerMenuAction::OpenInTerminal),
            (Key::FilesNewFile, ExplorerMenuAction::NewFile),
            (Key::MenuNewFolder, ExplorerMenuAction::NewFolder),
            (Key::Rename, ExplorerMenuAction::Rename),
            (Key::Delete, ExplorerMenuAction::Delete),
        ];

        for (key, action) in items {
            assert_eq!(sent_action(&click(folder(), key)), Some(action));
        }
    }

    #[test]
    fn the_file_menu_opens_renames_and_deletes() {
        let items = [
            (Key::FilesOpen, ExplorerMenuAction::Open),
            (Key::Rename, ExplorerMenuAction::Rename),
            (Key::Delete, ExplorerMenuAction::Delete),
        ];

        for (key, action) in items {
            assert_eq!(sent_action(&click(file(), key)), Some(action));
        }
    }

    /// Open in terminal is a folder item: a file has no such item.
    #[test]
    fn the_file_menu_has_no_terminal_and_no_new_entries() {
        let themes = ThemeManager::new();
        let target = file();
        let mut ui = simulator(view(
            menu_of(&target),
            ThemeProps::new(themes.current()),
        ));

        assert!(ui.find(i18n::t(Key::FilesOpenInTerminal)).is_err());
        assert!(ui.find(i18n::t(Key::FilesNewFile)).is_err());
    }

    #[test]
    fn the_empty_area_menu_opens_a_terminal_and_takes_new_entries() {
        let items = [
            (Key::FilesOpenInTerminal, ExplorerMenuAction::OpenInTerminal),
            (Key::FilesNewFile, ExplorerMenuAction::NewFile),
            (Key::MenuNewFolder, ExplorerMenuAction::NewFolder),
        ];

        for (key, action) in items {
            let intents = click(MenuTarget::Background, key);
            assert_eq!(sent_action(&intents), Some(action));
        }
    }

    /// A press away from the menu closes it instead of reaching the tree
    /// or the terminal under it.
    #[test]
    fn a_press_outside_the_menu_closes_it() {
        let themes = ThemeManager::new();
        let target = file();
        let mut ui = simulator(view(
            menu_of(&target),
            ThemeProps::new(themes.current()),
        ));

        ui.point_at(Point::new(900.0, 700.0));
        let _ = ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Right,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Right,
            )),
        ]);

        let intents: Vec<ExplorerIntent> = ui.into_messages().collect();
        assert!(matches!(
            intents.as_slice(),
            [ExplorerIntent::ContextMenuDismiss]
        ));
    }
}
