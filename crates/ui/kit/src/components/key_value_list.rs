//! `KeyValueList`: two-column list of facts about one item.

use iced::widget::text::Wrapping;
use iced::widget::{column, responsive, row, text};
use iced::{Element, Length};

use super::wrapped_text::WrappedText;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing};

/// Text size of keys and values (`font-size: 12.5px`).
pub(super) const TEXT_SIZE: f32 = 12.5;

/// Two columns: muted keys at a fixed width, mono values that wrap. Below
/// [`size::KEY_VALUE_STACK_BELOW`] wide, each value sits under its key
/// instead. A value can hold links: accent text, underlined under the
/// pointer, each sending its own message when pressed.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::key_value_list::{KeyValueList, ValuePart};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     OpenImage,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let list: Element<'_, Message> = KeyValueList::new(&palette)
///     .entry_parts(
///         "Image",
///         [ValuePart::link("nginx:1.27", Message::OpenImage)],
///     )
///     .entry("Ports", "8080:80/tcp")
///     .into();
/// ```
pub struct KeyValueList<Message> {
    palette: UiPalette,
    entries: Vec<(String, Vec<ValuePart<Message>>)>,
}

impl<Message> KeyValueList<Message> {
    /// Creates an empty list.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            entries: Vec::new(),
        }
    }

    /// Adds a `key` and its plain `value` below the previous entry.
    pub fn entry(
        self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.entry_parts(key, [ValuePart::text(value)])
    }

    /// Adds a `key` whose value is `parts` in order, below the previous
    /// entry: plain text and links. The value wraps like a plain one, and
    /// each link is pressed on its own.
    pub fn entry_parts(
        mut self,
        key: impl Into<String>,
        parts: impl IntoIterator<Item = ValuePart<Message>>,
    ) -> Self {
        self.entries.push((key.into(), parts.into_iter().collect()));
        self
    }
}

impl<'a, Message: Clone + 'a> From<KeyValueList<Message>>
    for Element<'a, Message>
{
    fn from(value: KeyValueList<Message>) -> Self {
        let palette = value.palette;
        let entries = value.entries;

        // `responsive` fills the height by default; shrinking keeps the list
        // as tall as its entries, also inside a scrollable panel.
        responsive(move |available| {
            let stacked = available.width < size::KEY_VALUE_STACK_BELOW;
            let mut lines = column![].spacing(spacing::SPACE_3);

            for (key, parts) in &entries {
                lines = lines.push(entry_view(key, parts, stacked, &palette));
            }

            lines.into()
        })
        .height(Length::Shrink)
        .into()
    }
}

/// One piece of a [`KeyValueList`] value: plain text, or a link that sends
/// a message when pressed.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::key_value_list::ValuePart;
///
/// let ports: Vec<ValuePart<u16>> = vec![
///     ValuePart::link("8080:80/tcp", 8080),
///     ValuePart::text(", "),
///     ValuePart::text("53/udp"),
/// ];
/// assert_eq!(ports.len(), 3);
/// ```
pub struct ValuePart<Message> {
    text: String,
    on_press: Option<Message>,
}

impl<Message> ValuePart<Message> {
    /// Plain text.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            on_press: None,
        }
    }

    /// A link showing `text` that sends `on_press` when pressed.
    pub fn link(text: impl Into<String>, on_press: Message) -> Self {
        Self {
            text: text.into(),
            on_press: Some(on_press),
        }
    }
}

