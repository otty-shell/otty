//! `KeyValueField`: environment variables or labels as name and value rows.

use std::rc::Rc;

use iced::widget::{column, container, row, text_input};
use iced::{Alignment, Element, Length, Padding, Pixels};

use super::field_chrome::{self, Note, input_style};
use super::glyph;
use super::icon_button::IconButton;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Horizontal padding of an input (`padding: 0 11px`).
const PADDING_X: f32 = 11.0;
/// Vertical padding that makes a 13/20 text line 34px high.
const PADDING_Y: f32 = 7.0;
/// Share of the name column (`minmax(0, 1fr)`).
const NAME_PORTION: u16 = 2;
/// Share of the value column (`minmax(0, 1.5fr)`).
const VALUE_PORTION: u16 = 3;

/// Rows of name and value inputs, a cross on each row and an add button.
/// Handlers that are not set leave the matching control disabled.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::key_value_field::KeyValueField;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Name(usize, String),
///     Value(usize, String),
///     Remove(usize),
///     Add,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let rows = [("RUST_LOG".to_owned(), "debug".to_owned())];
/// let field: Element<'_, Message> =
///     KeyValueField::new("Environment variables", &rows, &palette)
///         .placeholders("NAME", "value")
///         .add_label("Add variable")
///         .remove_label("Remove variable")
///         .on_name(Message::Name)
///         .on_value(Message::Value)
///         .on_remove(Message::Remove)
///         .on_add(Message::Add)
///         .into();
/// ```
pub struct KeyValueField<'a, Message> {
    label: String,
    rows: &'a [(String, String)],
    palette: UiPalette,
    hint: Option<String>,
    optional: Option<String>,
    add_label: String,
    remove_label: String,
    name_placeholder: String,
    value_placeholder: String,
    on_name: Option<Rc<dyn Fn(usize, String) -> Message + 'a>>,
    on_value: Option<Rc<dyn Fn(usize, String) -> Message + 'a>>,
    on_remove: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_add: Option<Message>,
}

impl<'a, Message> KeyValueField<'a, Message> {
    /// Creates a field over `rows` of `(name, value)`. Its other texts come
    /// from the app, in its language, through the methods below.
    pub fn new(
        label: impl Into<String>,
        rows: &'a [(String, String)],
        palette: &UiPalette,
    ) -> Self {
        Self {
            label: label.into(),
            rows,
            palette: palette.clone(),
            hint: None,
            optional: None,
            add_label: String::new(),
            remove_label: String::new(),
            name_placeholder: String::new(),
            value_placeholder: String::new(),
            on_name: None,
            on_value: None,
            on_remove: None,
            on_add: None,
        }
    }

    /// Sets the line under the field.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Marks the field as one that may stay empty: `marker` shows beside the
    /// label, in the language of the app.
    pub fn optional(mut self, marker: impl Into<String>) -> Self {
        self.optional = Some(marker.into());
        self
    }

    /// Sets the text of the add button.
    pub fn add_label(mut self, label: impl Into<String>) -> Self {
        self.add_label = label.into();
        self
    }

    /// Sets the tooltip of the cross on each row.
    pub fn remove_label(mut self, label: impl Into<String>) -> Self {
        self.remove_label = label.into();
        self
    }

    /// Sets the texts shown while the name and the value inputs are empty.
    pub fn placeholders(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.name_placeholder = name.into();
        self.value_placeholder = value.into();
        self
    }

    /// Sets the function that maps `(row, new name)` to a message.
    pub fn on_name(
        mut self,
        on_name: impl Fn(usize, String) -> Message + 'a,
    ) -> Self {
        self.on_name = Some(Rc::new(on_name));
        self
    }

    /// Sets the function that maps `(row, new value)` to a message.
    pub fn on_value(
        mut self,
        on_value: impl Fn(usize, String) -> Message + 'a,
    ) -> Self {
        self.on_value = Some(Rc::new(on_value));
        self
    }

