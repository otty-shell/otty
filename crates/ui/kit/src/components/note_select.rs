//! `NoteSelect`: pick one option from a list whose options carry a short
//! note, such as a program and the place it runs.

use iced::widget::{button, row, svg, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::floating_list::FloatingList;
use super::suggestion_input::{ROW_TEXT_SIZE, open_list};
use super::{field_chrome, glyph};
use crate::palette::UiPalette;
use crate::{fonts, radius, spacing, typography};

/// Edge of the caret after the picked option.
const CARET_SIZE: f32 = 12.0;

/// One option of a [`NoteSelect`]: its label and an optional note shown
/// after it, muted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSelectOption {
    label: String,
    note: Option<String>,
}

impl NoteSelectOption {
    /// An option with `label` and no note.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            note: None,
        }
    }

    /// The same option with `note` after its label.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// A dropdown like `Select` whose options each show a muted note after
/// their label, such as "Postgres   Docker". The field shows the
/// picked option with its note and a caret; pressing it sends the toggle
/// message, and the caller opens or closes the list with
/// [`NoteSelect::open`]. The open list floats under the field with the
/// Menu look; pressing an option sends its index, and a press outside the
/// field and the list sends the toggle message again.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::note_select::{NoteSelect, NoteSelectOption};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Toggle,
///     Picked(usize),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let options = vec![
///     NoteSelectOption::new("Postgres").with_note("Docker"),
///     NoteSelectOption::new("Redis").with_note("Podman"),
/// ];
/// let dropdown: Element<'_, Message> =
///     NoteSelect::new(options, Some(0), &palette)
///         .open(true)
///         .on_toggle(Message::Toggle)
///         .on_pick(Message::Picked)
///         .into();
/// ```
pub struct NoteSelect<'a, Message> {
    options: Vec<NoteSelectOption>,
    selected: Option<usize>,
    is_open: bool,
    palette: UiPalette,
    on_toggle: Option<Message>,
    on_pick: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

impl<'a, Message> NoteSelect<'a, Message> {
    /// Creates a closed dropdown over `options` with the option at
    /// `selected` picked.
    pub fn new(
        options: Vec<NoteSelectOption>,
        selected: Option<usize>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            options,
            selected,
            is_open: false,
            palette: palette.clone(),
            on_toggle: None,
            on_pick: None,
        }
    }

    /// Shows the list under the field while `is_open`.
    pub fn open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Sets the message sent when the field is pressed, or a press lands
    /// outside the open list. Without it the field is disabled.
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.on_toggle = Some(message);
        self
    }

    /// Sets the message sent when an option of the list is pressed, built
    /// from its index.
    pub fn on_pick(mut self, on_pick: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_pick = Some(Box::new(on_pick));
        self
    }
}

impl<'a, Message: Clone + 'a> From<NoteSelect<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: NoteSelect<'a, Message>) -> Self {
        let palette = value.palette;
        let picked = value
            .selected
            .and_then(|index| value.options.get(index))
            .cloned();

        let field = field_view(picked, value.on_toggle.clone(), &palette);
        let mut floating = FloatingList::new(field).on_close(value.on_toggle);
        if value.is_open {
            let rows =
                option_rows(value.options, value.on_pick.as_deref(), &palette);
            floating = open_list(floating, rows, value.selected, &palette);
        }

        floating.into()
    }
}

/// The closed field: the picked option with its note, and a caret.
fn field_view<'a, Message: Clone + 'a>(
    picked: Option<NoteSelectOption>,
    on_toggle: Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let body = typography::BODY;
    let strong = palette.text_strong();
    let muted = palette.text_muted();
    let (label, note) = match picked {
        Some(option) => (option.label, option.note),
        None => (String::new(), None),
    };

    let mut content = row![
        text(label)
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI)
            .color(strong)
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);
    if let Some(note) = note {
        content = content.push(
            text(note)
                .size(ROW_TEXT_SIZE)
                .font(fonts::UI)
                .color(muted)
                .width(Length::Fill),
        );
    } else {
        content = content.push(iced::widget::Space::new().width(Length::Fill));
    }
    content = content.push(
        svg(glyph::caret(true))
            .width(CARET_SIZE)
            .height(CARET_SIZE)
            .style(move |_, _| svg::Style { color: Some(muted) }),
    );

    let field_palette = palette.clone();
    button(content)
        .width(Length::Fill)
        .padding(Padding::from([
            field_chrome::PADDING_Y,
            field_chrome::PADDING_X,
        ]))
        .on_press_maybe(on_toggle)
        .style(move |_, status| field_style(&field_palette, status))
        .into()
}

