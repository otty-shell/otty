//! `DataTable`: a header row over scrolling rows of cells, for lists of
//! resources such as containers or images.

use std::cell::Cell;
use std::rc::Rc;

use iced::widget::text::Alignment as TextAlignment;
use iced::widget::{
    Space, button, column, container, responsive, row, scrollable, svg, text,
    tooltip,
};
use iced::{Alignment, Color, Element, Font, Length, Padding, Point};

use super::ellipsis_text::EllipsisText;
use super::status_badge::{StatusBadge, StatusTone, TEXT_SIZE};
use super::table_body::{MenuPress, RightPress, TableBody};
use super::table_sideways::{SidewaysOffset, SidewaysScroll};
use super::{glyph, icon_button, scrollbar};
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
/// Width of the "⋯" button column: the small button and 6px on each side.
const MENU_COLUMN_WIDTH: f32 = size::CONTROL_SM + 2.0 * spacing::SPACE_2;
/// Edge of the "⋯" dots inside their button, as in a small `IconButton`.
const MENU_ICON: f32 = 16.0;
/// Edge of the sort arrow. 12 keeps it on whole pixels at common display
/// scales and inside the 12px title line.
const SORT_ARROW: f32 = 12.0;

/// Message builder for a row press: row index.
type RowPress<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;
/// Message builder for a header press: column index.
type SortPress<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;
/// Message builder for a link press: row index, column index and the
/// position of the link among the links of its cell.
type LinkPress<'a, Message> = Box<dyn Fn(usize, usize, usize) -> Message + 'a>;

/// Direction of the sorted column of a [`DataTable`], shown as an arrow in
/// its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Up arrow: A to Z, smallest or oldest first.
    Ascending,
    /// Down arrow: Z to A, largest or newest first.
    Descending,
}

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
/// let name = TableColumn::new("Name", 3).sortable(true);
/// let size = TableColumn::new("Size", 1).align(ColumnAlign::End);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumn {
    title: String,
    share: u16,
    align: ColumnAlign,
    is_sortable: bool,
    min_width: u16,
}

impl TableColumn {
    /// The text of the column header.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Creates a left-aligned column that is not sortable and has no minimum
    /// width. `share` is its part of the table width: a column with share 2
    /// is twice as wide as one with share 1.
    pub fn new(title: impl Into<String>, share: u16) -> Self {
        Self {
            title: title.into(),
            share,
            align: ColumnAlign::Start,
            is_sortable: false,
            min_width: 0,
        }
    }

    /// Sets the width in pixels the column never goes below, side padding
    /// included. Columns with no minimum give way first when the table is
    /// narrow. When even the minimums do not fit, the table scrolls sideways
    /// with its header, and a column with no minimum is not shown, so give
    /// every column a minimum if the table should scroll instead of hiding
    /// one.
    pub fn min_width(mut self, min_width: u16) -> Self {
        self.min_width = min_width;
        self
    }

    /// Sets the alignment of the title and the cells.
    pub fn align(mut self, align: ColumnAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets whether pressing the title sends the table's
    /// [`DataTable::on_sort`] message for this column.
    pub fn sortable(mut self, is_sortable: bool) -> Self {
        self.is_sortable = is_sortable;
        self
    }
}

/// One piece of a [`TableCell::Parts`] cell: text or a link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellPart {
    /// Muted mono text between or beside links.
    Text(String),
    /// A link in the accent color; pressing it sends the table's
    /// [`DataTable::on_link_press`] message with the link's position among
    /// the links of the cell.
    Link(String),
}

impl CellPart {
    /// Creates a text part.
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// Creates a link part.
    pub fn link(label: impl Into<String>) -> Self {
        Self::Link(label.into())
    }
}

/// One cell of a [`DataTable`] row. Long text ends with "…".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableCell {
    /// Several [`CellPart`]s on one line in the mono font: links with text
    /// between them, such as published ports. A part that does not fit is
    /// cut at the cell edge.
    Parts(Vec<CellPart>),
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
    /// Text in the accent color, underlined under the pointer. Pressing it
    /// sends the table's [`DataTable::on_link_press`] message instead of
    /// pressing the row.
    Link(String),
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

    /// Creates a link cell.
    pub fn link(label: impl Into<String>) -> Self {
        Self::Link(label.into())
    }

    /// Creates a cell of text and links.
    pub fn parts(parts: Vec<CellPart>) -> Self {
        Self::Parts(parts)
    }
}

/// Table with a fixed header and rows that scroll under it. One row can be
/// selected. When there are no rows, the empty message shows instead.
///
/// The table fills the space it gets. Columns share the width by their
/// shares but never go below their [`TableColumn::min_width`]; when the
/// minimums do not fit, the header and the rows scroll sideways together
/// with a thin scrollbar. Each row is a list of cells in column
/// order; missing cells stay blank and extra cells are not shown.
///
/// A right click on a row sends its index and the pointer position in the
/// coordinates of the table's parent: window coordinates, unless the table
/// itself sits inside another `scrollable`.
///
/// The table does not sort rows: the caller passes them in order and marks
/// the sorted column with [`DataTable::sorted_by`].
///
/// # Example
///
/// ```
/// use iced::{Element, Point};
/// use otty_ui_kit::components::data_table::{
///     ColumnAlign, DataTable, SortDirection, TableCell, TableColumn,
/// };
/// use otty_ui_kit::components::status_badge::StatusTone;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Select(usize),
///     OpenMenu(usize, Point),
///     Sort(usize),
///     OpenLink(usize, usize, usize),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let columns = vec![
///     TableColumn::new("Name", 3).sortable(true),
///     TableColumn::new("Status", 2).sortable(true),
///     TableColumn::new("Compose", 2),
///     TableColumn::new("Age", 1).align(ColumnAlign::End),
/// ];
/// let table: Element<'_, Message> = DataTable::new(columns, &palette)
///     .row(vec![
///         TableCell::mono("redis"),
///         TableCell::badge("Running", StatusTone::Ok),
///         TableCell::link("otty-site"),
///         TableCell::muted("3 d"),
///     ])
///     .selected(Some(0))
///     .sorted_by(0, SortDirection::Ascending)
///     .empty_message("No containers")
///     .on_row_press(Message::Select)
///     .on_row_right_press(Message::OpenMenu)
///     .on_sort(Message::Sort)
///     .on_link_press(Message::OpenLink)
///     .on_row_menu("More actions", Message::OpenMenu)
///     .into();
/// ```
pub struct DataTable<'a, Message> {
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    palette: UiPalette,
    selected: Option<usize>,
    sort: Option<(usize, SortDirection)>,
    empty_message: Option<String>,
    on_row_press: Option<RowPress<'a, Message>>,
    on_row_right_press: Option<RightPress<'a, Message>>,
    on_sort: Option<SortPress<'a, Message>>,
    on_link_press: Option<LinkPress<'a, Message>>,
    row_menu: Option<RowMenu<'a, Message>>,
}