    /// Sets the function that maps a row to the message of its cross.
    pub fn on_remove(
        mut self,
        on_remove: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        self.on_remove = Some(Box::new(on_remove));
        self
    }

    /// Sets the message sent by the add button.
    pub fn on_add(mut self, message: Message) -> Self {
        self.on_add = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<KeyValueField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: KeyValueField<'a, Message>) -> Self {
        let palette = value.palette;

        let mut body = column![].spacing(spacing::SPACE_2);
        for (index, (name, content)) in value.rows.iter().enumerate() {
            let name_input =
                mono_input(&value.name_placeholder, name, &palette)
                    .on_input_maybe(
                        value
                            .on_name
                            .clone()
                            .map(|on_name| move |text| on_name(index, text)),
                    );
            let value_input =
                mono_input(&value.value_placeholder, content, &palette)
                    .on_input_maybe(
                        value
                            .on_value
                            .clone()
                            .map(|on_value| move |text| on_value(index, text)),
                    );
            let mut cross = IconButton::new(
                glyph::cross(),
                value.remove_label.as_str(),
                &palette,
            );
            if let Some(on_remove) = &value.on_remove {
                cross = cross.on_press(on_remove(index));
            }

            body = body.push(
                row![
                    name_input.width(Length::FillPortion(NAME_PORTION)),
                    value_input.width(Length::FillPortion(VALUE_PORTION)),
                    container(cross).width(size::CONTROL_MD),
                ]
                .spacing(spacing::SPACE_2)
                .align_y(Alignment::Center),
            );
        }

        body = body.push(field_chrome::add_button(
            value.add_label,
            value.on_add,
            &palette,
        ));

        field_chrome::labelled(
            &value.label,
            value.optional.as_deref(),
            body.into(),
            value.hint.map(Note::Hint),
            &palette,
        )
    }
}

/// Builds a boxed mono input of control-lg height.
fn mono_input<'a, Message: Clone + 'a>(
    placeholder: &str,
    content: &str,
    palette: &UiPalette,
) -> text_input::TextInput<'a, Message> {
    let style = typography::TERM;

    text_input(placeholder, content)
        .font(fonts::MONO)
        .size(style.size())
        .line_height(Pixels(style.line_height()))
        .padding(Padding::from([PADDING_Y, PADDING_X]))
        .style(input_style(palette, false))
}

#[cfg(test)]
mod tests {
    use iced::Theme;
    use iced_test::{Simulator, simulator};

    use super::super::button::{Button, ButtonKind, ButtonSize};
    use super::super::mapping_field::MappingField;
    use super::*;

    /// Says whether `field` is drawn exactly as the drawing saved at
    /// `reference`; the first drawing of a `reference` saves it.
    fn matches_drawing(
        field: Element<'_, ()>,
        reference: &std::path::Path,
    ) -> bool {
        Simulator::with_size(iced::Settings::default(), (240.0, 60.0), field)
            .snapshot(&Theme::Dark)
            .expect("the field should draw")
            .matches_hash(reference)
            .expect("the reference should be readable")
    }

    #[test]
    fn given_no_rows_when_drawn_then_the_add_button_looks_like_the_one_of_a_mapping_field()
     {
        for (name, palette) in [
            ("dark", UiPalette::otty_dark()),
            ("light", UiPalette::otty_light()),
        ] {
            let folder = std::env::temp_dir().join(format!(
                "otty-kit-add-button-{}-{name}",
                std::process::id()
            ));
            let reference = folder.join("button");
            let mapping = MappingField::new("", "", &palette)
                .add_label("Add")
                .on_add(());
            let key_value = KeyValueField::new("", &[], &palette)
                .add_label("Add")
                .on_add(());

            let mapping_saved = matches_drawing(mapping.into(), &reference);
            let same = matches_drawing(key_value.into(), &reference);
            let _ = std::fs::remove_dir_all(&folder);

            assert!(mapping_saved);
            assert!(same, "the two add buttons differ in {name}");
        }
    }

