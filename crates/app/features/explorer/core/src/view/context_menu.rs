use iced::Element;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::context_menu;
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::palette::UiPalette;

use crate::event::{ExplorerIntent, ExplorerMenuAction};
use crate::interaction::MenuTarget;
use crate::model::ContextMenuViewModel;

/// Render the Files context menu at the pointer, kept inside the main area
/// it opened in. A press away from it closes it.
pub(crate) fn view<'a>(
    vm: ContextMenuViewModel<'_>,
    theme: ThemeProps<'a>,
) -> Element<'a, ExplorerIntent> {
    context_menu(
        menu(vm.target, theme.theme.ui()),
        vm.cursor,
        vm.area_size,
        ExplorerIntent::ContextMenuDismiss,
    )
}

/// The menu of `target`: a file opens, is renamed or deleted; a folder also
/// opens a terminal and takes new entries; the empty area acts on the
/// followed folder.
fn menu(target: &MenuTarget, palette: &UiPalette) -> Menu<ExplorerIntent> {
    use ExplorerMenuAction::*;

    let menu = Menu::new(palette);

    match target {
        MenuTarget::File(_) => menu
            .item(item(Key::FilesOpen, Open))
            .item(item(Key::Rename, Rename))
            .separator()
            .item(item(Key::Delete, Delete).danger(true)),
        MenuTarget::Folder(_) => menu
            .item(item(Key::FilesOpen, Open))
            .item(item(Key::FilesOpenInTerminal, OpenInTerminal))
            .separator()
            .item(item(Key::FilesNewFile, NewFile))
            .item(item(Key::MenuNewFolder, NewFolder))
            .separator()
            .item(item(Key::Rename, Rename))
            .separator()
            .item(item(Key::Delete, Delete).danger(true)),
        MenuTarget::Background => menu
            .item(item(Key::FilesOpenInTerminal, OpenInTerminal))
            .separator()
            .item(item(Key::FilesNewFile, NewFile))
            .item(item(Key::MenuNewFolder, NewFolder)),
    }
}

fn item(key: Key, action: ExplorerMenuAction) -> MenuItem<ExplorerIntent> {
    MenuItem::new(i18n::t(key))
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