impl<'a, Message> DataTable<'a, Message> {
    /// Creates a table with `columns` and no rows.
    pub fn new(columns: Vec<TableColumn>, palette: &UiPalette) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            palette: palette.clone(),
            selected: None,
            sort: None,
            empty_message: None,
            on_row_press: None,
            on_row_right_press: None,
            on_sort: None,
            on_link_press: None,
            row_menu: None,
        }
    }

    /// Adds a row of `cells` after the others. With
    /// [`DataTable::on_row_menu`] it ends with a "⋯" button.
    pub fn row(mut self, cells: Vec<TableCell>) -> Self {
        self.rows.push(TableRow {
            cells,
            has_menu: true,
        });
        self
    }

    /// Adds a row of `cells` without the "⋯" button, for a resource that
    /// has no actions.
    pub fn row_without_menu(mut self, cells: Vec<TableCell>) -> Self {
        self.rows.push(TableRow {
            cells,
            has_menu: false,
        });
        self
    }

    /// Shows an arrow in the header of the column at `column`: up for
    /// [`SortDirection::Ascending`], down for [`SortDirection::Descending`].
    pub fn sorted_by(
        mut self,
        column: usize,
        direction: SortDirection,
    ) -> Self {
        self.sort = Some((column, direction));
        self
    }

    /// Sets the message sent when the title of a sortable column is
    /// pressed, built from the column index. Without it titles are not
    /// pressable.
    pub fn on_sort(mut self, on_sort: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_sort = Some(Box::new(on_sort));
        self
    }

    /// Sets the message sent when a [`TableCell::Link`] or a link of a
    /// [`TableCell::Parts`] is pressed, built from its row index, column
    /// index and position among the links of the cell (0 for a
    /// [`TableCell::Link`]). The row is not pressed. Without it links are
    /// drawn but not pressable.
    pub fn on_link_press(
        mut self,
        on_link_press: impl Fn(usize, usize, usize) -> Message + 'a,
    ) -> Self {
        self.on_link_press = Some(Box::new(on_link_press));
        self
    }

    /// Adds a last column with a "⋯" button on each row added with
    /// [`DataTable::row`]. `label` is the button's tooltip. Pressing it
    /// sends the row index and the bottom-left corner of the button, in the
    /// same coordinates as [`DataTable::on_row_right_press`], to open the
    /// row's menu there. The row is not pressed.
    pub fn on_row_menu(
        mut self,
        label: impl Into<String>,
        on_press: impl Fn(usize, Point) -> Message + 'a,
    ) -> Self {
        self.row_menu = Some(RowMenu {
            label: label.into(),
            on_press: Rc::new(on_press),
        });
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
        self.on_row_right_press = Some(Rc::new(on_right_press));
        self
    }
}

impl<'a, Message: Clone + 'a> From<DataTable<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: DataTable<'a, Message>) -> Self {
        // The columns depend on the width the table gets, which is known only
        // while laying out, so the table is built again for each width.
        let sideways: SidewaysOffset = Rc::new(Cell::new(0.0));

        responsive(move |size| table_view(&value, size.width, &sideways)).into()
    }
}

/// One body row of a [`DataTable`]: its cells and whether it ends with a
/// "⋯" button.
#[derive(Clone)]
struct TableRow {
    cells: Vec<TableCell>,
    has_menu: bool,
}

/// The "⋯" button column of a [`DataTable`]: tooltip and message builder.
struct RowMenu<'a, Message> {
    label: String,
    on_press: MenuPress<'a, Message>,
}

/// Draws `table` for the `width` it gets. The columns share the width, but
/// never go below their minimums. When the minimums do not fit, the header
/// and the rows scroll sideways together, and `sideways` holds how far.
fn table_view<'a, Message: Clone + 'a>(
    table: &DataTable<'a, Message>,
    width: f32,
    sideways: &SidewaysOffset,
) -> Element<'a, Message> {
    let palette = &table.palette;
    let canvas = palette.surface_canvas();
    let has_menu_column = table.row_menu.is_some();
    let menu_width = if has_menu_column {
        MENU_COLUMN_WIDTH
    } else {
        0.0
    };

    let available = (width - menu_width).max(0.0);
    let minimum = minimum_width(&table.columns);
    let is_sideways = minimum > available;
    let widths = column_widths(&table.columns, available);

    let header = header_view(
        &table.columns,
        &widths,
        table.sort,
        table.on_sort.as_deref(),
        has_menu_column,
        palette,
    );
    let body: Element<'a, Message> = if table.rows.is_empty() {
        empty_view(table.empty_message.clone(), palette)
    } else {
        rows_view(table, &widths, is_sideways.then(|| sideways.clone()))
    };

    if !is_sideways {
        sideways.set(0.0);
    }
    let content_width = if is_sideways {
        Length::Fixed(minimum + menu_width)
    } else {
        Length::Fill
    };
    let content = column![header, body]
        .width(content_width)
        .height(Length::Fill);
    let content: Element<'a, Message> = if is_sideways {
        SidewaysScroll::new(
            scrollable(content)
                .direction(scrollbar::horizontal())
                .style(scrollbar::style(palette))
                .width(Length::Fill)
                .height(Length::Fill),
            sideways.clone(),
        )
        .into()
    } else {
        content.into()
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(canvas.into()),
            ..container::Style::default()
        })
        .into()
}

