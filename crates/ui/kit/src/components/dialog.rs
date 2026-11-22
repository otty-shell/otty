//! `Dialog`: a modal card centered over a dimmed window.

use iced::widget::{
    Space, container, mouse_area, opaque, row, scrollable, text,
};
use iced::{Alignment, Element, Length, alignment};

use super::dialog_column::DialogColumn;
use super::field_focus::FieldFocus;
use super::icon_button::IconButton;
use super::{glyph, scrollbar};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Room between the card and each edge of the window, so that the window
/// does not cut the card's shadow (`0 14px 34px`: the blur reaches 34 + 14 px
/// below it).
const WINDOW_MARGIN: f32 = 48.0;

/// Modal dialog: a title, an optional body and a row of actions, `space-5`
/// apart, on a `surface_overlay` card `size::DIALOG` wide by default. The
/// title can end with a close cross. The card sits in the middle of a
/// full-window `scrim`. Clicks never reach the UI behind it, and a press on the
/// scrim does nothing unless the dialog asks for it with `on_backdrop_press`.
/// The card is never taller than the window less a margin: a body that does
/// not fit scrolls inside the card, with its bar along the card's right
/// side, while the title and the actions stay. A body that fits shows no
/// bar. In a window narrower than the card, the card shrinks to it. Tab and
/// Shift+Tab walk the dialog's fields, round from the last to the first,
/// and never reach a field behind it.
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

        let mut content = DialogColumn::new(title, spacing::SPACE_5);
        if let Some(body) = value.body {
            content = content.body(scrolling_body(body, &palette));
        }
        if !value.actions.is_empty() {
            let actions = row![Space::new().width(Length::Fill)]
                .extend(value.actions)
                .spacing(spacing::SPACE_3)
                .width(Length::Fill);
            content = content.actions(actions.into());
        }

        let card_background = palette.surface_overlay();
        let card_shadow = palette.shadow_popover();
        let card = container(content)
            .width(Length::Fill)
            .max_width(value.width)
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
        let backdrop = container(card)
            .padding(WINDOW_MARGIN)
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

        // `opaque` keeps clicks from reaching the UI behind the dialog, and
        // Tab walks only the dialog's own fields.
        FieldFocus::modal(opaque(backdrop)).into()
    }
}

