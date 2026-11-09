//! `Dialog`: a modal card centered over a dimmed window.

use iced::widget::{
    Space, column, container, mouse_area, opaque, row, scrollable, text,
};
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
/// full-window `scrim`. Clicks never reach the UI behind it, and a press on the
/// scrim does nothing unless the dialog asks for it with `on_backdrop_press`.
/// In a window too small for it, the card keeps its width and scrolls both
/// ways.
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
///     .on_backdrop_press(Message::Close)
///     .body(text("Theme"))
///     .into();
/// ```
pub struct Dialog<'a, Message> {
    title: String,
    palette: UiPalette,
    width: f32,
    close: Option<(Message, String)>,
    backdrop_press: Option<Message>,
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
            backdrop_press: None,
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

    /// Sends `message` when the scrim around the card is pressed with the
    /// left, right or middle button, as a press outside a menu closes it. A
    /// press on the card never sends it. Without this, a press on the scrim
    /// does nothing.
    ///
    /// # Example
    ///
    /// ```
    /// use iced::Element;
    /// use otty_ui_kit::components::dialog::Dialog;
    /// use otty_ui_kit::palette::UiPalette;
    ///
    /// #[derive(Clone)]
    /// enum Message {
    ///     Close,
    /// }
    ///
    /// let palette = UiPalette::otty_dark();
    /// let dialog: Element<'_, Message> = Dialog::new("Settings", &palette)
    ///     .on_backdrop_press(Message::Close)
    ///     .into();
    /// ```
    pub fn on_backdrop_press(mut self, message: Message) -> Self {
        self.backdrop_press = Some(message);
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

        // `opaque` keeps a press on the card from reaching the scrim.
        let card = opaque(card);

        let scrim = palette.scrim();
        let card = scrollable(container(card).padding(SHADOW_ROOM))
            .direction(scrollbar::both())
            .style(scrollbar::style(&palette));
        let backdrop = container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(move |_| container::Style {
                background: Some(scrim.into()),
                ..Default::default()
            });

        let backdrop: Element<'a, Message> = match value.backdrop_press {
            Some(message) => mouse_area(backdrop)
                .on_press(message.clone())
                .on_right_press(message.clone())
                .on_middle_press(message)
                .into(),
            None => backdrop.into(),
        };

        // `opaque` keeps clicks from reaching the UI behind the dialog.
        opaque(backdrop)
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
        Backdrop,
        Picked(&'static str),
    }

    const BUTTONS: [mouse::Button; 3] = [
        mouse::Button::Left,
        mouse::Button::Right,
        mouse::Button::Middle,
    ];

    /// Options of the list in [`dialog_with_a_list`]: enough of them for the
    /// open list to reach past the card, over the scrim.
    const OPTIONS: [&str; 6] = ["one", "two", "three", "four", "five", "six"];

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

    /// The dialog of [`dialog`] that also sends `Backdrop` on a scrim press.
    fn closing_dialog() -> Simulator<'static, Message> {
        let palette = UiPalette::otty_dark();
        let dialog = Dialog::new("Delete session", &palette)
            .on_backdrop_press(Message::Backdrop)
            .body(text("body").width(Length::Fill))
            .action(button(text("Delete")).on_press(Message::Deleted));

        Simulator::with_size(
            iced::Settings::default(),
            (1024.0, 768.0),
            Element::from(dialog),
        )
    }

    /// A closing dialog that holds only a list at its bottom.
    fn dialog_with_a_list() -> Simulator<'static, Message> {
        let palette = UiPalette::otty_dark();
        let list = crate::components::select::select(
            &OPTIONS,
            Some("one"),
            Message::Picked,
            &palette,
        );
        let dialog = Dialog::new("Pick", &palette)
            .on_backdrop_press(Message::Backdrop)
            .body(list);

        Simulator::with_size(
            iced::Settings::default(),
            (1024.0, 768.0),
            Element::from(dialog),
        )
    }

    /// Presses and releases `button` at `point`.
    fn press_at(
        ui: &mut Simulator<'_, Message>,
        point: Point,
        button: mouse::Button,
    ) {
        ui.point_at(point);
        let _ = ui.simulate([
            Event::Mouse(mouse::Event::ButtonPressed(button)),
            Event::Mouse(mouse::Event::ButtonReleased(button)),
        ]);
    }

    /// The bounds of the card of [`closing_dialog`]: around the title, the
    /// body and the action, with the padding.
    fn card_bounds(ui: &mut Simulator<'_, Message>) -> iced::Rectangle {
        let title = ui.find("Delete session").expect("title").bounds();
        let body = ui.find("body").expect("body").bounds();
        let action = ui.find("Delete").expect("action").bounds();

        let top = title.y - spacing::SPACE_5;
        let bottom = action.y + action.height + spacing::SPACE_5;
        iced::Rectangle {
            x: body.x - spacing::SPACE_5,
            y: top,
            width: size::DIALOG,
            height: bottom - top,
        }
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

    /// The scrim is everything around the card, the room left for its
    /// shadow too, and any button closes the dialog there, as a press
    /// outside a menu closes it.
    #[test]
    fn given_on_backdrop_press_when_any_button_is_pressed_on_the_scrim_then_its_message_is_sent()
     {
        for button in BUTTONS {
            let mut ui = closing_dialog();
            let card = card_bounds(&mut ui);
            let corner = Point::new(10.0, 10.0);
            let under_the_card =
                Point::new(card.center_x(), card.y + card.height + 10.0);

            press_at(&mut ui, corner, button);
            press_at(&mut ui, under_the_card, button);

            let pressed: Vec<Message> = ui.into_messages().collect();
            assert_eq!(
                pressed,
                vec![Message::Backdrop, Message::Backdrop],
                "{button:?}"
            );
        }
    }

    #[test]
    fn given_on_backdrop_press_when_the_card_is_pressed_then_nothing_is_sent() {
        for button in BUTTONS {
            let mut ui = closing_dialog();
            let card = card_bounds(&mut ui);
            let body = ui.find("body").expect("body").bounds();
            let title = ui.find("Delete session").expect("title").bounds();
            let padding = Point::new(card.x + 4.0, card.center_y());

            press_at(&mut ui, body.center(), button);
            press_at(&mut ui, title.center(), button);
            press_at(&mut ui, padding, button);

            let pressed: Vec<Message> = ui.into_messages().collect();
            assert!(pressed.is_empty(), "{button:?}: {pressed:?}");
        }
    }

    /// A drag that starts on the card is no press on the scrim, wherever it
    /// ends.
    #[test]
    fn given_on_backdrop_press_when_a_press_on_the_card_ends_on_the_scrim_then_nothing_is_sent()
     {
        let mut ui = closing_dialog();
        let body = ui.find("body").expect("body").bounds();

        ui.point_at(body.center());
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))]);
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ))]);

        let pressed: Vec<Message> = ui.into_messages().collect();
        assert!(pressed.is_empty(), "{pressed:?}");
    }

    #[test]
    fn given_on_backdrop_press_then_the_card_buttons_still_send_their_messages()
    {
        let mut ui = closing_dialog();

        ui.click("Delete").expect("the action should show");

        let pressed: Vec<Message> = ui.into_messages().collect();
        assert_eq!(pressed, vec![Message::Deleted]);
    }

    /// An open list that reaches over the scrim takes its own press: an
    /// option is picked and the dialog stays. The list draws its text itself,
    /// so the test finds the field by place, right under the title. Inside
    /// the scrolling card there is no room under the field, so the list opens
    /// upwards, over the title and the scrim above the card.
    #[test]
    fn given_on_backdrop_press_when_an_option_over_the_scrim_is_picked_then_only_the_option_is_sent()
     {
        let mut ui = dialog_with_a_list();
        let title = ui.find("Pick").expect("the title should show").bounds();
        let card_top = title.y - spacing::SPACE_5;
        let field = Point::new(
            title.x + 20.0,
            title.y + title.height + spacing::SPACE_5 + size::CONTROL_LG / 2.0,
        );
        let over_the_scrim = Point::new(field.x, card_top - 20.0);

        ui.point_at(field);
        let _ = ui.simulate(iced_test::simulator::click());
        ui.point_at(over_the_scrim);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: over_the_scrim,
        })]);
        let _ = ui.simulate(iced_test::simulator::click());

        let pressed: Vec<Message> = ui.into_messages().collect();
        assert!(
            matches!(pressed.as_slice(), [Message::Picked(option)] if *option != "one"),
            "{pressed:?}"
        );
    }
}