/// Draws the body rows of `table` with the column `widths`, scrolling
/// vertically. `sideways` is given when the table also scrolls sideways.
fn rows_view<'a, Message: Clone + 'a>(
    table: &DataTable<'a, Message>,
    widths: &[f32],
    sideways: Option<SidewaysOffset>,
) -> Element<'a, Message> {
    let palette = &table.palette;
    let menu_rows: Vec<bool> =
        table.rows.iter().map(|row| row.has_menu).collect();
    let menu_label = table.row_menu.as_ref().map(|menu| menu.label.as_str());

    let rows = table
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let on_press =
                table.on_row_press.as_ref().map(|on_press| on_press(index));
            let on_link = |column: usize, link: usize| {
                table
                    .on_link_press
                    .as_ref()
                    .map(|on_link_press| on_link_press(index, column, link))
            };
            let menu_cell = menu_label.map(|label| {
                if row.has_menu {
                    menu_button(label, palette)
                } else {
                    Space::new().into()
                }
            });

            row_view(
                row.cells.clone(),
                &table.columns,
                widths,
                table.selected == Some(index),
                on_press,
                &on_link,
                menu_cell,
                palette,
            )
        })
        .collect::<Vec<_>>();

    let mut body =
        TableBody::new(column(rows), table.on_row_right_press.clone());
    if let Some(menu) = &table.row_menu {
        body = body.row_menus(menu_rows, menu.on_press.clone());
    }
    if let Some(sideways) = sideways {
        body = body.sideways(sideways);
    }

    scrollable(body)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Draws the header: medium titles over a `line` rule, and a blank title
/// over the "⋯" column when `has_menu_column`. `widths` are the column
/// widths.
fn header_view<'a, Message: Clone + 'a>(
    columns: &[TableColumn],
    widths: &[f32],
    sort: Option<(usize, SortDirection)>,
    on_sort: Option<&(dyn Fn(usize) -> Message + 'a)>,
    has_menu_column: bool,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut titles = row(columns.iter().zip(widths).enumerate().map(
        |(index, (column, width))| {
            let direction = sort
                .filter(|(sorted, _)| *sorted == index)
                .map(|(_, direction)| direction);
            let on_press = on_sort
                .filter(|_| column.is_sortable)
                .map(|on_sort| on_sort(index));

            header_cell(column, *width, direction, on_press, palette)
        },
    ));
    if has_menu_column {
        titles = titles.push(Space::new().width(MENU_COLUMN_WIDTH));
    }

    column![titles.height(size::TABLE_HEADER), rule(palette.line())].into()
}

/// Draws the title of `column`, `width` wide: muted, or `text-body` with an
/// arrow in `direction` when it is the sorted column. The arrow sits after
/// the title, or before it in a right-aligned column so the title still
/// lines up with the cells. With `on_press` the title is a button that shows
/// `surface-header` on hover.
fn header_cell<'a, Message: Clone + 'a>(
    column: &TableColumn,
    width: f32,
    direction: Option<SortDirection>,
    on_press: Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let color = match direction {
        Some(_) => sort_arrow_color(palette),
        None => palette.text_muted(),
    };
    let hover = palette.surface_header();

    let title = EllipsisText::new(
        column.title.as_str(),
        typography::SMALL.size(),
        fonts::UI_MEDIUM,
        color,
    )
    .align_x(column.align.text_alignment());
    let content: Element<'a, Message> = match direction {
        None => title.into(),
        Some(direction) => {
            let arrow =
                svg(glyph::sort_arrow(direction == SortDirection::Ascending))
                    .width(SORT_ARROW)
                    .height(SORT_ARROW)
                    .style(move |_, _| svg::Style { color: Some(color) });
            // The title is as wide as its text so the arrow sits right
            // against it. In a left-aligned column the arrow comes second
            // and the title leaves room for it; in a right-aligned one the
            // arrow comes first and the title takes what is left.
            let parts = match column.align {
                ColumnAlign::Start => {
                    row![title.shrink(SORT_ARROW + spacing::SPACE_1), arrow]
                },
                ColumnAlign::End => row![arrow, title.shrink(0.0)],
            };

            parts
                .spacing(spacing::SPACE_1)
                .align_y(Alignment::Center)
                .into()
        },
    };
    let cell = cell_frame(content, width, column.align);

    let Some(message) = on_press else {
        return cell;
    };

    button(cell)
        .width(width)
        .height(Length::Fill)
        .padding(0)
        .on_press(message)
        .style(move |_, status| button::Style {
            background: matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            )
            .then(|| hover.into()),
            ..button::Style::default()
        })
        .into()
}

/// Draws one body row: its cells on a button, over a `surface-raised` rule.
/// The row shows `surface-header` on hover and `surface-selected` with
/// `text-strong` text when selected. `on_link` builds the message of a
/// link in the column at an index. `menu_cell` fills the "⋯" column when
/// the table has one. `widths` are the column widths.
///
/// `table_body::menu_button_bounds` finds the "⋯" button by this layout:
/// the row button, its row of cells, the last cell, its first child.
fn row_view<'a, Message: Clone + 'a>(
    cells: Vec<TableCell>,
    columns: &[TableColumn],
    widths: &[f32],
    is_selected: bool,
    on_press: Option<Message>,
    on_link: &dyn Fn(usize, usize) -> Option<Message>,
    menu_cell: Option<Element<'a, Message>>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let hover = palette.surface_header();
    let selected = palette.surface_selected();

    let mut cells = cells.into_iter();
    let mut content = row(columns.iter().zip(widths).enumerate().map(
        |(index, (column, width))| {
            let (content, align) = match cells.next() {
                Some(cell) => cell_view(
                    cell,
                    column.align,
                    is_selected,
                    &|link| on_link(index, link),
                    palette,
                ),
                None => (Space::new().into(), column.align),
            };

            cell_frame(content, *width, align)
        },
    ))
    .height(Length::Fill);
    if let Some(menu_cell) = menu_cell {
        content = content.push(
            container(menu_cell)
                .width(MENU_COLUMN_WIDTH)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        );
    }

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
/// row every text cell is `text-strong`; badges and links keep their color.
/// `on_link` gives the message of the link at a position among the links of
/// the cell.
fn cell_view<'a, Message: Clone + 'a>(
    cell: TableCell,
    align: ColumnAlign,
    is_selected: bool,
    on_link: &dyn Fn(usize) -> Option<Message>,
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
            TableCell::Link(label) => {
                return (link_view(label, align, on_link(0), palette), align);
            },
            TableCell::Parts(parts) => {
                return (parts_view(parts, muted, on_link, palette), align);
            },
        };

    let content = EllipsisText::new(value, TEXT_SIZE, font, color)
        .align_x(align.text_alignment());

    (content.into(), align)
}