/// The rows of the open list: each option with its note, and what a press
/// on it sends.
fn option_rows<'a, Message: 'a>(
    options: Vec<NoteSelectOption>,
    on_pick: Option<&(dyn Fn(usize) -> Message + 'a)>,
    palette: &UiPalette,
) -> Vec<(Element<'a, Message>, Option<Message>)> {
    let muted = palette.text_muted();

    options
        .into_iter()
        .enumerate()
        .map(|(index, option)| {
            let mut content = row![
                text(option.label)
                    .size(ROW_TEXT_SIZE)
                    .font(fonts::UI)
                    .width(Length::Fill)
            ]
            .spacing(spacing::SPACE_3)
            .align_y(Alignment::Center);
            if let Some(note) = option.note {
                content = content.push(
                    text(note).size(ROW_TEXT_SIZE).font(fonts::UI).color(muted),
                );
            }

            (content.into(), on_pick.map(|on_pick| on_pick(index)))
        })
        .collect()
}

/// The look of the field, the same as `Select`: `surface-raised`, and
/// `surface-hover` under the pointer.
fn field_style(palette: &UiPalette, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            palette.surface_hover()
        },
        _ => palette.surface_raised(),
    };

    button::Style {
        background: Some(background.into()),
        text_color: palette.text_strong(),
        border: Border {
            radius: radius::RADIUS_SM.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Toggle,
        Picked(usize),
    }

    fn options() -> Vec<NoteSelectOption> {
        vec![
            NoteSelectOption::new("Postgres").with_note("Docker"),
            NoteSelectOption::new("Redis").with_note("Podman"),
        ]
    }

    fn dropdown(is_open: bool) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();

        NoteSelect::new(options(), Some(0), &palette)
            .open(is_open)
            .on_toggle(Message::Toggle)
            .on_pick(Message::Picked)
            .into()
    }

    #[test]
    fn the_closed_field_shows_the_picked_option_with_its_note() {
        let mut ui = simulator(dropdown(false));

        ui.find("Postgres").expect("the picked option should show");
        ui.find("Docker").expect("its note should show");
        assert!(ui.find("Redis").is_err(), "the list is closed");
    }

    #[test]
    fn pressing_the_field_sends_the_toggle_message() {
        let mut ui = simulator(dropdown(false));

        ui.click("Postgres").expect("the field should show");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert_eq!(sent, vec![Message::Toggle]);
    }

    #[test]
    fn the_open_list_shows_each_option_with_its_note_and_sends_its_index() {
        let mut ui = simulator(dropdown(true));

        ui.find("Podman").expect("the note of Redis should show");
        ui.click("Redis").expect("Redis should show in the list");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert_eq!(sent, vec![Message::Picked(1)]);
    }

    /// A list of `count` options, `Option 00` and on, the one at
    /// `selected` picked.
    fn long_list(count: usize, selected: usize) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();
        let options = (0..count)
            .map(|index| NoteSelectOption::new(format!("Option {index:02}")))
            .collect();

        NoteSelect::new(options, Some(selected), &palette)
            .open(true)
            .on_toggle(Message::Toggle)
            .on_pick(Message::Picked)
            .into()
    }

    /// A long list grows no taller than the lists of the other fields: it
    /// shows about eight options, then scrolls.
    #[test]
    fn a_long_list_shows_about_eight_options_then_scrolls() {
        let mut ui = simulator(long_list(20, 0));

        let eighth = ui.find("Option 07").expect("option should be listed");
        let ninth = ui.find("Option 08").expect("option should be listed");

        assert!(eighth.visible_bounds().is_some());
        assert!(ninth.visible_bounds().is_none(), "the ninth option scrolls");
    }

    #[test]
    fn the_picked_option_of_a_long_list_is_kept_in_view() {
        let mut ui = simulator(long_list(20, 15));

        let picked = ui.find("Option 15").expect("option should be listed");
        let first = ui.find("Option 00").expect("option should be listed");

        assert!(picked.visible_bounds().is_some());
        assert!(first.visible_bounds().is_none(), "the list has scrolled");
    }
}
