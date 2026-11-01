//! `MappingField`: rows that map something on this computer to something in
//! a container, such as ports and volumes (`.port-row`, `.vol-row`).

use iced::widget::{Space, column, container, row, text, text_input};
use iced::{Alignment, Element, Length, Padding, Pixels};

use super::button::{Button, ButtonKind, ButtonSize};
use super::field_chrome::{self, Note, input_style};
use super::glyph;
use super::icon_button::IconButton;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Horizontal padding of an input (`padding: 0 11px`).
const PADDING_X: f32 = 11.0;
/// Vertical padding that makes a 13/20 text line 34px high.
const PADDING_Y: f32 = 7.0;
/// Width of the arrow column (`16px`).
const ARROW_WIDTH: f32 = 16.0;
/// Width of the column of the per-row control unless the caller sets one
/// (`width: 108px` on the screen).
const CONTROL_WIDTH: f32 = 108.0;

/// One row of a [`MappingField`]: the source input, an arrow, the target
/// input, an optional control (for example the protocol or a read-only
/// switch) and a cross. Without input handlers the inputs are disabled.
pub struct MappingRow<'a, Message> {
    source: &'a str,
    target: &'a str,
    source_placeholder: String,
    target_placeholder: String,
    is_source_invalid: bool,
    is_target_invalid: bool,
    error: Option<String>,
    control: Option<Element<'a, Message>>,
    on_source: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_target: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_remove: Option<Message>,
    remove_label: String,
}

impl<'a, Message> MappingRow<'a, Message> {
    /// Creates a row with the typed `source` and `target`.
    pub fn new(source: &'a str, target: &'a str) -> Self {
        Self {
            source,
            target,
            source_placeholder: String::new(),
            target_placeholder: String::new(),
            is_source_invalid: false,
            is_target_invalid: false,
            error: None,
            control: None,
            on_source: None,
            on_target: None,
            on_remove: None,
            remove_label: "Remove".to_owned(),
        }
    }

    /// Sets the texts shown while the inputs are empty.
    pub fn placeholders(
        mut self,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        self.source_placeholder = source.into();
        self.target_placeholder = target.into();
        self
    }

    /// Colors the border of the source and target inputs `status-error`.
    pub fn invalid(mut self, is_source: bool, is_target: bool) -> Self {
        self.is_source_invalid = is_source;
        self.is_target_invalid = is_target;
        self
    }

    /// Sets the error shown under the row.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Sets the control between the target input and the cross.
    pub fn control(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.control = Some(control.into());
        self
    }

    /// Sets the function that maps the new source to a message.
    pub fn on_source(
        mut self,
        on_source: impl Fn(String) -> Message + 'a,
    ) -> Self {
        self.on_source = Some(Box::new(on_source));
        self
    }

    /// Sets the function that maps the new target to a message.
    pub fn on_target(
        mut self,
        on_target: impl Fn(String) -> Message + 'a,
    ) -> Self {
        self.on_target = Some(Box::new(on_target));
        self
    }

    /// Sets the message of the cross and its tooltip.
    pub fn on_remove(
        mut self,
        message: Message,
        label: impl Into<String>,
    ) -> Self {
        self.on_remove = Some(message);
        self.remove_label = label.into();
        self
    }
}

/// Rows of source → target inputs under two column headings, each with an
/// optional control and a cross, then an add button and a hint.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::mapping_field::{MappingField, MappingRow};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Host(usize, String),
///     Remove(usize),
///     Add,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let field: Element<'_, Message> =
///     MappingField::new("Host port", "Container port", &palette)
///         .row(
///             MappingRow::new("8081", "80")
///                 .on_source(|value| Message::Host(0, value))
///                 .on_remove(Message::Remove(0), "Remove port 8081"),
///         )
///         .add_label("Add port")
///         .on_add(Message::Add)
///         .hint("The image listens on 80/tcp.")
///         .into();
/// ```
pub struct MappingField<'a, Message> {
    source_heading: String,
    target_heading: String,
    palette: UiPalette,
    rows: Vec<MappingRow<'a, Message>>,
    control_width: f32,
    separator: String,
    hint: Option<String>,
    add_label: String,
    on_add: Option<Message>,
}