/// Draws a link cell in the accent color. With `on_press` only the text is
/// pressable, underlined under the pointer; the press does not reach the
/// row. Without it the text is not pressable.
fn link_view<'a, Message: Clone + 'a>(
    label: String,
    align: ColumnAlign,
    on_press: Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let text =
        EllipsisText::new(label, TEXT_SIZE, fonts::UI, link_color(palette))
            .align_x(align.text_alignment());

    let Some(message) = on_press else {
        return text.into();
    };

    button(text.link())
        .padding(0)
        .on_press(message)
        .style(|_, _| button::Style::default())
        .into()
}

/// Draws the parts of a cell side by side in the mono font: text in `muted`,
/// links in the accent color. `on_link` gives the message of the link at a
/// position among the links; a link without one is drawn but not pressable.
/// Each link is its own button, so pressing one does not reach the row, and
/// the text around it presses the row.
fn parts_view<'a, Message: Clone + 'a>(
    parts: Vec<CellPart>,
    muted: Color,
    on_link: &dyn Fn(usize) -> Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut links = 0;
    let views = parts.into_iter().map(|part| -> Element<'a, Message> {
        match part {
            CellPart::Text(value) => {
                EllipsisText::new(value, TEXT_SIZE, fonts::MONO, muted)
                    .shrink(0.0)
                    .into()
            },
            CellPart::Link(label) => {
                let text = EllipsisText::new(
                    label,
                    TEXT_SIZE,
                    fonts::MONO,
                    link_color(palette),
                );
                let message = on_link(links);
                links += 1;

                match message {
                    Some(message) => button(text.link())
                        .padding(0)
                        .on_press(message)
                        .style(|_, _| button::Style::default())
                        .into(),
                    None => text.shrink(0.0).into(),
                }
            },
        }
    });

    row(views).into()
}

/// Draws the "⋯" button of a row: dots in `text-muted`, `text-strong` under
/// the pointer, and a tooltip `label`. [`TableBody`] handles its press.
fn menu_button<'a, Message: 'a>(
    label: &str,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let idle = palette.text_muted();
    let hover = palette.text_strong();

    let dots = svg(glyph::dots()).width(MENU_ICON).height(MENU_ICON).style(
        move |_, status| svg::Style {
            color: Some(match status {
                svg::Status::Hovered => hover,
                _ => idle,
            }),
        },
    );
    let button = container(dots).center(size::CONTROL_SM);

    tooltip(
        button,
        icon_button::tooltip_hint(label.to_owned(), palette),
        tooltip::Position::Bottom,
    )
    .gap(spacing::SPACE_1)
    .into()
}

/// Places cell or title `content` in its column: `width` wide, 12px side
/// padding, centered vertically, cut at the edge.
fn cell_frame<'a, Message: 'a>(
    content: Element<'a, Message>,
    width: f32,
    align: ColumnAlign,
) -> Element<'a, Message> {
    container(content)
        .width(width)
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

/// Returns the least width `columns` can have: the sum of their minimums.
fn minimum_width(columns: &[TableColumn]) -> f32 {
    columns
        .iter()
        .map(|column| f32::from(column.min_width))
        .sum()
}

/// Splits `available` width between `columns` by their shares. A column
/// whose share is below its minimum keeps the minimum, and the others share
/// what is left, which may push another column below its minimum in turn.
/// When even the minimums do not fit, every column is its minimum and a
/// column with no minimum is 0 wide.
fn column_widths(columns: &[TableColumn], available: f32) -> Vec<f32> {
    let mut is_minimum = vec![false; columns.len()];

    loop {
        let (free, shares) = free_width(columns, &is_minimum, available);

        let mut is_changed = false;
        for (index, column) in columns.iter().enumerate() {
            let width = share_width(column, free, shares);
            if !is_minimum[index] && width < f32::from(column.min_width) {
                is_minimum[index] = true;
                is_changed = true;
            }
        }
        if !is_changed {
            break;
        }
    }

    let (free, shares) = free_width(columns, &is_minimum, available);

    columns
        .iter()
        .zip(&is_minimum)
        .map(|(column, is_minimum)| {
            if *is_minimum {
                f32::from(column.min_width)
            } else {
                share_width(column, free, shares)
            }
        })
        .collect()
}

/// Returns the width left for the columns that are not at their minimum,
/// and the sum of their shares.
fn free_width(
    columns: &[TableColumn],
    is_minimum: &[bool],
    available: f32,
) -> (f32, f32) {
    let mut taken = 0.0;
    let mut shares = 0.0;
    for (column, is_minimum) in columns.iter().zip(is_minimum) {
        if *is_minimum {
            taken += f32::from(column.min_width);
        } else {
            shares += f32::from(column.share);
        }
    }

    ((available - taken).max(0.0), shares)
}

/// Returns the part of `free` width that `column` gets out of `shares`.
fn share_width(column: &TableColumn, free: f32, shares: f32) -> f32 {
    if shares > 0.0 {
        free * f32::from(column.share) / shares
    } else {
        0.0
    }
}

