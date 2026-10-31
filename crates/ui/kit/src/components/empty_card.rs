//! `EmptyCard`: centered start card shown when no session is open.

use iced::widget::{column, container, text};
use iced::{Element, Length};

use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Column with the logo, a title, a muted description and a stack of
/// actions such as [`ActionTile`]s. It is 460 px wide; center it in the
/// pane with a container.
///
/// The caller passes the logo as an element and the card never recolors it.
/// The card gives it a slot of `EMPTY_LOGO_WIDTH` by `EMPTY_LOGO_HEIGHT`,
/// so give the logo `Length::Fill` on both sides.
///
/// [`ActionTile`]: super::action_tile::ActionTile
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::action_tile::ActionTile;
/// use otty_ui_kit::components::empty_card::EmptyCard;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let icon = iced::widget::svg::Handle::from_memory(&b"<svg/>"[..]);
/// let card: Element<'_, ()> = EmptyCard::new(
///     "Start a session",
///     "Open a terminal on this computer.",
///     text("logo"),
///     &palette,
/// )
/// .action(ActionTile::new(icon, "New terminal", &palette))
/// .into();
/// ```
pub struct EmptyCard<'a, Message> {
    title: String,
    description: String,
    logo: Element<'a, Message>,
    palette: UiPalette,
    actions: Vec<Element<'a, Message>>,
}

impl<'a, Message> EmptyCard<'a, Message> {
    /// Creates a card with a `title`, a `description` and the `logo`.
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        logo: impl Into<Element<'a, Message>>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            logo: logo.into(),
            palette: palette.clone(),
            actions: Vec::new(),
        }
    }

    /// Adds an action under the description.
    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(action.into());
        self
    }
}

impl<'a, Message: 'a> From<EmptyCard<'a, Message>> for Element<'a, Message> {
    fn from(value: EmptyCard<'a, Message>) -> Self {
        let palette = value.palette;
        let title_style = typography::TITLE;
        let body = typography::BODY;

        let logo = container(value.logo)
            .width(size::EMPTY_LOGO_WIDTH)
            .height(size::EMPTY_LOGO_HEIGHT);
        let heading = column![
            text(value.title)
                .size(title_style.size())
                .line_height(iced::Pixels(title_style.line_height()))
                .font(fonts::UI_SEMIBOLD)
                .color(palette.text_strong()),
            text(value.description)
                .size(body.size())
                .line_height(iced::Pixels(body.line_height()))
                .font(fonts::UI)
                .color(palette.text_muted()),
        ]
        .spacing(spacing::SPACE_1);
        let actions = column(value.actions).spacing(spacing::SPACE_2);

        container(column![logo, heading, actions].spacing(spacing::SPACE_5))
            .width(Length::Fixed(size::EMPTY_CARD))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::text;
    use iced_test::simulator;

    use super::*;
    use crate::components::action_tile::ActionTile;
    use crate::components::glyph;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        NewTerminal,
        NewLaunch,
    }

    fn card() -> EmptyCard<'static, Message> {
        let palette = UiPalette::otty_dark();

        EmptyCard::new(
            "Start a session",
            "Open a terminal on this computer.",
            text("logo"),
            &palette,
        )
        .action(
            ActionTile::new(glyph::plus(), "New terminal", &palette)
                .on_press(Message::NewTerminal),
        )
        .action(
            ActionTile::new(glyph::plus(), "New launch…", &palette)
                .on_press(Message::NewLaunch),
        )
    }

    #[test]
    fn given_empty_card_then_title_and_description_are_shown() {
        let mut ui = simulator(card());

        assert!(ui.find("Start a session").is_ok());
        assert!(ui.find("Open a terminal on this computer.").is_ok());
        assert!(ui.find("logo").is_ok());
    }

    #[test]
    fn given_empty_card_when_second_tile_pressed_then_only_its_message_is_sent()
    {
        let mut ui = simulator(card());

        ui.click("New launch…").expect("tile should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::NewLaunch]);
    }
}