    #[test]
    fn given_no_rows_when_drawn_then_the_add_button_is_a_plain_small_button() {
        for (name, palette) in [
            ("dark", UiPalette::otty_dark()),
            ("light", UiPalette::otty_light()),
        ] {
            let folder = std::env::temp_dir().join(format!(
                "otty-kit-plain-add-button-{}-{name}",
                std::process::id()
            ));
            let reference = folder.join("button");
            let plain = Button::new("Add", &palette)
                .kind(ButtonKind::Ghost)
                .size(ButtonSize::Small)
                .on_press(());
            let key_value = KeyValueField::new("", &[], &palette)
                .add_label("Add")
                .on_add(());

            let plain_saved = matches_drawing(plain.into(), &reference);
            let same = matches_drawing(key_value.into(), &reference);
            let _ = std::fs::remove_dir_all(&folder);

            assert!(plain_saved);
            assert!(same, "the add button is not plain in {name}");
        }
    }

    #[test]
    fn given_an_optional_marker_when_drawn_then_it_reads_as_given() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, ()> =
            KeyValueField::new("Окружение", &[], &palette)
                .optional("необязательно")
                .into();
        let mut ui = simulator(field);

        assert!(ui.find("необязательно").is_ok());
        assert!(ui.find("optional").is_err());
    }

    /// Text that shows comes from the app, in its language: the kit has none.
    #[test]
    fn no_text_of_the_kit_shows_until_the_app_gives_it() {
        let palette = UiPalette::otty_dark();

        let field: KeyValueField<'_, ()> =
            KeyValueField::new("Environment", &[], &palette);

        assert_eq!(field.name_placeholder, "");
        assert_eq!(field.value_placeholder, "");
        assert_eq!(field.add_label, "");
    }

    /// Says whether a row is drawn as saved in `reference`, with the texts the
    /// app gives and the pointer on the cross; the first call saves it. iced
    /// draws tooltips as overlays that widget queries do not reach, so they are
    /// checked on the drawing.
    fn hovered_row_matches(
        name_placeholder: &str,
        remove_label: &str,
        reference: &std::path::Path,
    ) -> bool {
        const WIDTH: f32 = 300.0;

        let palette = UiPalette::otty_dark();
        let rows = [(String::new(), "typed".to_owned())];
        let field = KeyValueField::new("", &rows, &palette)
            .placeholders(name_placeholder, "")
            .remove_label(remove_label)
            .on_remove(|_| ());
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (WIDTH, 120.0),
            container(field).width(WIDTH),
        );
        let row = ui.find("typed").expect("the value").bounds();
        let position = iced::Point::new(
            WIDTH - size::CONTROL_MD / 2.0,
            row.y + row.height / 2.0,
        );
        ui.point_at(position);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved { position },
        )]);

        ui.snapshot(&Theme::Dark)
            .expect("the field should draw")
            .matches_hash(reference)
            .expect("the reference should be readable")
    }

    #[test]
    fn the_cross_of_a_row_shows_the_label_the_app_gives() {
        let folder = crate::test_support::TempDir::new("key-value-cross");
        let reference = folder.path().join("cross");

        assert!(hovered_row_matches("", "Remove variable", &reference));
        assert!(!hovered_row_matches("", "Variable entfernen", &reference));
    }

    #[test]
    fn an_empty_name_shows_the_placeholder_the_app_gives() {
        let folder = crate::test_support::TempDir::new("key-value-placeholder");
        let reference = folder.path().join("placeholder");

        assert!(hovered_row_matches("NAME", "", &reference));
        assert!(!hovered_row_matches("ИМЯ", "", &reference));
    }

    #[test]
    fn each_column_takes_its_own_placeholder() {
        let palette = UiPalette::otty_dark();

        let field: KeyValueField<'_, ()> =
            KeyValueField::new("Environment", &[], &palette)
                .placeholders("Name", "Value");

        assert_eq!(field.name_placeholder, "Name");
        assert_eq!(field.value_placeholder, "Value");
    }
}