/// Draws one entry: the key left of its value, or above it when `stacked`.
pub(super) fn entry_view<'a, Message: Clone + 'a>(
    key: &str,
    parts: &[ValuePart<Message>],
    stacked: bool,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let key = text(key.to_owned())
        .size(TEXT_SIZE)
        .font(fonts::UI)
        .wrapping(Wrapping::Glyph)
        .color(palette.text_muted());

    // Values break at spaces, slashes, dashes, colons and dots first, and
    // between characters only when a piece is wider than the line, so a long
    // id or path never pushes the list wider than its panel.
    let value = WrappedText::new(
        parts
            .iter()
            .map(|part| (part.text.clone(), part.on_press.clone()))
            .collect(),
        TEXT_SIZE,
        fonts::MONO,
        palette.text_body(),
        link_color(palette),
    );

    if stacked {
        column![key, value].spacing(spacing::SPACE_1).into()
    } else {
        row![key.width(size::KEY_VALUE_KEY), value]
            .spacing(spacing::SPACE_3)
            .into()
    }
}

/// Color of a link in a value, on every panel: `accent`.
fn link_color(palette: &UiPalette) -> iced::Color {
    palette.accent()
}

#[cfg(test)]
mod tests {
    use iced::{Event, Point, mouse};
    use iced_test::simulator;

    use super::*;
    use crate::contrast::contrast_ratio;

