use iced::Element;
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::size;

use crate::i18n::{self, Key};
use crate::shortcuts::AppShortcut;
use crate::theme::ThemeProps;

/// Width of the New session menu, from the design.
pub(crate) const NEW_SESSION_MENU_WIDTH: f32 = 280.0;

/// Height of one separator: 4px margin, a 1px line, 4px margin.
const SEPARATOR_HEIGHT: f32 = 9.0;
/// Height of the "Launches" label: 6px above, 4px below, one text line.
const HEADING_HEIGHT: f32 = 26.0;
/// New terminal, Split current pane right and New launch.
const FIXED_ITEMS: f32 = 3.0;

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
    pub(crate) on_split: Option<Message>,
    /// Launches at the top level of the launch tree: the title and the
    /// message that runs the launch.
    pub(crate) launches: Vec<(&'a str, Message)>,
    /// Message sent by "New launch".
    pub(crate) on_new_launch: Message,
}

/// Height of the menu for `launch_count` launches, used to keep it inside
/// the window.
pub(crate) fn menu_height(launch_count: usize) -> f32 {
    let frame = 2.0 * menu::MENU_PADDING;

    if launch_count == 0 {
        return frame + size::ROW * FIXED_ITEMS + SEPARATOR_HEIGHT;
    }

    frame
        + size::ROW * (FIXED_ITEMS + launch_count as f32)
        + 2.0 * SEPARATOR_HEIGHT
        + HEADING_HEIGHT
}

/// Render the New session menu: New terminal, Split current pane right, the
/// top-level launches, and New launch.
pub(crate) fn view<'a, Message: Clone + 'a>(
    props: NewSessionMenuProps<'a, Message>,
) -> Element<'a, Message> {
    let palette = props.theme.theme.ui();

    let new_terminal =
        format!("{} ({})", i18n::t(Key::MenuNewTerminal), props.shell_name);
    let new_terminal = MenuItem::new(new_terminal)
        .shortcut(AppShortcut::NewTerminal.label())
        .on_press(props.on_new_terminal);

    let mut split = MenuItem::new(i18n::t(Key::MenuSplitCurrentPane))
        .shortcut(AppShortcut::SplitPane.label());
    if let Some(on_split) = props.on_split {
        split = split.on_press(on_split);
    }

    let mut menu = Menu::new(palette)
        .width(NEW_SESSION_MENU_WIDTH)
        .item(new_terminal)
        .item(split)
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
    .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::container;
    use iced_test::simulator;
    use otty_ui_kit::size;

    use super::{NewSessionMenuProps, menu_height, view};
    use crate::theme::{ThemeManager, ThemeProps};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        NewTerminal,
        Split,
        Launch(&'static str),
        NewLaunch,
    }

    fn menu<'a>(
        themes: &'a ThemeManager,
        can_split: bool,
        launch_titles: Vec<&'static str>,
    ) -> iced::Element<'a, Message> {
        let launches = launch_titles
            .into_iter()
            .map(|title| (title, Message::Launch(title)))
            .collect();

        container(view(NewSessionMenuProps {
            theme: ThemeProps::new(themes.current()),
            shell_name: "zsh",
            on_new_terminal: Message::NewTerminal,
            on_split: can_split.then_some(Message::Split),
            launches,
            on_new_launch: Message::NewLaunch,
        }))
        .into()
    }

    fn click(
        can_split: bool,
        launch_titles: Vec<&'static str>,
        label: &str,
    ) -> Vec<Message> {
        let themes = ThemeManager::new();
        let mut ui = simulator(menu(&themes, can_split, launch_titles));

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

        assert_eq!(messages, vec![Message::Split]);
    }

    #[test]
    fn given_no_split_message_when_split_item_pressed_then_nothing_is_sent() {
        let messages = click(false, Vec::new(), "Split current pane right");

        assert!(messages.is_empty());
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
        let mut with = simulator(menu(&themes, false, vec!["htop"]));
        let mut without = simulator(menu(&themes, false, Vec::new()));

        assert!(with.find("Launches").is_ok());
        assert!(without.find("Launches").is_err());
    }

    #[test]
    fn given_launches_when_height_computed_then_each_adds_one_row() {
        assert_eq!(menu_height(2) - menu_height(1), size::ROW);
        assert!(menu_height(1) > menu_height(0));
    }
}
