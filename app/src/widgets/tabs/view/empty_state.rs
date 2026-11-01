//! Empty state shown in the main area when no session is open.

use std::sync::OnceLock;

use iced::widget::{container, image, svg};
use iced::{Element, Length};
use otty_ui_kit::components::action_tile::ActionTile;
use otty_ui_kit::components::empty_card::EmptyCard;

use crate::i18n::{self, Key};
use crate::icons;
use crate::shortcuts::AppShortcut;
use crate::theme::ThemeProps;

/// Props for the empty state.
pub(crate) struct EmptyStateProps<'a, Message> {
    pub(crate) theme: ThemeProps<'a>,
    /// Message sent by the New terminal tile.
    pub(crate) on_new_terminal: Message,
    /// Message sent by the New launch tile.
    pub(crate) on_new_launch: Message,
}

/// Render the start card centered in the main area: the logo, a title, a
/// description and two tiles, New terminal and New launch.
pub(crate) fn view<'a, Message: Clone + 'a>(
    props: EmptyStateProps<'a, Message>,
) -> Element<'a, Message> {
    let palette = props.theme.theme.ui();

    // The logo keeps its own colors; the card only gives it a slot.
    let logo = image(logo_handle().clone())
        .width(Length::Fill)
        .height(Length::Fill);

    let new_terminal = ActionTile::new(
        svg::Handle::from_memory(icons::ADD_TAB_HEADER),
        i18n::t(Key::EmptyNewTerminal),
        palette,
    )
    .shortcut(AppShortcut::NewTerminal.label())
    .accent(true)
    .on_press(props.on_new_terminal);

    let new_launch = ActionTile::new(
        svg::Handle::from_memory(icons::PLAY),
        i18n::t(Key::EmptyNewLaunch),
        palette,
    )
    .on_press(props.on_new_launch);

    let card = EmptyCard::new(
        i18n::t(Key::EmptyTitle),
        i18n::t(Key::EmptyDescription),
        logo,
        palette,
    )
    .action(new_terminal)
    .action(new_launch);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// Return the logo handle, created once. `Handle::from_bytes` gives every
/// new handle a new id, so a handle built on each render looks like a new
/// image to iced: it reloads the logo every frame and the logo flickers.
fn logo_handle() -> &'static image::Handle {
    static LOGO: OnceLock<image::Handle> = OnceLock::new();

    LOGO.get_or_init(|| image::Handle::from_bytes(icons::LOGO))
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::{EmptyStateProps, view};
    use crate::theme::{ThemeManager, ThemeProps};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        NewTerminal,
        NewLaunch,
    }

    fn card(themes: &ThemeManager) -> iced::Element<'_, Message> {
        view(EmptyStateProps {
            theme: ThemeProps::new(themes.current()),
            on_new_terminal: Message::NewTerminal,
            on_new_launch: Message::NewLaunch,
        })
    }

    #[test]
    fn given_empty_state_when_viewed_then_title_and_tiles_show() {
        let themes = ThemeManager::new();
        let mut ui = simulator(card(&themes));

        assert!(ui.find("Start a session").is_ok());
        assert!(ui.find("New terminal").is_ok());
        assert!(ui.find("New launch…").is_ok());
        assert!(ui.find("Terminal in a folder…").is_err());
    }

    #[test]
    fn given_empty_state_when_new_terminal_pressed_then_it_is_sent() {
        let themes = ThemeManager::new();
        let mut ui = simulator(card(&themes));

        ui.click("New terminal").expect("tile should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::NewTerminal]);
    }

    #[test]
    fn given_empty_state_when_new_launch_pressed_then_it_is_sent() {
        let themes = ThemeManager::new();
        let mut ui = simulator(card(&themes));

        ui.click("New launch…").expect("tile should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::NewLaunch]);
    }
}
