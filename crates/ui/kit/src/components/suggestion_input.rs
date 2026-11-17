//! A text input with suggestions floating under it: the one way a field of
//! the kit suggests, whatever it suggests. [`TagSearchField`] suggests keys,
//! values and names; [`TextField`] suggests what the app gives it, such as
//! folders. Each gives only its rows, the row that is highlighted and what
//! each key and each row sends.
//!
//! The piece wires the rest: the keys bound on the focused input (they do
//! not reach the input), the list that opens while there are rows, a press
//! on a row, a press away, the highlighted row kept in view, the cursor
//! moved to the end when the app replaces the text (a pick), and the look of
//! the list and of its rows, as a menu.
//!
//! [`TagSearchField`]: super::tag_search_field::TagSearchField
//! [`TextField`]: super::text_field::TextField

use iced::keyboard::key::Named;
use iced::widget::text_input::TextInput;
use iced::widget::{button, column, container, scrollable};
use iced::{Alignment, Border, Element, Length, Padding};

use super::floating_list::FloatingList;
use super::input_keys::InputKeys;
use super::{menu, scrollbar};
use crate::palette::UiPalette;
use crate::{radius, size};

/// Text size of a row, as in a menu item (`font-size: 12.5px`).
pub(super) const ROW_TEXT_SIZE: f32 = 12.5;
/// Rows the list shows before it scrolls.
const LIST_ROWS: f32 = 8.0;
/// Tallest the list grows: its rows and its padding.
const LIST_MAX_HEIGHT: f32 = LIST_ROWS * size::ROW + 2.0 * menu::MENU_PADDING;
/// Horizontal padding of a row, as in a menu item (`padding: 0 10px`).
const ROW_PADDING_X: f32 = 10.0;

/// A text input and the rows suggested for it. The list shows while there
/// are rows, so the owner closes it by giving none. The input is always
/// wrapped the same way, with the list open or not, so it keeps the
/// keyboard while the rows come and go.
pub(super) struct SuggestionInput<'a, Message> {
    input: TextInput<'a, Message>,
    text: &'a str,
    palette: UiPalette,
    rows: Vec<(Element<'a, Message>, Option<Message>)>,
    highlighted: Option<usize>,
    keys: Vec<(Named, Message)>,
    on_focus: Option<Message>,
    on_close: Option<Message>,
}

impl<'a, Message: Clone + 'a> SuggestionInput<'a, Message> {
    /// Wraps `input`, which shows `text`, with no rows.
    pub(super) fn new(
        input: TextInput<'a, Message>,
        text: &'a str,
        palette: &UiPalette,
    ) -> Self {
        Self {
            input,
            text,
            palette: palette.clone(),
            rows: Vec::new(),
            highlighted: None,
            keys: Vec::new(),
            on_focus: None,
            on_close: None,
        }
    }

    /// Adds a row showing `content`; a press on it sends `on_press`.
    pub(super) fn row(
        mut self,
        content: impl Into<Element<'a, Message>>,
        on_press: Option<Message>,
    ) -> Self {
        self.rows.push((content.into(), on_press));
        self
    }

    /// Lights the row at `highlighted` and keeps it in view.
    pub(super) fn highlighted(mut self, highlighted: Option<usize>) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// Sends `message` when `key` is pressed in the focused input.
    pub(super) fn on_key(mut self, key: Named, message: Message) -> Self {
        self.keys.push((key, message));
        self
    }

    /// Sends `message` when the input gets the focus.
    pub(super) fn on_focus(mut self, message: Option<Message>) -> Self {
        self.on_focus = message;
        self
    }

    /// Sends `message` on a press away from the input and its open list.
    pub(super) fn on_close(mut self, message: Option<Message>) -> Self {
        self.on_close = message;
        self
    }
}

impl<'a, Message: Clone + 'a> From<SuggestionInput<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: SuggestionInput<'a, Message>) -> Self {
        let mut keys = InputKeys::new(value.input).follow_text(value.text);
        for (key, message) in value.keys {
            keys = keys.on_key(key, message);
        }
        if let Some(message) = value.on_focus {
            keys = keys.on_focus(message);
        }

        let mut floating = FloatingList::new(keys).on_close(value.on_close);
        if !value.rows.is_empty() {
            let list = list_view(value.rows, value.highlighted, &value.palette);
            floating = floating
                .list(list, value.highlighted, size::ROW)
                .tallest(LIST_MAX_HEIGHT);
        }

        floating.into()
    }
}

/// The floating surface with one row per suggestion, in the look of a
/// menu, the highlighted one lit. Past [`LIST_ROWS`] rows it scrolls.
fn list_view<'a, Message: Clone + 'a>(
    rows: Vec<(Element<'a, Message>, Option<Message>)>,
    highlighted: Option<usize>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut list = column![].width(Length::Fill);
    for (index, (content, on_press)) in rows.into_iter().enumerate() {
        let row = button(
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .clip(true),
        )
        .width(Length::Fill)
        .height(size::ROW)
        .padding(Padding::from([0.0, ROW_PADDING_X]))
        .on_press_maybe(on_press)
        .style(menu::item_style(palette, highlighted == Some(index)));
        list = list.push(row);
    }

    let background = palette.surface_overlay();
    let shadow = palette.shadow_popover();

    container(
        scrollable(list)
            .width(Length::Fill)
            .direction(scrollbar::vertical())
            .style(scrollbar::style(palette)),
    )
    .width(Length::Fill)
    .max_height(LIST_MAX_HEIGHT)
    .padding(menu::MENU_PADDING)
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