    /// WCAG 2.1 contrast for normal text.
    const TEXT_MIN: f32 = 4.5;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Port(u16),
        Volume,
    }

    #[test]
    fn given_entries_when_rendered_then_keys_and_values_are_shown() {
        let palette = UiPalette::otty_dark();
        let list: Element<'_, ()> = KeyValueList::new(&palette)
            .entry("Image", "nginx:1.27")
            .entry("Ports", "8080:80/tcp")
            .into();
        let mut ui = simulator(list);

        for label in ["Image", "nginx:1.27", "Ports", "8080:80/tcp"] {
            assert!(ui.find(label).is_ok(), "{label} should be shown");
        }
    }

    #[test]
    fn given_entry_when_laid_out_then_value_starts_after_key_column() {
        let palette = UiPalette::otty_light();
        let list: Element<'_, ()> =
            KeyValueList::new(&palette).entry("Image", "nginx").into();
        let mut ui = simulator(list);

        let key = ui.find("Image").expect("key should be shown").bounds();
        let value = ui.find("nginx").expect("value should be shown").bounds();

        assert!(
            (value.x - key.x - size::KEY_VALUE_KEY - spacing::SPACE_3).abs()
                < 0.5,
            "value x {} should be {} right of key x {}",
            value.x,
            size::KEY_VALUE_KEY + spacing::SPACE_3,
            key.x
        );
    }

    #[test]
    fn given_long_port_value_when_laid_out_then_it_wraps_inside_the_width_and_keeps_its_text()
     {
        let palette = UiPalette::otty_dark();
        let ports = "0.0.0.0:8080->80/tcp, [::]:8080->80/tcp";
        let list: Element<'_, ()> = iced::widget::container(
            KeyValueList::new(&palette).entry("Ports", ports),
        )
        .width(160)
        .into();
        let mut ui = simulator(list);

        let value = ui.find(ports).expect("value should be shown").bounds();

        assert!(
            value.x + value.width <= 160.5,
            "value {value:?} should stay inside 160px"
        );
        assert!(
            value.height > TEXT_SIZE * 2.0,
            "value {value:?} should wrap onto several lines"
        );
    }

    #[test]
    fn given_long_value_when_laid_out_then_it_wraps_inside_the_width() {
        let palette = UiPalette::otty_dark();
        let long = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef";
        let list: Element<'_, ()> = iced::widget::container(
            KeyValueList::new(&palette).entry("ID", long),
        )
        .width(200)
        .into();
        let mut ui = simulator(list);

        let value = ui.find(long).expect("value should be shown").bounds();

        assert!(
            value.x + value.width <= 200.5,
            "value {value:?} should stay inside 200px"
        );
        assert!(
            value.height > TEXT_SIZE * 2.0,
            "value {value:?} should wrap onto several lines"
        );
    }

    /// A list with two port links and a volume link followed by text.
    fn linked_list(palette: &UiPalette) -> Element<'_, Message> {
        KeyValueList::new(palette)
            .entry_parts(
                "Ports",
                [
                    ValuePart::link("8080:80/tcp", Message::Port(8080)),
                    ValuePart::text(", "),
                    ValuePart::link("8443:443/tcp", Message::Port(8443)),
                    ValuePart::text(", 53/udp"),
                ],
            )
            .entry_parts(
                "Mounts",
                [
                    ValuePart::link("otty-fx-data", Message::Volume),
                    ValuePart::text(" → /data"),
                ],
            )
            .into()
    }

    #[test]
    fn given_two_links_in_one_value_when_each_is_pressed_then_each_sends_its_own_message()
     {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_list(&palette));

        ui.click("8443:443/tcp").expect("second link should show");
        ui.click("8080:80/tcp").expect("first link should show");
        ui.click("otty-fx-data").expect("volume link should show");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::Port(8443), Message::Port(8080), Message::Volume]
        );
    }

    #[test]
    fn given_links_when_laid_out_then_they_sit_side_by_side_in_their_value() {
        let palette = UiPalette::otty_light();
        let mut ui = simulator(linked_list(&palette));

        let value = ui
            .find("8080:80/tcp, 8443:443/tcp, 53/udp")
            .expect("whole value should show")
            .bounds();
        let first = ui.find("8080:80/tcp").expect("first link").bounds();
        let second = ui.find("8443:443/tcp").expect("second link").bounds();

        assert!((first.x - value.x).abs() < 0.5, "{first:?} in {value:?}");
        assert!(
            second.x > first.x + first.width,
            "{second:?} after {first:?}"
        );
        assert!((second.y - first.y).abs() < 0.5, "{second:?} on one line");
        assert!(first.width < value.width, "a link is as wide as its text");
    }

    #[test]
    fn given_text_beside_a_link_when_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_list(&palette));
        let link = ui.find("otty-fx-data").expect("volume link").bounds();
        // Inside " → /data", right after the link.
        let beside =
            Point::new(link.x + link.width + 3.0 * TEXT_SIZE, link.center_y());

        ui.point_at(beside);
        let _ = ui.simulate(simulator::click());
        ui.click("Mounts").expect("key should show");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn given_a_long_linked_value_when_laid_out_then_it_wraps_and_each_link_still_works()
     {
        let palette = UiPalette::otty_dark();
        let list: Element<'_, Message> =
            iced::widget::container(KeyValueList::new(&palette).entry_parts(
                "Ports",
                [
                    ValuePart::link("18080:8080/tcp", Message::Port(18080)),
                    ValuePart::text(", "),
                    ValuePart::link("18443:8443/tcp", Message::Port(18443)),
                ],
            ))
            .width(96)
            .into();
        let mut ui = simulator(list);

        let value = ui
            .find("18080:8080/tcp, 18443:8443/tcp")
            .expect("whole value should show")
            .bounds();
        let first = ui.find("18080:8080/tcp").expect("first link").bounds();
        let second = ui.find("18443:8443/tcp").expect("second link").bounds();
        ui.click("18443:8443/tcp").expect("second link");
        ui.click("18080:8080/tcp").expect("first link");

        assert!(value.x + value.width <= 96.5, "{value:?} inside 96px");
        assert!(value.height > TEXT_SIZE * 2.0, "{value:?} should wrap");
        assert!(second.y > first.y, "{second:?} on the next line");
        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Port(18443), Message::Port(18080)]);
    }

    /// The linked list inside a container `width` pixels wide.
    fn linked_list_in(palette: &UiPalette, width: f32) -> Element<'_, Message> {
        iced::widget::container(linked_list(palette))
            .width(width)
            .into()
    }

    #[test]
    fn given_a_wide_width_when_laid_out_then_the_value_sits_right_of_its_key() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_list_in(&palette, 400.0));

        let key = ui.find("Mounts").expect("key").bounds();
        let value = ui.find("otty-fx-data").expect("link").bounds();

        assert!(
            value.x > key.x + key.width - 0.5,
            "{value:?} right of {key:?}"
        );
        assert!(
            (value.y - key.y).abs() < 4.0,
            "{value:?} level with {key:?}"
        );
    }

    #[test]
    fn given_a_narrow_width_when_laid_out_then_the_value_sits_below_its_key() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_list_in(
            &palette,
            size::KEY_VALUE_STACK_BELOW - 20.0,
        ));

        let key = ui.find("Mounts").expect("key").bounds();
        let value = ui.find("otty-fx-data").expect("link").bounds();

        assert!((value.x - key.x).abs() < 0.5, "{value:?} under {key:?}");
        assert!(
            value.y >= key.y + key.height - 0.5,
            "{value:?} below {key:?}"
        );
    }

    #[test]
    fn given_the_threshold_width_when_laid_out_then_it_is_still_two_columns() {
        let palette = UiPalette::otty_dark();
        let mut ui =
            simulator(linked_list_in(&palette, size::KEY_VALUE_STACK_BELOW));

        let key = ui.find("Mounts").expect("key").bounds();
        let value = ui.find("otty-fx-data").expect("link").bounds();

        assert!(
            value.x > key.x + key.width - 0.5,
            "{value:?} right of {key:?}"
        );
    }

    #[test]
    fn given_a_narrow_width_when_links_are_pressed_then_each_sends_its_message()
    {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_list_in(&palette, 200.0));

        ui.click("8443:443/tcp").expect("port link");
        ui.click("otty-fx-data").expect("volume link");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Port(8443), Message::Volume]);
    }

    #[test]
    fn given_a_long_value_in_a_narrow_list_when_laid_out_then_it_wraps_inside_the_width()
     {
        let palette = UiPalette::otty_dark();
        let long = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef";
        let list: Element<'_, ()> = iced::widget::container(
            KeyValueList::new(&palette).entry("ID", long),
        )
        .width(150)
        .into();
        let mut ui = simulator(list);

        let value = ui.find(long).expect("value").bounds();

        assert!(value.x + value.width <= 150.5, "{value:?} inside 150px");
        assert!(value.height > TEXT_SIZE * 2.0, "{value:?} should wrap");
    }

    #[test]
    fn given_a_list_in_a_scrollable_when_laid_out_then_it_sizes_to_its_content()
    {
        let palette = UiPalette::otty_dark();
        let list: Element<'_, Message> = iced::widget::container(
            iced::widget::scrollable(linked_list(&palette)),
        )
        .width(200)
        .height(400)
        .into();
        let mut ui = simulator(list);

        let key = ui.find("Ports").expect("first key").bounds();
        let last = ui.find("otty-fx-data").expect("last link").bounds();

        assert!(last.y > key.y, "{last:?} below {key:?}");
        assert!(last.y < 200.0, "{last:?} sits near the top, not centered");
    }

    /// Draws the linked list with the pointer at `pointer` and checks it
    /// against the picture saved in `reference`; the first call saves it.
    fn hovered_list_matches(
        pointer: Point,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_list(&palette));
        ui.point_at(pointer);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: pointer,
        })]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("list should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn given_a_link_when_the_pointer_is_over_it_then_it_is_underlined() {
        let reference = std::env::temp_dir()
            .join(format!("otty-key-value-list-{}", std::process::id()))
            .join("link-hovered");
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_list(&palette));
        let link = ui.find("otty-fx-data").expect("volume link").bounds();
        let on_link = link.center();
        let beside = Point::new(link.center_x(), link.y + 4.0 * link.height);

        assert!(hovered_list_matches(beside, &reference));
        assert!(!hovered_list_matches(on_link, &reference));
        assert!(hovered_list_matches(beside, &reference));
    }

    #[test]
    fn given_each_theme_when_links_drawn_on_panels_then_they_reach_4_5() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            for surface in [palette.surface_panel(), palette.surface_canvas()] {
                let ratio = contrast_ratio(link_color(&palette), surface);

                assert!(ratio >= TEXT_MIN, "link on {surface:?}: {ratio:.2}");
            }
        }
    }
}