impl<'a, Message> MappingField<'a, Message> {
    /// Creates a field with no rows under the two column headings.
    pub fn new(
        source_heading: impl Into<String>,
        target_heading: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            source_heading: source_heading.into(),
            target_heading: target_heading.into(),
            palette: palette.clone(),
            rows: Vec::new(),
            control_width: CONTROL_WIDTH,
            separator: "→".to_owned(),
            hint: None,
            add_label: "Add".to_owned(),
            on_add: None,
        }
    }

    /// Appends a row.
    pub fn row(mut self, row: MappingRow<'a, Message>) -> Self {
        self.rows.push(row);
        self
    }

    /// Sets the width of the control column, so every row lines up.
    pub fn control_width(mut self, width: f32) -> Self {
        self.control_width = width;
        self
    }

    /// Sets the text between the two inputs of every row (default "→").
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    /// Sets the line under the field.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Sets the text of the add button.
    pub fn add_label(mut self, label: impl Into<String>) -> Self {
        self.add_label = label.into();
        self
    }

    /// Sets the message of the add button; without it the button is
    /// disabled.
    pub fn on_add(mut self, message: Message) -> Self {
        self.on_add = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<MappingField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: MappingField<'a, Message>) -> Self {
        let palette = value.palette;
        let control_width = value.control_width;

        // The headings name the inputs, so they show only above rows.
        let mut body = column![].spacing(spacing::SPACE_2);
        if !value.rows.is_empty() {
            body = body.push(heading_row(
                value.source_heading,
                value.target_heading,
                control_width,
                &palette,
            ));
        }
        for mapping in value.rows {
            body = body.push(mapping_row(
                mapping,
                &value.separator,
                control_width,
                &palette,
            ));
        }

        let mut add = Button::new(value.add_label, &palette)
            .kind(ButtonKind::Ghost)
            .size(ButtonSize::Small);
        if let Some(message) = value.on_add {
            add = add.on_press(message);
        }
        body = body.push(add);

        field_chrome::labelled(
            "",
            false,
            body.into(),
            value.hint.map(Note::Hint),
            &palette,
        )
    }
}

/// The muted column headings over the inputs.
fn heading_row<'a, Message: 'a>(
    source: String,
    target: String,
    control_width: f32,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let style = typography::SMALL;
    let heading = |content: String| {
        text(content)
            .size(style.size())
            .font(fonts::UI)
            .color(palette.text_muted())
            .width(Length::Fill)
    };

    row![
        heading(source),
        Space::new().width(ARROW_WIDTH),
        heading(target),
        Space::new().width(control_width),
        Space::new().width(size::CONTROL_MD),
    ]
    .spacing(spacing::SPACE_2)
    .into()
}

/// One row: source, `separator`, target, the control, the cross, and the
/// error under them.
fn mapping_row<'a, Message: Clone + 'a>(
    mapping: MappingRow<'a, Message>,
    separator: &str,
    control_width: f32,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let source = mono_input(
        &mapping.source_placeholder,
        mapping.source,
        mapping.is_source_invalid,
        palette,
    )
    .on_input_maybe(mapping.on_source);
    let target = mono_input(
        &mapping.target_placeholder,
        mapping.target,
        mapping.is_target_invalid,
        palette,
    )
    .on_input_maybe(mapping.on_target);
    let arrow = text(separator.to_owned())
        .font(fonts::UI)
        .color(palette.text_faint())
        .width(ARROW_WIDTH)
        .align_x(Alignment::Center);
    let control: Element<'a, Message> = match mapping.control {
        Some(control) => control,
        None => Space::new().into(),
    };
    let mut cross =
        IconButton::new(glyph::cross(), mapping.remove_label, palette);
    if let Some(message) = mapping.on_remove {
        cross = cross.on_press(message);
    }

    let line = row![
        source.width(Length::Fill),
        arrow,
        target.width(Length::Fill),
        container(control).width(control_width),
        container(cross).width(size::CONTROL_MD),
    ]
    .spacing(spacing::SPACE_2)
    .align_y(Alignment::Center);

    let Some(error) = mapping.error else {
        return line.into();
    };
    let style = typography::SMALL;
    column![
        line,
        text(error)
            .size(style.size())
            .font(fonts::UI)
            .color(palette.status_error()),
    ]
    .spacing(spacing::SPACE_1)
    .into()
}