/// Color of a link cell, on every row look: `accent`.
fn link_color(palette: &UiPalette) -> Color {
    palette.accent()
}

/// Color of the sort arrow and the title of the sorted column:
/// `text-body`, a step above the muted titles.
fn sort_arrow_color(palette: &UiPalette) -> Color {
    palette.text_body()
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
    use crate::contrast::contrast_ratio;

    /// WCAG 2.1 AA minimum for normal text.
    const TEXT_MIN: f32 = 4.5;
    /// WCAG 2.1 AA minimum for non-text marks.
    const NON_TEXT_MIN: f32 = 3.0;
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
        Sorted(usize),
        Linked(usize, usize, usize),
        MenuPressed(usize, Point),
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

    /// Columns of the tables with sorting, links and row menus: Name and
    /// Size sort, Compose holds links.
    fn action_columns() -> Vec<TableColumn> {
        vec![
            TableColumn::new("Name", 3).sortable(true),
            TableColumn::new("Compose", 2),
            TableColumn::new("Size", 1)
                .align(ColumnAlign::End)
                .sortable(true),
        ]
    }

    /// Table of `count` rows `row-0`, `row-1`, … with a Compose link
    /// `site-<index>`, sorting, link presses and row menus. Row 1 has no
    /// menu.
    fn action_table(count: usize, height: f32) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();
        let mut table = DataTable::new(action_columns(), &palette)
            .sorted_by(0, SortDirection::Ascending)
            .on_row_press(Message::Pressed)
            .on_sort(Message::Sorted)
            .on_link_press(Message::Linked)
            .on_row_menu("More actions", Message::MenuPressed);
        for index in 0..count {
            let cells = vec![
                TableCell::mono(format!("row-{index}")),
                TableCell::link(format!("site-{index}")),
                TableCell::number("1 MB"),
            ];
            table = if index == 1 {
                table.row_without_menu(cells)
            } else {
                table.row(cells)
            };
        }

        container(table).width(WIDTH).height(height).into()
    }

    /// Center of the "⋯" button of the row at `index`, with no scroll.
    fn menu_button_center(index: usize) -> Point {
        Point::new(
            WIDTH - MENU_COLUMN_WIDTH / 2.0,
            BODY_TOP + index as f32 * ROW_STRIDE + size::TABLE_ROW / 2.0,
        )
    }

    /// Bottom-left corner of the "⋯" button of the row at `index`, with no
    /// scroll.
    fn menu_button_corner(index: usize) -> Point {
        Point::new(
            WIDTH - MENU_COLUMN_WIDTH
                + (MENU_COLUMN_WIDTH - size::CONTROL_SM) / 2.0,
            BODY_TOP
                + index as f32 * ROW_STRIDE
                + (size::TABLE_ROW + size::CONTROL_SM) / 2.0,
        )
    }

    fn left_click_at(ui: &mut iced_test::Simulator<'_, Message>, point: Point) {
        ui.point_at(point);
        let _ = ui.simulate(simulator::click());
    }

    #[test]
    fn given_sortable_column_when_its_title_pressed_then_its_index_is_sent() {
        let mut ui = simulator(action_table(3, 300.0));

        ui.click("Size").expect("title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Sorted(2)]);
    }

    #[test]
    fn given_column_that_does_not_sort_when_its_title_pressed_then_nothing_is_sent()
     {
        let mut ui = simulator(action_table(3, 300.0));

        ui.click("Compose").expect("title should be visible");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_sortable_columns_without_handler_when_title_pressed_then_nothing_is_sent()
     {
        let palette = UiPalette::otty_dark();
        let table: Element<'_, Message> =
            DataTable::new(action_columns(), &palette)
                .row(vec![TableCell::plain("redis")])
                .into();
        let mut ui = simulator(container(table).width(WIDTH).height(200));

        ui.click("Name").expect("title should be visible");

        assert_eq!(ui.into_messages().count(), 0);
    }

    /// Draws a one-row table sorted as `sort` and checks it against the
    /// picture saved in `reference`; the first call saves the picture.
    fn sorted_table_matches(
        sort: Option<(usize, SortDirection)>,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let mut table = DataTable::<Message>::new(action_columns(), &palette)
            .row(vec![TableCell::plain("redis")]);
        if let Some((column, direction)) = sort {
            table = table.sorted_by(column, direction);
        }
        let mut ui = simulator(container(table).width(WIDTH).height(120));

        ui.snapshot(&iced::Theme::Dark)
            .expect("table should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    /// Two equal columns, the second right-aligned, sorted by `sorted`.
    fn sorted_pair(sorted: usize) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();
        let columns = vec![
            TableColumn::new("Name", 1).sortable(true),
            TableColumn::new("Size", 1)
                .align(ColumnAlign::End)
                .sortable(true),
        ];
        let table: Element<'_, Message> = DataTable::new(columns, &palette)
            .row(vec![TableCell::plain("redis"), TableCell::number("1")])
            .sorted_by(sorted, SortDirection::Ascending)
            .into();

        container(table).width(WIDTH).height(120).into()
    }

    #[test]
    fn given_left_aligned_sorted_column_when_laid_out_then_the_title_takes_only_its_text_so_the_arrow_follows_it()
     {
        let mut ui = simulator(sorted_pair(0));

        let title = ui.find("Name").expect("title should be visible").bounds();

        assert!(
            (title.x - CELL_PADDING_X).abs() < 0.5,
            "the title should start at the cell padding: {title:?}"
        );
        assert!(
            title.width < 60.0,
            "the title should be as wide as its text, so the arrow sits right \
             after it: {title:?}"
        );
    }

    #[test]
    fn given_right_aligned_sorted_column_when_laid_out_then_the_title_ends_at_the_cell_edge_with_the_arrow_before_it()
     {
        let mut ui = simulator(sorted_pair(1));

        let title = ui.find("Size").expect("title should be visible").bounds();

        assert!(
            (title.x + title.width - (WIDTH - CELL_PADDING_X)).abs() < 0.5,
            "the title should line up with the cells at the right edge: \
             {title:?}"
        );
        assert!(
            title.width < 60.0,
            "the title should be as wide as its text: {title:?}"
        );
    }

    #[test]
    fn given_long_title_in_a_narrow_sorted_column_when_laid_out_then_the_arrow_still_fits()
     {
        let palette = UiPalette::otty_dark();
        let columns = vec![
            TableColumn::new("A very long column title", 1).sortable(true),
        ];
        let table: Element<'_, Message> = DataTable::new(columns, &palette)
            .row(vec![TableCell::plain("redis")])
            .sorted_by(0, SortDirection::Ascending)
            .into();
        let mut ui = simulator(container(table).width(120).height(120));

        let title = ui
            .find("A very long column title")
            .expect("title should be visible")
            .bounds();

        assert!(
            title.x + title.width
                <= 120.0 - CELL_PADDING_X - SORT_ARROW - spacing::SPACE_1 + 0.5,
            "the title should leave room for the arrow: {title:?}"
        );
    }

    #[test]
    fn given_sorted_column_when_drawn_then_an_arrow_shows_its_direction() {
        let reference = std::env::temp_dir()
            .join(format!("otty-data-table-{}", std::process::id()))
            .join("sorted-ascending");
        let ascending = Some((0, SortDirection::Ascending));

        assert!(sorted_table_matches(ascending, &reference));
        assert!(!sorted_table_matches(
            Some((0, SortDirection::Descending)),
            &reference
        ));
        assert!(!sorted_table_matches(None, &reference));
        assert!(!sorted_table_matches(
            Some((2, SortDirection::Ascending)),
            &reference
        ));
        assert!(sorted_table_matches(ascending, &reference));
    }

    #[test]
    fn given_link_cell_when_pressed_then_its_row_and_column_are_sent_and_the_row_is_not_pressed()
     {
        let mut ui = simulator(action_table(3, 300.0));

        ui.click("site-2").expect("link should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Linked(2, 1, 0)]);
    }

    /// Table with one row whose Ports cell holds two links with text
    /// between and after them, and a row press.
    fn ports_table() -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();
        let table = DataTable::new(
            vec![TableColumn::new("Name", 1), TableColumn::new("Ports", 3)],
            &palette,
        )
        .on_row_press(Message::Pressed)
        .on_link_press(Message::Linked)
        .row(vec![
            TableCell::mono("web"),
            TableCell::parts(vec![
                CellPart::link("8080:80/tcp"),
                CellPart::text(" and "),
                CellPart::link("5432/tcp"),
                CellPart::text(" then 53/udp"),
            ]),
        ]);

        container(table).width(WIDTH).height(120).into()
    }

    #[test]
    fn given_two_links_in_one_cell_when_each_is_pressed_then_each_sends_its_own_index()
     {
        let mut first = simulator(ports_table());
        first.click("8080:80/tcp").expect("first link is visible");
        let first: Vec<Message> = first.into_messages().collect();

        let mut second = simulator(ports_table());
        second.click("5432/tcp").expect("second link is visible");
        let second: Vec<Message> = second.into_messages().collect();

        assert_eq!(first, vec![Message::Linked(0, 1, 0)]);
        assert_eq!(second, vec![Message::Linked(0, 1, 1)]);
    }

    #[test]
    fn given_text_between_links_when_pressed_then_the_row_is_pressed_not_a_link()
     {
        let mut ui = simulator(ports_table());

        ui.click(" and ").expect("text is visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Pressed(0)]);
    }

    #[test]
    fn given_parts_cell_when_rendered_then_each_part_shows_in_order() {
        let mut ui = simulator(ports_table());

        for part in ["8080:80/tcp", " and ", "5432/tcp", " then 53/udp"] {
            assert!(ui.find(part).is_ok(), "{part:?} should be shown");
        }
    }

    #[test]
    fn given_parts_cell_without_handler_when_a_link_is_pressed_then_the_row_is_pressed()
     {
        let palette = UiPalette::otty_dark();
        let table: Element<'_, Message> =
            DataTable::new(vec![TableColumn::new("Ports", 1)], &palette)
                .on_row_press(Message::Pressed)
                .row(vec![TableCell::parts(vec![CellPart::link("80/tcp")])])
                .into();
        let mut ui = simulator(container(table).width(WIDTH).height(120));

        ui.click("80/tcp").expect("link is visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Pressed(0)]);
    }

    /// Draws the first row of the action table with the pointer at
    /// `pointer` and checks it against the picture saved in `reference`;
    /// the first call saves the picture.
    fn hovered_row_matches(
        pointer: Point,
        reference: &std::path::Path,
    ) -> bool {
        let mut ui = simulator(action_table(1, 120.0));
        ui.point_at(pointer);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: pointer,
        })]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("table should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn given_link_cell_when_pointer_over_it_then_it_is_underlined() {
        let reference = std::env::temp_dir()
            .join(format!("otty-data-table-{}", std::process::id()))
            .join("row-hovered");
        let row_y = BODY_TOP + size::TABLE_ROW / 2.0;
        let compose = (WIDTH - MENU_COLUMN_WIDTH) / 2.0;
        // Both points light the row; only the first is off the link.
        let beside = Point::new(compose + 100.0, row_y);
        let link = Point::new(compose + CELL_PADDING_X + 4.0, row_y);

        assert!(hovered_row_matches(beside, &reference));
        assert!(!hovered_row_matches(link, &reference));
        assert!(hovered_row_matches(beside, &reference));
    }

    #[test]
    fn given_link_cell_when_space_beside_it_pressed_then_the_row_is_pressed() {
        let mut ui = simulator(action_table(3, 300.0));
        // The Compose column ends at 5/6 of the width left of the menu
        // column; its link is short, so the end of the column is blank row.
        let blank = Point::new(
            (WIDTH - MENU_COLUMN_WIDTH) * 5.0 / 6.0 - 20.0,
            BODY_TOP + size::TABLE_ROW / 2.0,
        );

        left_click_at(&mut ui, blank);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Pressed(0)]);
    }

    #[test]
    fn given_link_cell_without_handler_when_pressed_then_the_row_is_pressed() {
        let palette = UiPalette::otty_light();
        let table: Element<'_, Message> =
            DataTable::new(action_columns(), &palette)
                .row(vec![TableCell::plain("redis"), TableCell::link("site")])
                .on_row_press(Message::Pressed)
                .into();
        let mut ui = simulator(container(table).width(WIDTH).height(200));

        ui.click("site").expect("link should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Pressed(0)]);
    }

    #[test]
    fn given_row_menu_when_its_button_pressed_then_index_and_corner_are_sent_and_the_row_is_not_pressed()
     {
        let mut ui = simulator(action_table(3, 300.0));

        left_click_at(&mut ui, menu_button_center(2));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::MenuPressed(2, menu_button_corner(2))]
        );
    }

    #[test]
    fn given_scrolled_rows_when_menu_button_pressed_then_corner_is_where_the_button_shows()
     {
        let mut ui = simulator(action_table(20, BODY_TOP + 3.0 * ROW_STRIDE));
        let pointer = menu_button_center(0);

        ui.point_at(pointer);
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: ScrollDelta::Pixels {
                x: 0.0,
                y: -2.0 * ROW_STRIDE,
            },
        })]);
        let _ = ui.simulate(simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::MenuPressed(2, menu_button_corner(0))]
        );
    }

    #[test]
    fn given_row_without_menu_when_its_menu_place_pressed_then_the_row_is_pressed()
     {
        let mut ui = simulator(action_table(3, 300.0));

        left_click_at(&mut ui, menu_button_center(1));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Pressed(1)]);
    }

    /// Draws a one-row table whose "⋯" tooltip is `label`, with the pointer
    /// over the button, and checks it against the picture saved in
    /// `reference`; the first call saves the picture. Tooltips are
    /// overlays that widget queries do not reach.
    fn hovered_menu_button_matches(
        label: &str,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let table = DataTable::new(action_columns(), &palette)
            .row(vec![TableCell::plain("redis")])
            .on_row_menu(label, Message::MenuPressed);
        let mut ui = simulator(container(table).width(WIDTH).height(160));
        let center = menu_button_center(0);
        ui.point_at(center);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: center,
        })]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("table should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn given_row_menu_when_its_button_hovered_then_the_tooltip_shows_its_label()
    {
        let reference = std::env::temp_dir()
            .join(format!("otty-data-table-{}", std::process::id()))
            .join("menu-tooltip");

        assert!(hovered_menu_button_matches("More actions", &reference));
        assert!(!hovered_menu_button_matches("Row actions", &reference));
        assert!(hovered_menu_button_matches("More actions", &reference));
    }

    /// Minimum widths of the "Name", "Compose" and "Size" columns of
    /// [`minimum_columns`].
    const NAME_MIN: u16 = 150;
    const COMPOSE_MIN: u16 = 150;
    const SIZE_MIN: u16 = 100;

    /// The action columns with a minimum width on each.
    fn minimum_columns() -> Vec<TableColumn> {
        vec![
            TableColumn::new("Name", 3)
                .sortable(true)
                .min_width(NAME_MIN),
            TableColumn::new("Compose", 2).min_width(COMPOSE_MIN),
            TableColumn::new("Size", 1)
                .align(ColumnAlign::End)
                .sortable(true)
                .min_width(SIZE_MIN),
        ]
    }

    /// Table of `count` rows like [`action_table`], with minimum widths, in a
    /// space `width` wide.
    fn minimum_table(
        count: usize,
        width: f32,
        height: f32,
    ) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();
        let mut table = DataTable::new(minimum_columns(), &palette)
            .on_row_press(Message::Pressed)
            .on_row_right_press(Message::RightPressed)
            .on_sort(Message::Sorted)
            .on_link_press(Message::Linked)
            .on_row_menu("More actions", Message::MenuPressed);
        for index in 0..count {
            table = table.row(vec![
                TableCell::mono(format!("row-{index}")),
                TableCell::link(format!("site-{index}")),
                TableCell::number("1 MB"),
            ]);
        }

        container(table).width(width).height(height).into()
    }

    /// Scrolls the table under `pointer` sideways as far as it goes.
    fn scroll_to_the_right_end(
        ui: &mut iced_test::Simulator<'_, Message>,
        pointer: Point,
    ) {
        ui.point_at(pointer);
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: ScrollDelta::Pixels {
                x: -10_000.0,
                y: 0.0,
            },
        })]);
    }

    /// Width of the text of a title or cell: the column minus its padding.
    fn text_width(
        ui: &mut iced_test::Simulator<'_, Message>,
        text: &str,
    ) -> f32 {
        ui.find(text).expect("text should be shown").bounds().width
    }

    #[test]
    fn given_columns_without_minimums_when_laid_out_then_widths_follow_shares()
    {
        let columns = vec![TableColumn::new("A", 3), TableColumn::new("B", 1)];

        assert_eq!(column_widths(&columns, 400.0), vec![300.0, 100.0]);
    }

    #[test]
    fn given_column_below_its_minimum_when_laid_out_then_it_keeps_the_minimum_and_others_shrink()
     {
        let columns = vec![
            TableColumn::new("A", 3),
            TableColumn::new("B", 1).min_width(80),
        ];

        // B would get 25 of 100; it keeps 80 and A gets what is left.
        assert_eq!(column_widths(&columns, 100.0), vec![20.0, 80.0]);
    }

    #[test]
    fn given_several_minimums_when_laid_out_then_each_is_kept_in_turn() {
        let columns = vec![
            TableColumn::new("A", 3),
            TableColumn::new("B", 2).min_width(150),
            TableColumn::new("C", 1).min_width(100),
        ];

        // Shares give C 77, then B 146 after C keeps 100: both keep theirs.
        assert_eq!(column_widths(&columns, 464.0), vec![214.0, 150.0, 100.0]);
    }

    #[test]
    fn given_less_room_than_the_minimums_when_laid_out_then_columns_are_their_minimums()
     {
        let columns = vec![
            TableColumn::new("A", 3).min_width(150),
            TableColumn::new("B", 1).min_width(100),
            TableColumn::new("C", 1),
        ];

        assert_eq!(column_widths(&columns, 200.0), vec![150.0, 100.0, 0.0]);
    }

    #[test]
    fn given_column_with_minimum_when_table_is_narrow_then_it_keeps_it_and_the_others_shrink()
     {
        let mut wide = simulator(minimum_table(1, 800.0, 200.0));
        let mut narrow = simulator(minimum_table(1, 500.0, 200.0));

        let wide_size = text_width(&mut wide, "Size");
        let narrow_size = text_width(&mut narrow, "Size");
        let wide_name = text_width(&mut wide, "Name");
        let narrow_name = text_width(&mut narrow, "Name");

        assert!(
            narrow_size >= f32::from(SIZE_MIN) - 2.0 * CELL_PADDING_X - 0.5,
            "Size should keep its minimum: {narrow_size}"
        );
        assert!(narrow_size <= wide_size);
        assert!(
            narrow_name < wide_name,
            "Name should give way: {narrow_name} against {wide_name}"
        );
    }

    #[test]
    fn given_less_room_than_the_minimums_when_scrolled_sideways_then_the_header_moves_with_the_rows()
     {
        // The table is 300 wide, so the minimums (400) and the "⋯" column
        // do not fit. Scrolled to the end, the Size column is under x = 200
        // and the Compose column, which does not sort, was there before.
        let title = Point::new(200.0, size::TABLE_HEADER / 2.0);

        let mut resting = simulator(minimum_table(2, 300.0, 200.0));
        let size_width = text_width(&mut resting, "Size");
        left_click_at(&mut resting, title);
        let resting: Vec<Message> = resting.into_messages().collect();

        let mut scrolled = simulator(minimum_table(2, 300.0, 200.0));
        scroll_to_the_right_end(&mut scrolled, Point::new(100.0, BODY_TOP));
        left_click_at(&mut scrolled, title);
        let scrolled: Vec<Message> = scrolled.into_messages().collect();

        assert!(
            (size_width - (f32::from(SIZE_MIN) - 2.0 * CELL_PADDING_X)).abs()
                < 0.5,
            "the column should be its minimum: {size_width}"
        );
        assert_eq!(resting, Vec::<Message>::new());
        assert_eq!(scrolled, vec![Message::Sorted(2)]);
    }

    #[test]
    fn given_less_room_than_the_minimums_when_scrolled_sideways_then_presses_hit_the_right_row_and_column()
     {
        let row_y = |index: usize| {
            BODY_TOP + index as f32 * ROW_STRIDE + size::TABLE_ROW / 2.0
        };
        let width = 300.0;
        let menu_x = width - MENU_COLUMN_WIDTH / 2.0;
        let corner = Point::new(
            width - MENU_COLUMN_WIDTH
                + (MENU_COLUMN_WIDTH - size::CONTROL_SM) / 2.0,
            BODY_TOP + ROW_STRIDE + (size::TABLE_ROW + size::CONTROL_SM) / 2.0,
        );
        let pointer = Point::new(100.0, row_y(1));

        let mut row = simulator(minimum_table(3, width, 300.0));
        scroll_to_the_right_end(&mut row, Point::new(100.0, row_y(0)));
        left_click_at(&mut row, pointer);
        let row: Vec<Message> = row.into_messages().collect();

        let mut right = simulator(minimum_table(3, width, 300.0));
        scroll_to_the_right_end(&mut right, Point::new(100.0, row_y(0)));
        right.point_at(pointer);
        let _ = right.simulate(right_click());
        let right: Vec<Message> = right.into_messages().collect();

        let mut menu = simulator(minimum_table(3, width, 300.0));
        scroll_to_the_right_end(&mut menu, Point::new(100.0, row_y(0)));
        left_click_at(&mut menu, Point::new(menu_x, row_y(1)));
        let menu: Vec<Message> = menu.into_messages().collect();

        let mut link = simulator(minimum_table(3, width, 300.0));
        scroll_to_the_right_end(&mut link, Point::new(100.0, row_y(0)));
        link.click("site-2").expect("link should be shown");
        let link: Vec<Message> = link.into_messages().collect();

        let mut sort = simulator(minimum_table(3, width, 300.0));
        scroll_to_the_right_end(&mut sort, Point::new(100.0, row_y(0)));
        left_click_at(&mut sort, Point::new(5.0, size::TABLE_HEADER / 2.0));
        let sort: Vec<Message> = sort.into_messages().collect();

        assert_eq!(row, vec![Message::Pressed(1)]);
        assert_eq!(right, vec![Message::RightPressed(1, pointer)]);
        assert_eq!(menu, vec![Message::MenuPressed(1, corner)]);
        assert_eq!(link, vec![Message::Linked(2, 1, 0)]);
        assert_eq!(sort, vec![Message::Sorted(0)]);
    }

    #[test]
    fn given_each_theme_when_links_drawn_on_rows_then_they_reach_4_5() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            for surface in [
                palette.surface_canvas(),
                palette.surface_header(),
                palette.surface_selected(),
            ] {
                let ratio = contrast_ratio(link_color(&palette), surface);

                assert!(ratio >= TEXT_MIN, "link on {surface:?}: {ratio:.2}");
            }
        }
    }

    #[test]
    fn given_each_theme_when_sort_arrow_drawn_then_it_reaches_3() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            for surface in [
                palette.surface_canvas(),
                palette.surface_header(),
                palette.surface_selected(),
            ] {
                let ratio = contrast_ratio(sort_arrow_color(&palette), surface);

                assert!(
                    ratio >= NON_TEXT_MIN,
                    "arrow on {surface:?}: {ratio:.2}"
                );
            }
        }
    }
}
