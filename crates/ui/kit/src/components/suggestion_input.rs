//! A text input with suggestions floating under it: the one way a field of
//! the kit suggests, whatever it suggests. [`TagSearchField`] suggests keys,
//! values and names; [`TextField`] suggests what the app gives it, such as
//! folders. Each gives only its rows, the row that is lit and what a pick
//! and a move of the light send.
//!
//! The piece holds the rest, the same in every field: the keys of the list
//! (see [`SuggestionInput::on_highlight`]), bound on the focused input so
//! they do not reach it; the list that opens while there are rows; a press
//! on a row; a press away; the lit row kept in view; the cursor moved to
//! the end when the app replaces the text (a pick); and the look of the
//! list and of its rows, as a menu.
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
    rows: Vec<Element<'a, Message>>,
    highlighted: Option<usize>,
    on_pick: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_highlight: Option<Box<dyn Fn(Option<usize>) -> Message + 'a>>,
    keys: Vec<(Named, Message)>,
    on_focus: Option<Message>,
    on_close: Option<Message>,
    on_escape: Option<Message>,
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
            on_pick: None,
            on_highlight: None,
            keys: Vec::new(),
            on_focus: None,
            on_close: None,
            on_escape: None,
        }
    }

    /// Adds a row showing `content`; a press on it is a pick of it.
    pub(super) fn row(
        mut self,
        content: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.rows.push(content.into());
        self
    }

    /// Lights the row at `highlighted` and keeps it in view. The owner
    /// keeps the lit row: it clears it when the text changes, on a pick and
    /// when the list closes.
    pub(super) fn highlighted(mut self, highlighted: Option<usize>) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// Sends `on_pick` with the place of a row in the list when the row is
    /// pressed, or when Enter or Tab is pressed while it is lit. Without it
    /// the rows can not be picked.
    pub(super) fn on_pick(
        mut self,
        on_pick: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    ) -> Self {
        self.on_pick = on_pick;
        self
    }

    /// Lets ↓ and ↑ move the light while the list shows, by the rules of
    /// every list under a field: no row is lit until ↓; ↓ lights the first
    /// row, then the next one, and stops at the last; ↑ lights the row
    /// above, and from the first row leaves the list (`None`). With a row
    /// lit, Enter and Tab pick it (see [`Self::on_pick`]); with none, they
    /// are left to the input and the app (Enter submits, Tab moves the
    /// focus). `on_highlight` sends the row to light next.
    pub(super) fn on_highlight(
        mut self,
        on_highlight: Option<Box<dyn Fn(Option<usize>) -> Message + 'a>>,
    ) -> Self {
        self.on_highlight = on_highlight;
        self
    }

    /// Sends `message` when `key` is pressed in the focused input, for a
    /// key of the field's own; the keys of the list come first.
    pub(super) fn on_key(mut self, key: Named, message: Message) -> Self {
        self.keys.push((key, message));
        self
    }

    /// Sends `message` on Esc while the list is closed; while it shows, Esc
    /// closes it (see [`Self::on_close`]).
    pub(super) fn on_escape(mut self, message: Option<Message>) -> Self {
        self.on_escape = message;
        self
    }

    /// Sends `message` when the input gets the focus.
    pub(super) fn on_focus(mut self, message: Option<Message>) -> Self {
        self.on_focus = message;
        self
    }

    /// Sends `message` on a press away from the input and its open list,
    /// and on Esc while the list shows.
    pub(super) fn on_close(mut self, message: Option<Message>) -> Self {
        self.on_close = message;
        self
    }

    /// The keys of the list for the row lit now, `highlighted`, which is in
    /// the list: ↓ and ↑ move the light, Enter and Tab pick the lit row,
    /// and Esc closes an open list before it is the field's own.
    fn list_keys(&self, highlighted: Option<usize>) -> Vec<(Named, Message)> {
        let rows = self.rows.len();
        let is_open = rows > 0;
        let mut keys = Vec::new();

        if is_open && let Some(on_highlight) = &self.on_highlight {
            keys.push((
                Named::ArrowDown,
                on_highlight(Some(row_below(highlighted, rows))),
            ));
            if let Some(row) = highlighted {
                keys.push((Named::ArrowUp, on_highlight(row_above(row))));
            }
        }
        if let (Some(row), Some(on_pick)) = (highlighted, &self.on_pick) {
            keys.push((Named::Enter, on_pick(row)));
            keys.push((Named::Tab, on_pick(row)));
        }

        let escape = if is_open {
            self.on_close.clone().or_else(|| self.on_escape.clone())
        } else {
            self.on_escape.clone()
        };
        if let Some(message) = escape {
            keys.push((Named::Escape, message));
        }

        keys
    }
}

impl<'a, Message: Clone + 'a> From<SuggestionInput<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: SuggestionInput<'a, Message>) -> Self {
        let highlighted =
            value.highlighted.filter(|row| *row < value.rows.len());
        let list_keys = value.list_keys(highlighted);
        let rows: Vec<(Element<'a, Message>, Option<Message>)> = value
            .rows
            .into_iter()
            .enumerate()
            .map(|(index, content)| {
                (
                    content,
                    value.on_pick.as_ref().map(|on_pick| on_pick(index)),
                )
            })
            .collect();

        let mut keys = InputKeys::new(value.input).follow_text(value.text);
        for (key, message) in list_keys.into_iter().chain(value.keys) {
            keys = keys.on_key(key, message);
        }
        if let Some(message) = value.on_focus {
            keys = keys.on_focus(message);
        }

        let mut floating = FloatingList::new(keys).on_close(value.on_close);
        if !rows.is_empty() {
            let list = list_view(rows, highlighted, &value.palette);
            floating = floating
                .list(list, highlighted, size::ROW)
                .tallest(LIST_MAX_HEIGHT);
        }

        floating.into()
    }
}

/// The row ↓ lights in a list of `rows` rows: the first one while none is
/// lit, else the next one; the last row keeps the light.
fn row_below(highlighted: Option<usize>, rows: usize) -> usize {
    match highlighted {
        Some(row) => (row + 1).min(rows.saturating_sub(1)),
        None => 0,
    }
}

/// The row ↑ lights from `row`: the one above; from the first row the light
/// leaves the list.
fn row_above(row: usize) -> Option<usize> {
    row.checked_sub(1)
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