/// A boxed mono input of control-lg height; `invalid` colors its border.
fn mono_input<'a, Message: Clone + 'a>(
    placeholder: &str,
    content: &str,
    invalid: bool,
    palette: &UiPalette,
) -> text_input::TextInput<'a, Message> {
    let style = typography::TERM;

    text_input(placeholder, content)
        .font(fonts::MONO)
        .size(style.size())
        .line_height(Pixels(style.line_height()))
        .padding(Padding::from([PADDING_Y, PADDING_X]))
        .style(input_style(palette, invalid))
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;

    /// Width of the field in the tests.
    const WIDTH: f32 = 600.0;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Source(usize, String),
        Target(usize, String),
        Remove(usize),
        Add,
    }

    fn ports(palette: &UiPalette) -> Element<'_, Message> {
        let field = MappingField::new("Host port", "Container port", palette)
            .row(
                MappingRow::new("8081", "80")
                    .on_source(|value| Message::Source(0, value))
                    .on_target(|value| Message::Target(0, value))
                    .on_remove(Message::Remove(0), "Remove port 8081"),
            )
            .row(
                MappingRow::new("80800", "443")
                    .invalid(true, false)
                    .error("A port is a number from 1 to 65535.")
                    .on_source(|value| Message::Source(1, value))
                    .on_target(|value| Message::Target(1, value))
                    .on_remove(Message::Remove(1), "Remove port 80800"),
            )
            .add_label("Add port")
            .on_add(Message::Add)
            .hint("The image listens on 80/tcp.");

        container(field).width(WIDTH).into()
    }

    #[test]
    fn given_rows_when_drawn_then_headings_values_error_and_hint_show() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let mut ui = simulator(ports(&palette));

            for shown in [
                "Host port",
                "Container port",
                "8081",
                "443",
                "→",
                "A port is a number from 1 to 65535.",
                "The image listens on 80/tcp.",
                "Add port",
            ] {
                assert!(ui.find(shown).is_ok(), "{shown} should be visible");
            }
        }
    }

    #[test]
    fn given_a_separator_when_drawn_then_it_replaces_the_arrow() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> =
            MappingField::new("Host port", "Container port", &palette)
                .separator(":")
                .row(MappingRow::new("8081", "80"))
                .into();
        let mut ui = simulator(field);

        assert!(ui.find(":").is_ok(), "the separator should be visible");
        assert!(ui.find("→").is_err(), "the arrow should be gone");
    }

    #[test]
    fn given_a_row_when_typing_in_its_inputs_then_the_row_is_reported() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(ports(&palette));

        ui.click("443").expect("target input");
        let _ = ui.typewrite("1");
        ui.click("8081").expect("source input");
        let _ = ui.typewrite("2");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            matches!(messages.first(), Some(Message::Target(1, _))),
            "{messages:?}"
        );
        assert!(
            matches!(messages.last(), Some(Message::Source(0, _))),
            "{messages:?}"
        );
    }

    #[test]
    fn given_a_row_when_its_cross_is_pressed_then_that_row_is_removed() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(ports(&palette));
        let input = ui.find("8081").expect("source input").bounds();

        ui.point_at(Point::new(
            WIDTH - size::CONTROL_MD / 2.0,
            input.y + input.height / 2.0,
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Remove(0)]);
    }

    #[test]
    fn given_the_add_button_when_pressed_then_add_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(ports(&palette));

        ui.click("Add port").expect("add button");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Add]);
    }

    #[test]
    fn given_a_row_without_handlers_when_typing_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> =
            MappingField::new("Source", "Path in the container", &palette)
                .row(MappingRow::new("otty-fx-cache", "/cache"))
                .into();
        let mut ui = simulator(container(field).width(WIDTH));

        ui.click("/cache").expect("target input");
        let _ = ui.typewrite("x");
        let _ = ui.click("Add");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_a_control_when_drawn_then_it_sits_between_target_and_cross() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> =
            MappingField::new("Source", "Path in the container", &palette)
                .row(
                    MappingRow::new("/srv", "/data").control(text("Read only")),
                )
                .into();
        let mut ui = simulator(container(field).width(WIDTH));

        let target = ui.find("/data").expect("target").bounds();
        let control = ui.find("Read only").expect("control").bounds();

        assert!(control.x >= target.x + target.width, "{control:?}");
        assert!(
            control.x + control.width <= WIDTH - size::CONTROL_MD,
            "{control:?}"
        );
    }
}
