//! The New session menu under the Sessions header. It is a header menu of
//! the sidebar, so the frame opens, places and closes it; its items belong
//! to several features (a terminal tab, a split, the launches), so the
//! frame builds it from them and gives each item its message.

use otty_app_domain::shortcuts::AppShortcut;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::{Menu, MenuItem};

/// Width of the New session menu, from the design.
const NEW_SESSION_MENU_WIDTH: f32 = 280.0;

/// Props for the New session menu.
///
/// The menu is generic over the message type: the caller gives the ready
/// message each item sends.
pub(crate) struct NewSessionMenuProps<'a, Message> {
    pub(crate) theme: ThemeProps<'a>,
    /// Shell name, shown next to "New terminal".
    pub(crate) shell_name: &'a str,
    /// Message sent by "New terminal".
    pub(crate) on_new_terminal: Message,
    /// Message sent by "Split current pane right"; `None` disables the item.
    pub(crate) on_split_right: Option<Message>,
    /// Message sent by "Split current pane down"; `None` disables the item.
    pub(crate) on_split_down: Option<Message>,
    /// Launches at the top level of the launch tree: the title and the
    /// message that runs the launch.
    pub(crate) launches: Vec<(&'a str, Message)>,
    /// Message sent by "New launch".
    pub(crate) on_new_launch: Message,
}

/// The New session menu: New terminal, Split current pane right and down,
/// the top-level launches, and New launch.
pub(crate) fn menu<Message>(
    props: NewSessionMenuProps<'_, Message>,
) -> Menu<Message> {
    let palette = props.theme.theme.ui();

    let new_terminal =
        format!("{} ({})", i18n::t(Key::MenuNewTerminal), props.shell_name);
    let new_terminal = MenuItem::new(new_terminal)
        .shortcut(AppShortcut::NewTerminal.label())
        .on_press(props.on_new_terminal);

    let mut split_right = MenuItem::new(i18n::t(Key::MenuSplitCurrentPane))
        .shortcut(AppShortcut::SplitRight.label());
    if let Some(on_split) = props.on_split_right {
        split_right = split_right.on_press(on_split);
    }
    let mut split_down = MenuItem::new(i18n::t(Key::MenuSplitCurrentPaneDown))
        .shortcut(AppShortcut::SplitDown.label());
    if let Some(on_split) = props.on_split_down {
        split_down = split_down.on_press(on_split);
    }

    let mut menu = Menu::new(palette)
        .width(NEW_SESSION_MENU_WIDTH)
        .item(new_terminal)
        .item(split_right)
        .item(split_down)
        .separator();

    if !props.launches.is_empty() {
        menu = menu.heading(i18n::t(Key::LaunchesPanelTitle));
        for (title, on_launch) in props.launches {
            menu = menu.item(MenuItem::new(title).on_press(on_launch));
        }
        menu = menu.separator();
    }

    menu.item(
        MenuItem::new(i18n::t(Key::MenuNewLaunch))
            .on_press(props.on_new_launch),
    )
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::{ThemeManager, ThemeProps};
    use otty_ui_kit::size;

    use super::{Menu, NEW_SESSION_MENU_WIDTH, NewSessionMenuProps};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        NewTerminal,
        SplitRight,
        SplitDown,
        Launch(&'static str),
        NewLaunch,
    }

    fn menu(
        themes: &ThemeManager,
        can_split: bool,
        launch_titles: Vec<&'static str>,
    ) -> Menu<Message> {
        let launches = launch_titles
            .into_iter()
            .map(|title| (title, Message::Launch(title)))
            .collect();

        super::menu(NewSessionMenuProps {
            theme: ThemeProps::new(themes.current()),
            shell_name: "zsh",
            on_new_terminal: Message::NewTerminal,
            on_split_right: can_split.then_some(Message::SplitRight),
            on_split_down: can_split.then_some(Message::SplitDown),
            launches,
            on_new_launch: Message::NewLaunch,
        })
    }

    fn click(
        can_split: bool,
        launch_titles: Vec<&'static str>,
        label: &str,
    ) -> Vec<Message> {
        let themes = ThemeManager::new();
        let menu = menu(&themes, can_split, launch_titles);
        let mut ui = simulator(iced::Element::from(menu));

        ui.click(label).expect("menu item should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn given_new_terminal_item_when_pressed_then_its_message_is_sent() {
        let messages = click(false, Vec::new(), "New terminal (zsh)");

        assert_eq!(messages, vec![Message::NewTerminal]);
    }

    #[test]
    fn given_split_message_when_split_item_pressed_then_it_is_sent() {
        let messages = click(true, Vec::new(), "Split current pane right");

        assert_eq!(messages, vec![Message::SplitRight]);
    }

    #[test]
    fn given_split_message_when_split_down_item_pressed_then_it_is_sent() {
        let messages = click(true, Vec::new(), "Split current pane down");

        assert_eq!(messages, vec![Message::SplitDown]);
    }

    #[test]
    fn given_no_split_message_when_split_item_pressed_then_nothing_is_sent() {
        let right = click(false, Vec::new(), "Split current pane right");
        let down = click(false, Vec::new(), "Split current pane down");

        assert!(right.is_empty());
        assert!(down.is_empty());
    }

    #[test]
    fn given_launch_item_when_pressed_then_that_launch_message_is_sent() {
        let messages = click(false, vec!["htop", "laptop"], "laptop");

        assert_eq!(messages, vec![Message::Launch("laptop")]);
    }

    #[test]
    fn given_new_launch_item_when_pressed_then_its_message_is_sent() {
        let messages = click(false, vec!["htop"], "New launch\u{2026}");

        assert_eq!(messages, vec![Message::NewLaunch]);
    }

    #[test]
    fn given_launches_when_viewed_then_the_launches_label_shows() {
        let themes = ThemeManager::new();
        let with = menu(&themes, false, vec!["htop"]);
        let without = menu(&themes, false, Vec::new());
        let mut with = simulator(iced::Element::from(with));
        let mut without = simulator(iced::Element::from(without));

        assert!(with.find("Launches").is_ok());
        assert!(without.find("Launches").is_err());
    }

    #[test]
    fn given_launches_when_sized_then_each_adds_one_row_of_the_wide_menu() {
        let themes = ThemeManager::new();
        let one = menu(&themes, false, vec!["htop"]).size();
        let two = menu(&themes, false, vec!["htop", "laptop"]).size();

        assert_eq!(two.height - one.height, size::ROW);
        assert_eq!(one.width, NEW_SESSION_MENU_WIDTH);
    }
}
