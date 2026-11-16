//! `SearchField`: small search input with a search icon and a clear cross.

use iced::keyboard::key::Named;
use iced::widget::{Id, Space, container, row, stack, svg, text_input};
use iced::{Alignment, Element, Length, Padding, Pixels};

use super::field_chrome::input_style;
use super::glyph;
use super::icon_button::{IconButton, IconButtonSize};
use super::input_keys::InputKeys;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Edge of the search icon (`width: 13px; height: 13px`).
const ICON_SIZE: f32 = 13.0;
/// Edge of the clear cross button: a tiny icon button.
const CROSS_SIZE: f32 = 14.0;
/// Gap between the icon, the text and the cross (`gap: 6px`).
const GAP: f32 = spacing::SPACE_2;
/// Side padding of the field (`padding: 0 8px`).
const PADDING_X: f32 = spacing::SPACE_3;
/// Vertical padding that makes a 13/20 text line one small control high.
const PADDING_Y: f32 =
    (size::CONTROL_MD - typography::BODY.line_height()) / 2.0;

/// Small search input, one small control high, that filters a list as the
/// user types, such as the projects of the left panel. The search icon
/// stands before the text. While there is text, a clear cross stands after
/// it; it and Esc in the focused input send the clear message. Enter in
/// the focused input sends the submit message. The field fills the width it
/// gets. Without an input handler it is disabled.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::search_field::SearchField;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Search(String),
///     ClearSearch,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let search: Element<'_, Message> = SearchField::new("lake", icon, &palette)
///     .placeholder("Name or folder")
///     .on_input(Message::Search)
///     .on_clear(Message::ClearSearch, "Clear the search")
///     .into();
/// ```
pub struct SearchField<'a, Message> {
    value: &'a str,
    icon: svg::Handle,
    palette: UiPalette,
    placeholder: String,
    id: Option<Id>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_clear: Option<(Message, String)>,
    on_submit: Option<Message>,
}

impl<'a, Message> SearchField<'a, Message> {
    /// Creates a field that shows `value` after the search `icon`.
    pub fn new(value: &'a str, icon: svg::Handle, palette: &UiPalette) -> Self {
        Self {
            value,
            icon,
            palette: palette.clone(),
            placeholder: String::new(),
            id: None,
            on_input: None,
            on_clear: None,
            on_submit: None,
        }
    }

    /// Sets the text shown while the value is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the widget id, so the app can focus the input.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the function that maps the new value to a message and enables
    /// the field.
    pub fn on_input(
        mut self,
        on_input: impl Fn(String) -> Message + 'a,
    ) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    /// Sets the message sent by the clear cross and by Esc in the focused
    /// input. `label` is the cross's tooltip, such as "Clear the search".
    pub fn on_clear(
        mut self,
        message: Message,
        label: impl Into<String>,
    ) -> Self {
        self.on_clear = Some((message, label.into()));
        self
    }

    /// Sets the message sent by Enter in the focused input, such as to open
    /// the first item the search lets through. The field takes the key, so
    /// it reaches nothing else.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<SearchField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: SearchField<'a, Message>) -> Self {
        let palette = value.palette;
        let body = typography::BODY;
        let has_value = !value.value.is_empty();

        // The input draws the box, its background and the focus ring. Its
        // side paddings keep the text clear of the icon and of the cross,
        // which are drawn over it. The cross always has its room, so the
        // text does not move when the cross appears.
        let mut input = text_input(&value.placeholder, value.value)
            .font(fonts::UI)
            .size(body.size())
            .line_height(Pixels(body.line_height()))
            .padding(Padding {
                top: PADDING_Y,
                right: PADDING_X + CROSS_SIZE + GAP,
                bottom: PADDING_Y,
                left: PADDING_X + ICON_SIZE + GAP,
            })
            .on_input_maybe(value.on_input)
            .style(input_style(&palette, false));
        if let Some(id) = value.id {
            input = input.id(id);
        }
        let mut keys = InputKeys::new(input).follow_text(value.value);
        if let Some((message, _)) = &value.on_clear {
            keys = keys.on_key(Named::Escape, message.clone());
        }
        if let Some(message) = value.on_submit {
            keys = keys.on_key(Named::Enter, message);
        }

