//! `NoteSelect`: pick one option from a list whose options carry a short
//! note, such as an agent and the company of its models.

use iced::widget::{button, column, container, row, svg, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::floating_list::FloatingList;
use super::glyph;
use super::menu::{MENU_PADDING, item_style};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Horizontal padding of the field, as in `Select` (`padding: 0 10px 0
/// 11px`).
const PADDING_X: f32 = 11.0;
/// Vertical padding that makes a 13/20 text line 34px high, as in
/// `Select`.
const PADDING_Y: f32 = 7.0;
/// Horizontal padding of a row of the list (`padding: 0 10px`).
const ROW_PADDING_X: f32 = 10.0;
/// Text size of the rows of the list (`font-size: 12.5px`).
const ROW_TEXT_SIZE: f32 = 12.5;
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
    /// The label of the option.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The note of the option, when it has one.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

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
/// their label, such as "Claude Code   Anthropic". The field shows the
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
///     NoteSelectOption::new("Claude Code").with_note("Anthropic"),
///     NoteSelectOption::new("Codex").with_note("OpenAI"),
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
            let list = list_view(
                value.options,
                value.selected,
                value.on_pick.as_deref(),
                &palette,
            );
            floating = floating.list(list, value.selected, size::ROW);
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
        .padding(Padding::from([PADDING_Y, PADDING_X]))
        .on_press_maybe(on_toggle)
        .style(move |_, status| field_style(&field_palette, status))
        .into()
}

/// The open list: each option with its note, the picked one lit.
fn list_view<'a, Message: Clone + 'a>(
    options: Vec<NoteSelectOption>,
    selected: Option<usize>,
    on_pick: Option<&(dyn Fn(usize) -> Message + 'a)>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let muted = palette.text_muted();
    let mut rows = column![];
    for (index, option) in options.into_iter().enumerate() {
        let mut content = row![
            text(option.label)
                .size(ROW_TEXT_SIZE)
                .font(fonts::UI)
                .width(Length::Fill)
        ]
        .spacing(spacing::SPACE_3)
        .height(Length::Fill)
        .align_y(Alignment::Center);
        if let Some(note) = option.note {
            content = content.push(
                text(note).size(ROW_TEXT_SIZE).font(fonts::UI).color(muted),
            );
        }

        rows = rows.push(
            button(content)
                .width(Length::Fill)
                .height(size::ROW)
                .padding(Padding::from([0.0, ROW_PADDING_X]))
                .on_press_maybe(on_pick.map(|on_pick| on_pick(index)))
                .style(item_style(palette, selected == Some(index))),
        );
    }

    let background = palette.surface_overlay();
    let shadow = palette.shadow_popover();
    container(rows)
        .width(Length::Fill)
        .padding(MENU_PADDING)
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_MD.into(),
                ..Border::default()
            },
            shadow,
            ..container::Style::default()
        })
        .into()
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

    fn agents() -> Vec<NoteSelectOption> {
        vec![
            NoteSelectOption::new("Claude Code").with_note("Anthropic"),
            NoteSelectOption::new("Codex").with_note("OpenAI"),
        ]
    }

    fn dropdown(is_open: bool) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();

        NoteSelect::new(agents(), Some(0), &palette)
            .open(is_open)
            .on_toggle(Message::Toggle)
            .on_pick(Message::Picked)
            .into()
    }

    #[test]
    fn the_closed_field_shows_the_picked_option_with_its_note() {
        let mut ui = simulator(dropdown(false));

        ui.find("Claude Code")
            .expect("the picked option should show");
        ui.find("Anthropic").expect("its note should show");
        assert!(ui.find("Codex").is_err(), "the list is closed");
    }

    #[test]
    fn pressing_the_field_sends_the_toggle_message() {
        let mut ui = simulator(dropdown(false));

        ui.click("Claude Code").expect("the field should show");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert_eq!(sent, vec![Message::Toggle]);
    }

    #[test]
    fn the_open_list_shows_each_option_with_its_note_and_sends_its_index() {
        let mut ui = simulator(dropdown(true));

        ui.find("OpenAI").expect("the note of Codex should show");
        ui.click("Codex").expect("Codex should show in the list");

        let sent: Vec<Message> = ui.into_messages().collect();
        assert_eq!(sent, vec![Message::Picked(1)]);
    }

    #[test]
    fn an_option_keeps_its_label_and_note() {
        let option = NoteSelectOption::new("Codex").with_note("OpenAI");

        assert_eq!(option.label(), "Codex");
        assert_eq!(option.note(), Some("OpenAI"));
        assert_eq!(NoteSelectOption::new("Gemini").note(), None);
    }
}
