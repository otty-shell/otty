//! `DataTable`: a header row over scrolling rows of cells, for lists of
//! resources such as containers or images.

use iced::widget::text::Alignment as TextAlignment;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Font, Length, Padding, Point};

use super::ellipsis_text::EllipsisText;
use super::scrollbar;
use super::status_badge::{StatusBadge, StatusTone, TEXT_SIZE};
use super::table_body::{RightPress, TableBody};
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Horizontal padding of a cell (`.ot-table td { padding: 0 12px }`).
const CELL_PADDING_X: f32 = spacing::SPACE_4;
/// Padding of the empty-table message (`padding: 16px 12px`).
const EMPTY_PADDING: Padding = Padding {
    top: spacing::SPACE_5,
    right: spacing::SPACE_4,
    bottom: spacing::SPACE_5,
    left: spacing::SPACE_4,
};

/// Message builder for a row press: row index.
type RowPress<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;

/// Horizontal alignment of a [`TableColumn`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlign {
    /// Left: names and other text.
    Start,
    /// Right: numbers, so their digits line up.
    End,
}

impl ColumnAlign {
    /// Returns the matching alignment of text inside its width.
    fn text_alignment(self) -> TextAlignment {
        match self {
            Self::Start => TextAlignment::Left,
            Self::End => TextAlignment::Right,
        }
    }

    /// Returns the matching alignment of a widget inside its cell.
    fn alignment(self) -> Alignment {
        match self {
            Self::Start => Alignment::Start,
            Self::End => Alignment::End,
        }
    }
}

/// Column of a [`DataTable`]: a title, a width share and an alignment.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::data_table::{ColumnAlign, TableColumn};
///
/// let name = TableColumn::new("Name", 3);
/// let size = TableColumn::new("Size", 1).align(ColumnAlign::End);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumn {
    title: String,
    share: u16,
    align: ColumnAlign,
}

impl TableColumn {
    /// Creates a left-aligned column. `share` is its part of the table
    /// width: a column with share 2 is twice as wide as one with share 1.
    pub fn new(title: impl Into<String>, share: u16) -> Self {
        Self {
            title: title.into(),
            share,
            align: ColumnAlign::Start,
        }
    }

    /// Sets the alignment of the title and the cells.
    pub fn align(mut self, align: ColumnAlign) -> Self {
        self.align = align;
        self
    }
}

/// One cell of a [`DataTable`] row. Long text ends with "…".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableCell {
    /// Body text in the interface font.
    Plain(String),
    /// Body text in the mono font: names, IDs.
    Mono(String),
    /// Muted text in the mono font: images, ports, drivers.
    MutedMono(String),
    /// Muted text in the interface font: ages.
    Muted(String),
    /// Number in the mono font, so digits line up; always right-aligned.
    Number(String),
    /// [`StatusBadge`]: dot and state word in a tone.
    Badge(String, StatusTone),
}

impl TableCell {
    /// Creates a plain cell.
    pub fn plain(value: impl Into<String>) -> Self {
        Self::Plain(value.into())
    }

    /// Creates a mono cell.
    pub fn mono(value: impl Into<String>) -> Self {
        Self::Mono(value.into())
    }

    /// Creates a muted mono cell.
    pub fn muted_mono(value: impl Into<String>) -> Self {
        Self::MutedMono(value.into())
    }

    /// Creates a muted cell.
    pub fn muted(value: impl Into<String>) -> Self {
        Self::Muted(value.into())
    }

    /// Creates a number cell.
    pub fn number(value: impl Into<String>) -> Self {
        Self::Number(value.into())
    }

    /// Creates a status badge cell.
    pub fn badge(label: impl Into<String>, tone: StatusTone) -> Self {
        Self::Badge(label.into(), tone)
    }
}

