//! `Dialog`: a modal card centered over a dimmed window.

use iced::widget::{Space, column, container, opaque, row, scrollable, text};
use iced::{Alignment, Element, Length, alignment};

use super::icon_button::IconButton;
use super::{glyph, scrollbar};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Room around the card inside the area that scrolls, so that the scrolling
/// does not cut the card's shadow (`0 14px 34px`: the blur reaches 34 + 14 px
/// below it).
const SHADOW_ROOM: f32 = 48.0;

/// Modal dialog: a title, an optional body and a row of actions, `space-5`
/// apart, on a `surface_overlay` card `size::DIALOG` wide by default. The
/// title can end with a close cross. The card sits in the middle of a
/// full-window `scrim`. Clicks never reach the UI behind it. In a window too
/// small for it, the card keeps its width and scrolls both ways.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::dialog::Dialog;
/// use otty_ui_kit::palette::UiPalette;
/// use otty_ui_kit::size;
///
/// #[derive(Clone)]
/// enum Message {
///     Close,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let dialog: Element<'_, Message> = Dialog::new("Delete session", &palette)
///     .body(text("This cannot be undone."))
///     .action(text("Cancel"))
///     .action(text("Delete"))
///     .into();
/// let settings: Element<'_, Message> = Dialog::new("Settings", &palette)
///     .width(size::DIALOG_WIDE)
///     .on_close(Message::Close, "Close")
///     .body(text("Theme"))
///     .into();
/// ```
pub struct Dialog<'a, Message> {
    title: String,
    palette: UiPalette,
    width: f32,
    close: Option<(Message, String)>,
    body: Option<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
}

impl<'a, Message> Dialog<'a, Message> {
    /// Creates a dialog `size::DIALOG` wide with a title and no close cross,
    /// body or actions.
    pub fn new(title: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            title: title.into(),
            palette: palette.clone(),
            width: size::DIALOG,
            close: None,
            body: None,
            actions: Vec::new(),
        }
    }

    /// Sets the width of the card, padding included. A dialog that holds a
    /// whole form uses `size::DIALOG_WIDE`.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Ends the title with a close cross that sends `message`, with `label`
    /// as its tooltip. Use it for a dialog that needs no Cancel action.
    pub fn on_close(
        mut self,
        message: Message,
        label: impl Into<String>,
    ) -> Self {
        self.close = Some((message, label.into()));
        self
    }

    /// Sets the body, shown under the title.
    pub fn body(mut self, body: impl Into<Element<'a, Message>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Appends an action. Actions sit right-aligned in one row, in the order
    /// added.
    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(action.into());
        self
    }
}

impl<'a, Message: Clone + 'a> From<Dialog<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: Dialog<'a, Message>) -> Self {
        let palette = value.palette;

        let title = title_row(value.title, value.close, &palette);

        let mut content =
            column![title].spacing(spacing::SPACE_5).width(Length::Fill);
        if let Some(body) = value.body {
            content = content.push(body);
        }
        if !value.actions.is_empty() {
            let actions = row![Space::new().width(Length::Fill)]
                .extend(value.actions)
                .spacing(spacing::SPACE_3)
                .width(Length::Fill);
            content = content.push(actions);
        }

        let card_background = palette.surface_overlay();
        let card_shadow = palette.shadow_popover();
        let card = container(content)
            .width(value.width)
            .padding(spacing::SPACE_5)
            .style(move |_| container::Style {
                background: Some(card_background.into()),
                border: iced::Border {
                    radius: radius::RADIUS_LG.into(),
                    ..Default::default()
                },
                shadow: card_shadow,
                ..Default::default()
            });

        let scrim = palette.scrim();
        let card = scrollable(container(card).padding(SHADOW_ROOM))
            .direction(scrollbar::both())
            .style(scrollbar::style(&palette));

        // `opaque` keeps clicks from reaching the UI behind the dialog.
        opaque(
            container(card)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(move |_| container::Style {
                    background: Some(scrim.into()),
                    ..Default::default()
                }),
        )
    }
}