/// The body in an area that scrolls up and down when the card has no room
/// for all of it. The bar then sits beside the body, `space-2` from it, and
/// takes its room from the body; while the body fits there is no bar.
fn scrolling_body<'a, Message: 'a>(
    body: Element<'a, Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    scrollable(body)
        .direction(scrollbar::vertical())
        .spacing(spacing::SPACE_2)
        .width(Length::Fill)
        .style(scrollbar::style(palette))
        .into()
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
    use iced::widget::{Column, button};
    use iced::{Event, Point, Rectangle, mouse};
    use iced_test::Simulator;
    use iced_test::selector::Candidate;

    use super::*;
    use crate::testing::events::named_input;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Deleted,
        Closed,
        Backdrop,
        Picked(&'static str),
        TypedIn(&'static str),
    }

    const BUTTONS: [mouse::Button; 3] = [
        mouse::Button::Left,
        mouse::Button::Right,
        mouse::Button::Middle,
    ];

    /// Options of the list in [`dialog_with_a_list`]: enough of them for the
    /// open list to reach past the card, over the scrim.
    const OPTIONS: [&str; 6] = ["one", "two", "three", "four", "five", "six"];

    /// How many lines the body of [`long_dialog`] has.
    const LONG_BODY_LINES: usize = 60;

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

    /// A dialog whose body, `line 0` to the last of [`LONG_BODY_LINES`], is
    /// taller than the window.
    fn long_dialog(width: f32, height: f32) -> Simulator<'static, Message> {
        let palette = UiPalette::otty_dark();
        let lines = (0..LONG_BODY_LINES)
            .map(|line| Element::from(text(format!("line {line}"))));
        let dialog = Dialog::new("Delete session", &palette)
            .body(Column::with_children(lines).width(Length::Fill))
            .action(button(text("Delete")).on_press(Message::Deleted));

        Simulator::with_size(
            iced::Settings::default(),
            (width, height),
            Element::from(dialog),
        )
    }

    /// The bounds of the scrolling body and of its content.
    fn body_scroll(ui: &mut Simulator<'_, Message>) -> (Rectangle, Rectangle) {
        ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                bounds,
                content_bounds,
                ..
            } => Some((bounds, content_bounds)),
            _ => None,
        })
        .expect("the body should scroll")
    }

    /// The card around a `title` and an `action` at the end of its row:
    /// their bounds with the padding of the card.
    fn card_around(title: Rectangle, action: Rectangle) -> Rectangle {
        let left = title.x - spacing::SPACE_5;
        let top = title.y - spacing::SPACE_5;

        Rectangle {
            x: left,
            y: top,
            width: action.x + action.width + spacing::SPACE_5 - left,
            height: action.y + action.height + spacing::SPACE_5 - top,
        }
    }

    /// Return whether `inner` lies inside `outer`.
    fn is_inside(inner: Rectangle, outer: Rectangle) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.x + inner.width <= outer.x + outer.width
            && inner.y + inner.height <= outer.y + outer.height
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

    /// The card shrinks to a window narrower than it, less the margin, so
    /// its body and its bar stay on the card and its action in reach.
    #[test]
    fn given_a_window_narrower_than_the_card_then_the_card_shrinks_to_it_and_its_action_can_be_pressed()
     {
        let width = 300.0;
        let mut ui = in_window(width, 400.0);

        let body = ui.find("body").expect("body should exist").bounds();
        ui.click("Delete").expect("the action should be in reach");

        let pressed: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            body.width,
            width - 2.0 * WINDOW_MARGIN - 2.0 * spacing::SPACE_5
        );
        assert_eq!(body.x, WINDOW_MARGIN + spacing::SPACE_5);
        assert_eq!(pressed, vec![Message::Deleted]);
    }

    /// Issue 33: the title and the action stay in the window, the card is
    /// no taller than the window less the margin, and only the body
    /// scrolls, with its bar inside the card.
    #[test]
    fn given_a_body_taller_than_the_window_then_the_title_and_the_action_stay_and_the_body_scrolls_inside_the_card()
     {
        let height = 400.0;
        let mut ui = long_dialog(1024.0, height);

        let title = ui.find("Delete session").expect("title").bounds();
        let action = ui.find("Delete").expect("action").bounds();
        let card = card_around(title, action);
        let (body, content) = body_scroll(&mut ui);
        ui.point_at(body.center());
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels {
                x: 0.0,
                y: -10_000.0,
            },
        })]);
        let title_after = ui.find("Delete session").expect("title").bounds();
        let action_after = ui.find("Delete").expect("action").bounds();
        let first = ui.find("line 0").expect("first line").visible_bounds();
        let last_line = format!("line {}", LONG_BODY_LINES - 1);
        let last = ui
            .find(last_line.as_str())
            .expect("last line")
            .visible_bounds();

        assert!(card.y >= WINDOW_MARGIN - 0.5, "{card:?}");
        assert!(card.y + card.height <= height - WINDOW_MARGIN + 0.5);
        assert!(is_inside(body, card), "{body:?} should be in {card:?}");
        assert!(content.height > body.height, "the body should overflow");
        assert!(
            content.width < body.width,
            "the bar should take its room beside the body"
        );
        assert_eq!(title_after, title, "the title should stay");
        assert_eq!(action_after, action, "the action should stay");
        assert_eq!(first, None, "the first line should scroll away");
        assert!(last.is_some(), "the last line should come into view");
    }

    /// Issue 33: a body that fits shows no bar, so it keeps all its width.
    #[test]
    fn given_a_body_that_fits_then_it_shows_no_bar() {
        let mut ui = in_window(1024.0, 768.0);

        let (body, content) = body_scroll(&mut ui);

        assert!(content.height <= body.height, "{content:?} in {body:?}");
        assert_eq!(content.width, body.width);
    }

    /// Nothing around the card scrolls: the body is the only area that
    /// does.
    #[test]
    fn given_a_small_window_then_only_the_body_scrolls() {
        let mut ui = long_dialog(300.0, 200.0);

        let mut count = 0;
        let _ = ui.find(|candidate: Candidate<'_>| {
            if matches!(candidate, Candidate::Scrollable { .. }) {
                count += 1;
            }
            None::<()>
        });

        assert_eq!(count, 1);
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

    /// A dialog with the fields `first`, `second` and `third`, over a
    /// window whose field `behind` lies under the scrim.
    fn form_over_a_field() -> Simulator<'static, Message> {
        let palette = UiPalette::otty_dark();
        let dialog = Dialog::new("Profile", &palette)
            .body(iced::widget::column![
                named_input("first", Message::TypedIn),
                named_input("second", Message::TypedIn),
                named_input("third", Message::TypedIn),
            ])
            .action(button(text("Delete")).on_press(Message::Deleted));
        let window = iced::widget::stack![
            iced::widget::column![named_input("behind", Message::TypedIn)]
                .height(Length::Fill),
            dialog,
        ];

        Simulator::with_size(iced::Settings::default(), (1024.0, 768.0), window)
    }

    /// Issue 43: Tab moves the keyboard to the next field of the dialog,
    /// and from the last one back to the first.
    #[test]
    fn tab_moves_to_the_next_field_and_wraps_inside_the_dialog() {
        let mut ui = form_over_a_field();
        ui.click(iced::widget::Id::new("first"))
            .expect("the first field should show");

        for _ in 0..4 {
            let _ = ui.simulate(crate::testing::events::tab_tap(
                iced::keyboard::Modifiers::empty(),
            ));
            let _ = ui.typewrite("x");
        }

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            ["second", "third", "first", "second"].map(Message::TypedIn)
        );
    }

    /// Issue 43: Shift+Tab moves back, and from the first field to the
    /// last one.
    #[test]
    fn shift_tab_moves_to_the_field_before_and_wraps_inside_the_dialog() {
        let mut ui = form_over_a_field();
        ui.click(iced::widget::Id::new("second"))
            .expect("the second field should show");

        for _ in 0..3 {
            let _ = ui.simulate(crate::testing::events::tab_tap(
                iced::keyboard::Modifiers::SHIFT,
            ));
            let _ = ui.typewrite("x");
        }

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            ["first", "third", "second"].map(Message::TypedIn)
        );
    }

    /// Issue 43: with no field of the dialog holding the keyboard, Tab
    /// gives it to the first field, and nothing behind the dialog ever
    /// gets it; the press stays in the dialog.
    #[test]
    fn tab_never_reaches_a_field_behind_the_dialog() {
        let mut ui = form_over_a_field();

        let mut statuses = Vec::new();
        for _ in 0..4 {
            statuses.extend(ui.simulate(crate::testing::events::tab_tap(
                iced::keyboard::Modifiers::empty(),
            )));
            let _ = ui.typewrite("x");
        }

        assert!(
            statuses
                .iter()
                .step_by(2)
                .all(|status| *status == iced::event::Status::Captured),
            "{statuses:?}"
        );
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            ["first", "second", "third", "first"].map(Message::TypedIn)
        );
    }

    /// A dialog with no field keeps Tab too, so it reaches nothing behind.
    #[test]
    fn tab_in_a_dialog_with_no_field_stays_in_the_dialog() {
        let mut ui = in_window(1024.0, 768.0);

        let statuses = ui.simulate(crate::testing::events::tab_tap(
            iced::keyboard::Modifiers::empty(),
        ));

        assert_eq!(statuses[0], iced::event::Status::Captured);
        assert_eq!(ui.into_messages().count(), 0);
    }
}