/// Table with a fixed header and rows that scroll under it. One row can be
/// selected. When there are no rows, the empty message shows instead.
///
/// The table fills the space it gets. Each row is a list of cells in column
/// order; missing cells stay blank and extra cells are not shown.
///
/// A right click on a row sends its index and the pointer position in the
/// coordinates of the table's parent: window coordinates, unless the table
/// itself sits inside another `scrollable`.
///
/// # Example
///
/// ```
/// use iced::{Element, Point};
/// use otty_ui_kit::components::data_table::{
///     ColumnAlign, DataTable, TableCell, TableColumn,
/// };
/// use otty_ui_kit::components::status_badge::StatusTone;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Select(usize),
///     OpenMenu(usize, Point),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let columns = vec![
///     TableColumn::new("Name", 3),
///     TableColumn::new("Status", 2),
///     TableColumn::new("Age", 1).align(ColumnAlign::End),
/// ];
/// let table: Element<'_, Message> = DataTable::new(columns, &palette)
///     .row(vec![
///         TableCell::mono("redis"),
///         TableCell::badge("Running", StatusTone::Ok),
///         TableCell::muted("3 d"),
///     ])
///     .selected(Some(0))
///     .empty_message("No containers")
///     .on_row_press(Message::Select)
///     .on_row_right_press(Message::OpenMenu)
///     .into();
/// ```
pub struct DataTable<'a, Message> {
    columns: Vec<TableColumn>,
    rows: Vec<Vec<TableCell>>,
    palette: UiPalette,
    selected: Option<usize>,
    empty_message: Option<String>,
    on_row_press: Option<RowPress<'a, Message>>,
    on_row_right_press: Option<RightPress<'a, Message>>,
}

impl<'a, Message> DataTable<'a, Message> {
    /// Creates a table with `columns` and no rows.
    pub fn new(columns: Vec<TableColumn>, palette: &UiPalette) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            palette: palette.clone(),
            selected: None,
            empty_message: None,
            on_row_press: None,
            on_row_right_press: None,
        }
    }

    /// Adds a row of `cells` after the others.
    pub fn row(mut self, cells: Vec<TableCell>) -> Self {
        self.rows.push(cells);
        self
    }

    /// Marks the row at `index` as the selected one.
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Sets the line shown under the header when there are no rows, such as
    /// "No containers" or "Nothing matches “web”".
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = Some(message.into());
        self
    }

    /// Sets the message sent when a row is pressed, built from its index.
    /// Without it rows show no hover.
    pub fn on_row_press(
        mut self,
        on_press: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        self.on_row_press = Some(Box::new(on_press));
        self
    }

    /// Sets the message sent when a row is right-pressed, built from its
    /// index and the pointer position, to open a context menu there.
    pub fn on_row_right_press(
        mut self,
        on_right_press: impl Fn(usize, Point) -> Message + 'a,
    ) -> Self {
        self.on_row_right_press = Some(Box::new(on_right_press));
        self
    }
}

impl<'a, Message: Clone + 'a> From<DataTable<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: DataTable<'a, Message>) -> Self {
        let DataTable {
            columns,
            rows,
            palette,
            selected,
            empty_message,
            on_row_press,
            on_row_right_press,
        } = value;
        let canvas = palette.surface_canvas();

        let header = header_view(&columns, &palette);

        let body: Element<'a, Message> = if rows.is_empty() {
            empty_view(empty_message, &palette)
        } else {
            let rows = rows
                .into_iter()
                .enumerate()
                .map(|(index, cells)| {
                    let on_press =
                        on_row_press.as_ref().map(|on_press| on_press(index));
                    row_view(
                        cells,
                        &columns,
                        selected == Some(index),
                        on_press,
                        &palette,
                    )
                })
                .collect::<Vec<_>>();

            scrollable(TableBody::new(column(rows), on_row_right_press))
                .direction(scrollbar::vertical())
                .style(scrollbar::style(&palette))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        container(column![header, body])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(canvas.into()),
                ..container::Style::default()
            })
            .into()
    }
}

/// Draws the header: muted medium titles over a `line` rule.
fn header_view<'a, Message: 'a>(
    columns: &[TableColumn],
    palette: &UiPalette,
) -> Element<'a, Message> {
    let muted = palette.text_muted();

    let titles = columns.iter().map(|column| {
        let title = EllipsisText::new(
            column.title.as_str(),
            typography::SMALL.size(),
            fonts::UI_MEDIUM,
            muted,
        )
        .align_x(column.align.text_alignment());

        cell_frame(title.into(), column.share, column.align)
    });

    column![row(titles).height(size::TABLE_HEADER), rule(palette.line()),]
        .into()
}