/// The title, and the close cross at the right end of its row when there is
/// one.
fn title_row<'a, Message: Clone + 'a>(
    title: String,
    close: Option<(Message, String)>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let title = text(title)
        .size(typography::HEADING.size())
        .font(fonts::UI)
        .color(palette.text_strong());

    let Some((message, label)) = close else {
        return title.into();
    };

    let cross =
        IconButton::new(glyph::cross(), label, palette).on_press(message);

    row![title.width(Length::Fill), cross]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::button;
    use iced::{Event, Point, mouse};
    use iced_test::Simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Deleted,
        Closed,
    }

    fn dialog() -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();

        Dialog::new("Delete session", &palette)
            .body(text("body").width(Length::Fill))
            .action(button(text("Delete")).on_press(Message::Deleted))
            .into()
    }

    /// A Settings-like dialog: wide, with a close cross and no actions.
    fn wide_dialog_with_a_cross() -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();

        Dialog::new("Settings", &palette)
            .width(size::DIALOG_WIDE)
            .on_close(Message::Closed, "Close")
            .body(text("body").width(Length::Fill))
            .into()
    }

    #[test]
    fn given_a_width_when_laid_out_then_the_card_is_that_wide_and_in_the_middle()
     {
        let width = 1024.0;
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (width, 768.0),
            wide_dialog_with_a_cross(),
        );

        let body = ui.find("body").expect("body should exist").bounds();

        assert_eq!(body.width, size::DIALOG_WIDE - 2.0 * spacing::SPACE_5);
        assert_eq!(
            body.x,
            (width - size::DIALOG_WIDE) / 2.0 + spacing::SPACE_5
        );
    }

    #[test]
    fn given_on_close_when_the_cross_is_pressed_then_its_message_is_sent() {
        let width = 1024.0;
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (width, 768.0),
            wide_dialog_with_a_cross(),
        );
        let title = ui.find("Settings").expect("title should exist").bounds();
        let body = ui.find("body").expect("body should exist").bounds();

        // The cross ends the title row, at the right edge of the content.
        let content_right =
            (width + size::DIALOG_WIDE) / 2.0 - spacing::SPACE_5;
        let cross = Point::new(
            content_right - size::CONTROL_MD / 2.0,
            title.center_y(),
        );
        ui.point_at(cross);
        let _ = ui.simulate(iced_test::simulator::click());

        let pressed: Vec<Message> = ui.into_messages().collect();
        assert_eq!(pressed, vec![Message::Closed]);
        assert_eq!(title.x, body.x, "the title should keep its place");
    }

    #[test]
    fn given_on_close_then_a_press_on_the_title_sends_nothing() {
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (1024.0, 768.0),
            wide_dialog_with_a_cross(),
        );

        ui.click("Settings").expect("title should exist");

        let pressed: Vec<Message> = ui.into_messages().collect();
        assert!(pressed.is_empty(), "{pressed:?}");
    }

    fn in_window(width: f32, height: f32) -> Simulator<'static, Message> {
        Simulator::with_size(
            iced::Settings::default(),
            (width, height),
            dialog(),
        )
    }

    #[test]
    fn given_a_window_with_room_when_laid_out_then_the_card_is_in_the_middle() {
        let (width, height) = (1024.0, 768.0);
        let mut ui = in_window(width, height);

        let body = ui.find("body").expect("body should exist").bounds();
        let title = ui.find("Delete session").expect("title should exist");
        let action = ui.find("Delete").expect("action should exist").bounds();

        let top = title.bounds().y - spacing::SPACE_5;
        let bottom = action.y + action.height + spacing::SPACE_5;
        assert_eq!(body.width, size::DIALOG - 2.0 * spacing::SPACE_5);
        assert_eq!(
            body.x,
            (width - size::DIALOG) / 2.0 + spacing::SPACE_5,
            "the card should be centered sideways"
        );
        assert!(
            ((top + bottom) / 2.0 - height / 2.0).abs() < 8.0,
            "the card should be centered: {top} to {bottom}"
        );
    }

    #[test]
    fn given_a_window_smaller_than_the_dialog_when_scrolled_then_it_keeps_its_width_and_its_action_can_be_pressed()
     {
        let mut ui = in_window(260.0, 80.0);

        let body = ui.find("body").expect("body should exist").bounds();
        ui.point_at(Point::new(100.0, 40.0));
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels {
                x: -10_000.0,
                y: -10_000.0,
            },
        })]);
        ui.click("Delete").expect("the action should be in reach");

        let pressed: Vec<Message> = ui.into_messages().collect();
        assert_eq!(body.width, size::DIALOG - 2.0 * spacing::SPACE_5);
        assert_eq!(pressed, vec![Message::Deleted]);
    }

    /// The scrim takes a press of any button, so none reaches the UI behind
    /// the dialog, and the dialog stays open: it sends nothing.
    #[test]
    fn given_a_dialog_when_any_button_is_pressed_on_the_scrim_then_the_press_stops_there()
     {
        for button in [
            mouse::Button::Left,
            mouse::Button::Right,
            mouse::Button::Middle,
        ] {
            let mut ui = in_window(1024.0, 768.0);

            ui.point_at(Point::new(10.0, 10.0));
            let statuses = ui.simulate([
                Event::Mouse(mouse::Event::ButtonPressed(button)),
                Event::Mouse(mouse::Event::ButtonReleased(button)),
            ]);

            let pressed: Vec<Message> = ui.into_messages().collect();
            assert_eq!(
                statuses[0],
                iced::event::Status::Captured,
                "{button:?}"
            );
            assert!(pressed.is_empty(), "{button:?}");
        }
    }
}
