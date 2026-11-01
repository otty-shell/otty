//! `DataTable`: a header row over scrolling rows of cells, for lists of
//! resources such as containers or images.

use iced::widget::text::Alignment as TextAlignment;
use iced::widget::{
    Space, button, column, container, row, scrollable, svg, text, tooltip,
};
use iced::{Alignment, Color, Element, Font, Length, Padding, Point};

use super::ellipsis_text::EllipsisText;
use super::status_badge::{StatusBadge, StatusTone, TEXT_SIZE};
use super::table_body::{MenuPress, RightPress, TableBody};
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
/// Message builder for a link press: row index and column index.
type LinkPress<'a, Message> = Box<dyn Fn(usize, usize) -> Message + 'a>;

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
}

impl TableColumn {
    /// Creates a left-aligned column that is not sortable. `share` is its
    /// part of the table width: a column with share 2 is twice as wide as
    /// one with share 1.
    pub fn new(title: impl Into<String>, share: u16) -> Self {
        Self {
            title: title.into(),
            share,
            align: ColumnAlign::Start,
            is_sortable: false,
        }
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
///     OpenLink(usize, usize),
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

    /// Sets the message sent when a [`TableCell::Link`] is pressed, built
    /// from its row index and column index. The row is not pressed.
    /// Without it links are drawn but not pressable.
    pub fn on_link_press(
        mut self,
        on_link_press: impl Fn(usize, usize) -> Message + 'a,
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
            on_press: Box::new(on_press),
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
            sort,
            empty_message,
            on_row_press,
            on_row_right_press,
            on_sort,
            on_link_press,
            row_menu,
        } = value;
        let canvas = palette.surface_canvas();
        let has_menu_column = row_menu.is_some();

        let header = header_view(
            &columns,
            sort,
            on_sort.as_deref(),
            has_menu_column,
            &palette,
        );

        let body: Element<'a, Message> = if rows.is_empty() {
            empty_view(empty_message, &palette)
        } else {
            let menu_rows: Vec<bool> =
                rows.iter().map(|row| row.has_menu).collect();
            let menu_label = row_menu.as_ref().map(|menu| menu.label.as_str());

            let rows = rows
                .into_iter()
                .enumerate()
                .map(|(index, row)| {
                    let on_press =
                        on_row_press.as_ref().map(|on_press| on_press(index));
                    let on_link = |column: usize| {
                        on_link_press
                            .as_ref()
                            .map(|on_link_press| on_link_press(index, column))
                    };
                    let menu_cell = menu_label.map(|label| {
                        if row.has_menu {
                            menu_button(label, &palette)
                        } else {
                            Space::new().into()
                        }
                    });

                    row_view(
                        row.cells,
                        &columns,
                        selected == Some(index),
                        on_press,
                        &on_link,
                        menu_cell,
                        &palette,
                    )
                })
                .collect::<Vec<_>>();

            let mut body = TableBody::new(column(rows), on_row_right_press);
            if let Some(menu) = row_menu {
                body = body.row_menus(menu_rows, menu.on_press);
            }

            scrollable(body)
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

/// One body row of a [`DataTable`]: its cells and whether it ends with a
/// "⋯" button.
struct TableRow {
    cells: Vec<TableCell>,
    has_menu: bool,
}

/// The "⋯" button column of a [`DataTable`]: tooltip and message builder.
struct RowMenu<'a, Message> {
    label: String,
    on_press: MenuPress<'a, Message>,
}

/// Draws the header: medium titles over a `line` rule, and a blank title
/// over the "⋯" column when `has_menu_column`.
fn header_view<'a, Message: Clone + 'a>(
    columns: &[TableColumn],
    sort: Option<(usize, SortDirection)>,
    on_sort: Option<&(dyn Fn(usize) -> Message + 'a)>,
    has_menu_column: bool,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut titles = row(columns.iter().enumerate().map(|(index, column)| {
        let direction = sort
            .filter(|(sorted, _)| *sorted == index)
            .map(|(_, direction)| direction);
        let on_press = on_sort
            .filter(|_| column.is_sortable)
            .map(|on_sort| on_sort(index));

        header_cell(column, direction, on_press, palette)
    }));
    if has_menu_column {
        titles = titles.push(Space::new().width(MENU_COLUMN_WIDTH));
    }

    column![titles.height(size::TABLE_HEADER), rule(palette.line())].into()
}

/// Draws the title of `column`: muted, or `text-body` with an arrow in
/// `direction` when it is the sorted column. The arrow sits after the
/// title, or before it in a right-aligned column so the title still lines
/// up with the cells. With `on_press` the title is a button that shows
/// `surface-header` on hover.
fn header_cell<'a, Message: Clone + 'a>(
    column: &TableColumn,
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
    let cell = cell_frame(content, column.share, column.align);

    let Some(message) = on_press else {
        return cell;
    };

    button(cell)
        .width(Length::FillPortion(column.share))
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
/// the table has one.
///
/// `table_body::menu_button_bounds` finds the "⋯" button by this layout:
/// the row button, its row of cells, the last cell, its first child.
fn row_view<'a, Message: Clone + 'a>(
    cells: Vec<TableCell>,
    columns: &[TableColumn],
    is_selected: bool,
    on_press: Option<Message>,
    on_link: &dyn Fn(usize) -> Option<Message>,
    menu_cell: Option<Element<'a, Message>>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let hover = palette.surface_header();
    let selected = palette.surface_selected();

    let mut cells = cells.into_iter();
    let mut content = row(columns.iter().enumerate().map(|(index, column)| {
        let (content, align) = match cells.next() {
            Some(cell) => cell_view(
                cell,
                column.align,
                is_selected,
                on_link(index),
                palette,
            ),
            None => (Space::new().into(), column.align),
        };

        cell_frame(content, column.share, align)
    }))
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
/// `on_link` is the message of a link cell.
fn cell_view<'a, Message: Clone + 'a>(
    cell: TableCell,
    align: ColumnAlign,
    is_selected: bool,
    on_link: Option<Message>,
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
                return (link_view(label, align, on_link, palette), align);
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
        Linked(usize, usize),
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
        assert_eq!(messages, vec![Message::Linked(2, 1)]);
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