/// Draws one body row: its cells on a button, over a `surface-raised` rule.
/// The row shows `surface-header` on hover and `surface-selected` with
/// `text-strong` text when selected.
fn row_view<'a, Message: Clone + 'a>(
    cells: Vec<TableCell>,
    columns: &[TableColumn],
    is_selected: bool,
    on_press: Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let hover = palette.surface_header();
    let selected = palette.surface_selected();

    let mut cells = cells.into_iter();
    let content = row(columns.iter().map(|column| {
        let (content, align) = match cells.next() {
            Some(cell) => cell_view(cell, column.align, is_selected, palette),
            None => (Space::new().into(), column.align),
        };

        cell_frame(content, column.share, align)
    }))
    .height(Length::Fill);

    let control = button(content)
        .width(Length::Fill)
        .height(size::TABLE_ROW)
        .padding(0)
        .on_press_maybe(on_press)
        .style(move |_, status| {
            let is_hovered = matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            );

            button::Style {
                background: if is_selected {
                    Some(selected.into())
                } else {
                    is_hovered.then(|| hover.into())
                },
                ..button::Style::default()
            }
        });

    column![control, rule(palette.surface_raised())].into()
}

/// Draws the content of `cell` and returns it with its alignment: the
/// column's, except numbers, which are always right-aligned. On a selected
/// row every text cell is `text-strong`; badges keep their tone.
fn cell_view<'a, Message: 'a>(
    cell: TableCell,
    align: ColumnAlign,
    is_selected: bool,
    palette: &UiPalette,
) -> (Element<'a, Message>, ColumnAlign) {
    let body = if is_selected {
        palette.text_strong()
    } else {
        palette.text_body()
    };
    let muted = if is_selected {
        palette.text_strong()
    } else {
        palette.text_muted()
    };

    let (value, font, color, align): (String, Font, Color, ColumnAlign) =
        match cell {
            TableCell::Plain(value) => (value, fonts::UI, body, align),
            TableCell::Mono(value) => (value, fonts::MONO, body, align),
            TableCell::MutedMono(value) => (value, fonts::MONO, muted, align),
            TableCell::Muted(value) => (value, fonts::UI, muted, align),
            TableCell::Number(value) => {
                (value, fonts::MONO, body, ColumnAlign::End)
            },
            TableCell::Badge(label, tone) => {
                return (StatusBadge::new(label, tone, palette).into(), align);
            },
        };

    let content = EllipsisText::new(value, TEXT_SIZE, font, color)
        .align_x(align.text_alignment());

    (content.into(), align)
}

/// Places cell or title `content` in its column: `share` of the width,
/// 12px side padding, centered vertically, cut at the edge.
fn cell_frame<'a, Message: 'a>(
    content: Element<'a, Message>,
    share: u16,
    align: ColumnAlign,
) -> Element<'a, Message> {
    container(content)
        .width(Length::FillPortion(share))
        .height(Length::Fill)
        .padding(Padding::from([0.0, CELL_PADDING_X]))
        .align_x(align.alignment())
        .align_y(Alignment::Center)
        .clip(true)
        .into()
}