        let muted = palette.text_muted();
        let icon = svg(value.icon)
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(move |_, _| svg::Style { color: Some(muted) });
        let mut marks = row![icon, Space::new().width(Length::Fill)]
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .padding(Padding::from([0.0, PADDING_X]));
        if let Some((message, label)) = value.on_clear.filter(|_| has_value) {
            // The field's own background is the hover background, so only
            // the cross turns brighter on hover.
            marks = marks.push(
                IconButton::new(glyph::cross(), label, &palette)
                    .size(IconButtonSize::Tiny)
                    .hover_background(false)
                    .on_press(message),
            );
        }

        container(stack![keys, marks])
            .width(Length::Fill)
            .height(size::CONTROL_MD)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::keyboard::key::Named;
    use iced::widget::container;
    use iced_test::simulator;

    use super::*;
    use crate::components::glyph;
    use crate::size;

    /// The id of the input of the fields under test.
    const FIELD: &str = "search";
    /// Width of the space the fields under test stand in.
    const WIDTH: f32 = 240.0;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Changed(String),
        Clear,
        Submit,
    }

    /// A field with `value`, a clear cross and an input handler, `WIDTH`
    /// wide.
    fn field(value: &str) -> Element<'_, Message> {
        let palette = UiPalette::otty_dark();

        container(
            SearchField::new(value, glyph::plus(), &palette)
                .placeholder("Name or folder")
                .id(Id::new(FIELD))
                .on_input(Message::Changed)
                .on_clear(Message::Clear, "Clear the search"),
        )
        .width(WIDTH)
        .into()
    }

    /// The middle of the clear cross at the right end of the field.
    fn cross_point() -> Point {
        Point::new(WIDTH - PADDING_X - CROSS_SIZE / 2.0, size::CONTROL_MD / 2.0)
    }

    #[test]
    fn given_a_field_when_laid_out_then_it_is_one_small_control_high() {
        let mut ui = simulator(field(""));

        let bounds = ui.find(Id::new(FIELD)).expect("the input").bounds();

        assert_eq!(bounds.height, size::CONTROL_MD);
        assert_eq!(bounds.width, WIDTH);
    }

    #[test]
    fn given_typing_when_a_letter_typed_then_the_new_value_is_sent() {
        let mut ui = simulator(field(""));

        ui.click(Id::new(FIELD)).expect("the field should show");
        let _ = ui.typewrite("l");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Changed("l".to_owned())]);
    }

    #[test]
    fn given_a_press_on_the_search_icon_then_the_input_takes_the_focus() {
        let mut ui = simulator(field(""));

        ui.point_at(Point::new(
            PADDING_X + ICON_SIZE / 2.0,
            size::CONTROL_MD / 2.0,
        ));
        let _ = ui.simulate(simulator::click());
        let _ = ui.typewrite("l");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Changed("l".to_owned())]);
    }

    #[test]
    fn given_a_value_when_the_cross_pressed_then_clear_is_sent() {
        let mut ui = simulator(field("lake"));

        ui.point_at(cross_point());
        let _ = ui.simulate(simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Clear]);
    }

    #[test]
    fn given_an_empty_value_when_pressed_where_the_cross_would_be_then_nothing_is_cleared()
     {
        let mut ui = simulator(field(""));

        ui.point_at(cross_point());
        let _ = ui.simulate(simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn given_the_focused_field_when_esc_pressed_then_clear_is_sent() {
        let mut ui = simulator(field("lake"));

        ui.click(Id::new(FIELD)).expect("the field should show");
        ui.tap_key(Named::Escape);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Clear]);
    }

    #[test]
    fn given_the_focused_field_when_enter_pressed_then_submit_is_sent_and_the_key_is_taken()
     {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> =
            SearchField::new("lake", glyph::plus(), &palette)
                .id(Id::new(FIELD))
                .on_input(Message::Changed)
                .on_submit(Message::Submit)
                .into();
        let mut ui = simulator(field);

        ui.click(Id::new(FIELD)).expect("the field should show");
        let status = ui.tap_key(Named::Enter);

        assert_eq!(status, iced::event::Status::Captured);
        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Submit]);
    }

    #[test]
    fn given_a_field_without_a_clear_message_when_esc_pressed_then_nothing_is_sent()
     {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> =
            SearchField::new("lake", glyph::plus(), &palette)
                .id(Id::new(FIELD))
                .on_input(Message::Changed)
                .into();
        let mut ui = simulator(field);

        ui.click(Id::new(FIELD)).expect("the field should show");
        ui.tap_key(Named::Escape);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(messages.is_empty(), "{messages:?}");
    }
}