/// Draws the muted line shown when the table has no rows (`.ot-hint`).
fn empty_view<'a, Message: 'a>(
    message: Option<String>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let Some(message) = message else {
        return Space::new().into();
    };
    let style = typography::SMALL;

    container(
        text(message)
            .size(style.size())
            .line_height(iced::Pixels(style.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted()),
    )
    .width(Length::Fill)
    .padding(EMPTY_PADDING)
    .into()
}

/// Draws a full-width 1px rule in `color`.
fn rule<'a, Message: 'a>(color: Color) -> Element<'a, Message> {
    container(Space::new())
        .width(Length::Fill)
        .height(size::LINE)
        .style(move |_| container::Style {
            background: Some(color.into()),
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use iced::mouse::{self, ScrollDelta};
    use iced::{Event, Point};
    use iced_test::simulator;

    use super::*;

    /// Width of the table in the tests.
    const WIDTH: f32 = 600.0;
    /// Top of the first body row: header and its rule.
    const BODY_TOP: f32 = size::TABLE_HEADER + size::LINE;
    /// Distance from one row's top to the next: row and its rule.
    const ROW_STRIDE: f32 = size::TABLE_ROW + size::LINE;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Pressed(usize),
        RightPressed(usize, Point),
    }

    const NAMES: [&str; 3] = ["otty-site-web-1", "postgres-dev-db-1", "redis"];

    fn columns() -> Vec<TableColumn> {
        vec![
            TableColumn::new("Name", 3),
            TableColumn::new("Status", 2),
            TableColumn::new("Size", 1).align(ColumnAlign::End),
            TableColumn::new("Age", 1),
        ]
    }

    /// Table of `count` rows named `row-0`, `row-1`, … or from [`NAMES`].
    fn table(count: usize, height: f32) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();
        let mut table = DataTable::new(columns(), &palette)
            .selected(Some(0))
            .empty_message("No containers")
            .on_row_press(Message::Pressed)
            .on_row_right_press(Message::RightPressed);
        for index in 0..count {
            let name = NAMES.get(index).map_or_else(
                || format!("row-{index}"),
                |name| (*name).to_owned(),
            );
            table = table.row(vec![
                TableCell::mono(name),
                TableCell::badge("Running", StatusTone::Ok),
                TableCell::number("192 MB"),
                TableCell::muted("2 h"),
            ]);
        }

        container(table).width(WIDTH).height(height).into()
    }

    fn right_click() -> [Event; 2] {
        [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
        ]
    }

    #[test]
    fn given_rows_when_a_cell_is_pressed_then_its_row_index_is_sent() {
        let mut ui = simulator(table(3, 300.0));

        ui.click("redis").expect("cell should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Pressed(2)]);
    }

    #[test]
    fn given_rows_when_right_pressed_then_index_and_pointer_are_sent() {
        let mut ui = simulator(table(3, 300.0));
        let pointer = Point::new(100.0, BODY_TOP + ROW_STRIDE + 10.0);

        ui.point_at(pointer);
        let _ = ui.simulate(right_click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::RightPressed(1, pointer)]);
    }

    #[test]
    fn given_scrolled_rows_when_right_pressed_then_pointer_is_not_moved_by_scroll()
     {
        let mut ui = simulator(table(20, BODY_TOP + 3.0 * ROW_STRIDE));
        let pointer = Point::new(100.0, BODY_TOP + 10.0);

        ui.point_at(pointer);
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: ScrollDelta::Pixels {
                x: 0.0,
                y: -2.0 * ROW_STRIDE,
            },
        })]);
        let _ = ui.simulate(right_click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::RightPressed(2, pointer)]);
    }

    #[test]
    fn given_header_when_right_pressed_then_nothing_is_sent() {
        let mut ui = simulator(table(3, 300.0));

        ui.point_at(Point::new(100.0, size::TABLE_HEADER / 2.0));
        let _ = ui.simulate(right_click());

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_space_under_rows_when_right_pressed_then_nothing_is_sent() {
        let mut ui = simulator(table(1, 300.0));

        ui.point_at(Point::new(100.0, BODY_TOP + 2.0 * ROW_STRIDE));
        let _ = ui.simulate(right_click());

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_rows_when_rendered_then_titles_and_cells_show() {
        let mut ui = simulator(table(3, 300.0));

        for label in ["Name", "Status", "Size", "Age", "redis", "Running"] {
            assert!(ui.find(label).is_ok(), "{label:?} should be shown");
        }
        assert!(ui.find("No containers").is_err());
    }

    #[test]
    fn given_no_rows_when_rendered_then_empty_message_shows_under_header() {
        let mut ui = simulator(table(0, 300.0));

        assert!(ui.find("Name").is_ok());
        assert!(ui.find("No containers").is_ok());
    }

    #[test]
    fn given_row_with_missing_cells_when_rendered_then_the_rest_is_blank() {
        let palette = UiPalette::otty_light();
        let table: Element<'_, Message> = DataTable::new(columns(), &palette)
            .row(vec![TableCell::plain("hello"), TableCell::muted_mono("—")])
            .row(vec![
                TableCell::plain("a"),
                TableCell::plain("b"),
                TableCell::plain("c"),
                TableCell::plain("d"),
                TableCell::plain("extra"),
            ])
            .into();
        let mut ui = simulator(container(table).width(WIDTH).height(200));

        assert!(ui.find("hello").is_ok());
        assert!(ui.find("—").is_ok());
        assert!(ui.find("d").is_ok());
        assert!(ui.find("extra").is_err());
    }

    #[test]
    fn given_table_without_handlers_when_rows_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let table: Element<'_, Message> = DataTable::new(columns(), &palette)
            .row(vec![TableCell::plain("redis")])
            .into();
        let mut ui = simulator(container(table).width(WIDTH).height(200));

        ui.click("redis").expect("cell should be visible");
        let _ = ui.simulate(right_click());

        assert_eq!(ui.into_messages().count(), 0);
    }
}
