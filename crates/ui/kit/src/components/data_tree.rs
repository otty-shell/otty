//! `DataTree`: structured data as readable rows in groups and sections.

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, responsive, row, svg, text};
use iced::{Alignment, Color, Element, Length, Padding};

use super::glyph;
use super::key_value_list::{self, TEXT_SIZE, ValuePart};
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Edge of the caret of a section title.
const CARET_SIZE: f32 = 11.0;
/// Indent of one level of rows.
const LEVEL_INDENT: f32 = 12.0;
/// The most a row is indented, so deep rows keep room for their value in a
/// narrow panel.
const MAX_INDENT: f32 = 4.0 * LEVEL_INDENT;

/// One row of a [`DataTree`]: a key and its value, or a key that only titles
/// the deeper rows after it. Rows are plain data; the caller decides what
/// they say.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::data_tree::DataRow;
///
/// let rows = [
///     DataRow::heading(0, "Health"),
///     DataRow::new(1, "Status", "healthy"),
/// ];
/// assert_ne!(rows[0], rows[1]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRow {
    depth: u16,
    key: String,
    value: Option<String>,
}

impl DataRow {
    /// A `key` and its `value`, indented by `depth` levels.
    pub fn new(
        depth: u16,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            depth,
            key: key.into(),
            value: Some(value.into()),
        }
    }

    /// A `key` without a value, titling the deeper rows after it, indented
    /// by `depth` levels.
    pub fn heading(depth: u16, key: impl Into<String>) -> Self {
        Self {
            depth,
            key: key.into(),
            value: None,
        }
    }
}

/// Groups and sections of [`DataRow`]s, in the order they are added. Rows
/// follow the rules of a key-value list: muted keys, mono values that wrap,
/// and each value under its key when the row is narrow. Deeper rows are
/// indented.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::data_tree::{DataRow, DataTree};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     ToggleState,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let tree: Element<'_, Message> = DataTree::new(&palette)
///     .group("General", vec![DataRow::new(0, "Id", "850b")])
///     .section(
///         "State",
///         true,
///         vec![
///             DataRow::new(0, "Status", "running"),
///             DataRow::heading(0, "Health"),
///             DataRow::new(1, "Status", "healthy"),
///         ],
///         Message::ToggleState,
///     )
///     .into();
/// ```
pub struct DataTree<Message> {
    palette: UiPalette,
    parts: Vec<Part<Message>>,
}

impl<Message> DataTree<Message> {
    /// Creates an empty tree.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            parts: Vec::new(),
        }
    }

    /// Adds a group that is always open.
    pub fn group(
        mut self,
        title: impl Into<String>,
        rows: Vec<DataRow>,
    ) -> Self {
        self.parts.push(Part {
            title: title.into(),
            rows,
            collapse: None,
        });
        self
    }

    /// Adds a section whose rows show only while `is_open`; pressing its
    /// title sends `on_toggle`.
    pub fn section(
        mut self,
        title: impl Into<String>,
        is_open: bool,
        rows: Vec<DataRow>,
        on_toggle: Message,
    ) -> Self {
        self.parts.push(Part {
            title: title.into(),
            rows,
            collapse: Some((is_open, on_toggle)),
        });
        self
    }
}

impl<'a, Message: Clone + 'a> From<DataTree<Message>> for Element<'a, Message> {
    fn from(value: DataTree<Message>) -> Self {
        let palette = value.palette;
        let parts = value.parts;

        // `responsive` fills the height by default; shrinking keeps the tree
        // as tall as its rows, also inside a scrollable panel.
        responsive(move |available| {
            let mut tree = column![].spacing(spacing::SPACE_4);

            for part in &parts {
                tree = tree.push(part_view(part, available.width, &palette));
            }

            tree.into()
        })
        .height(Length::Shrink)
        .into()
    }
}

/// One group or section; a section has its open state and its toggle.
struct Part<Message> {
    title: String,
    rows: Vec<DataRow>,
    collapse: Option<(bool, Message)>,
}

/// Draws the title of `part` and, unless it is a closed section, its rows.
fn part_view<'a, Message: Clone + 'a>(
    part: &Part<Message>,
    width: f32,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let is_open = match &part.collapse {
        Some((is_open, _)) => *is_open,
        None => true,
    };

    let mut lines =
        column![title_view(part, palette)].spacing(spacing::SPACE_2);
    if is_open {
        for data_row in &part.rows {
            lines = lines.push(row_view(data_row, width, palette));
        }
    }

    lines.into()
}

/// Draws the title of `part`: plain for a group, with a caret and a press
/// for a section.
fn title_view<'a, Message: Clone + 'a>(
    part: &Part<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let heading = typography::HEADING;
    let title = text(part.title.clone())
        .size(heading.size())
        .font(fonts::UI_SEMIBOLD)
        .color(title_color(palette));

    let Some((is_open, on_toggle)) = &part.collapse else {
        return container(title)
            .height(size::ROW)
            .align_y(Alignment::Center)
            .into();
    };

    let muted = palette.text_muted();
    let content = row![
        svg(glyph::caret(*is_open))
            .width(CARET_SIZE)
            .height(CARET_SIZE)
            .style(move |_, _| svg::Style { color: Some(muted) }),
        title,
    ]
    .spacing(spacing::SPACE_2)
    .height(Length::Fill)
    .align_y(Alignment::Center);

    button(content)
        .width(Length::Fill)
        .height(size::ROW)
        .padding(0)
        .on_press(on_toggle.clone())
        .style(|_, _| button::Style::default())
        .into()
}

/// Draws one row, indented by its depth. The stacking rule looks at the
/// width left after the indent.
fn row_view<'a, Message: Clone + 'a>(
    data_row: &DataRow,
    width: f32,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let indent = (f32::from(data_row.depth) * LEVEL_INDENT).min(MAX_INDENT);
    let stacked = width - indent < size::KEY_VALUE_STACK_BELOW;

    let content: Element<'a, Message> = match &data_row.value {
        Some(value) => key_value_list::entry_view(
            &data_row.key,
            &[ValuePart::text(value.clone())],
            stacked,
            palette,
        ),
        None => text(data_row.key.clone())
            .size(TEXT_SIZE)
            .font(fonts::UI)
            .wrapping(Wrapping::Glyph)
            .color(heading_color(palette))
            .into(),
    };

    container(content)
        .width(Length::Fill)
        .padding(Padding {
            left: indent,
            ..Padding::ZERO
        })
        .into()
}

/// Color of a group or section title.
fn title_color(palette: &UiPalette) -> Color {
    palette.text_strong()
}

/// Color of a row that titles the deeper rows after it.
fn heading_color(palette: &UiPalette) -> Color {
    palette.text_body()
}

#[cfg(test)]
mod tests {
    use iced::widget::{container, scrollable};
    use iced_test::simulator;

    use super::*;
    use crate::size;

    /// Indent of one level, in the tests.
    const ONE_LEVEL: f32 = 12.0;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Toggle(&'static str),
    }

    fn state_rows() -> Vec<DataRow> {
        vec![
            DataRow::new(0, "Status", "running"),
            DataRow::heading(0, "Health"),
            DataRow::new(1, "FailingStreak", "0"),
        ]
    }

    /// A General group, an open State section and a closed Config section.
    fn tree(palette: &UiPalette) -> DataTree<Message> {
        DataTree::new(palette)
            .group("General", vec![DataRow::new(0, "Id", "850b")])
            .section("State", true, state_rows(), Message::Toggle("State"))
            .section(
                "Config",
                false,
                vec![DataRow::new(0, "Image", "busybox:1.37")],
                Message::Toggle("Config"),
            )
    }

    /// The tree inside a container `width` pixels wide.
    fn tree_in(palette: &UiPalette, width: f32) -> Element<'_, Message> {
        container(tree(palette)).width(width).into()
    }

    #[test]
    fn given_a_tree_when_rendered_then_groups_and_open_sections_show_their_rows()
     {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(tree_in(&palette, 400.0));

        for label in [
            "General", "Id", "850b", "State", "Status", "running", "Health",
            "Config",
        ] {
            assert!(ui.find(label).is_ok(), "{label} should show");
        }
    }

    #[test]
    fn given_a_closed_section_when_rendered_then_only_its_title_shows() {
        let palette = UiPalette::otty_light();
        let mut ui = simulator(tree_in(&palette, 400.0));

        ui.find("Config").expect("title should show");

        assert!(ui.find("Image").is_err());
        assert!(ui.find("busybox:1.37").is_err());
    }

    #[test]
    fn given_section_titles_when_pressed_then_each_sends_its_own_toggle() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(tree_in(&palette, 400.0));

        ui.click("Config").expect("closed section");
        ui.click("State").expect("open section");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::Toggle("Config"), Message::Toggle("State")]
        );
    }

    #[test]
    fn given_a_group_title_when_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(tree_in(&palette, 400.0));

        ui.click("General").expect("group title");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn given_deeper_rows_when_laid_out_then_each_level_is_indented() {
        let palette = UiPalette::otty_dark();
        let rows = vec![
            DataRow::heading(0, "Level0"),
            DataRow::heading(1, "Level1"),
            DataRow::new(2, "Level2", "value"),
        ];
        let tree: Element<'_, ()> =
            container(DataTree::new(&palette).group("General", rows))
                .width(400)
                .into();
        let mut ui = simulator(tree);

        let first = ui.find("Level0").expect("level 0").bounds();
        let second = ui.find("Level1").expect("level 1").bounds();
        let third = ui.find("Level2").expect("level 2").bounds();

        assert!((second.x - first.x - ONE_LEVEL).abs() < 0.5, "{second:?}");
        assert!((third.x - second.x - ONE_LEVEL).abs() < 0.5, "{third:?}");
    }

    #[test]
    fn given_very_deep_rows_when_laid_out_then_the_indent_stops_growing() {
        let palette = UiPalette::otty_dark();
        let long = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef";
        let rows = vec![
            DataRow::new(MAX_TEST_DEPTH, "Deep", long),
            DataRow::new(MAX_TEST_DEPTH * 2, "Deeper", "value"),
        ];
        let tree: Element<'_, ()> =
            container(DataTree::new(&palette).group("General", rows))
                .width(160)
                .into();
        let mut ui = simulator(tree);

        let deep = ui.find("Deep").expect("deep key").bounds();
        let deeper = ui.find("Deeper").expect("deeper key").bounds();
        let value = ui.find(long).expect("long value").bounds();

        assert!((deep.x - deeper.x).abs() < 0.5, "{deep:?} and {deeper:?}");
        assert!(deep.x < 80.0, "{deep:?} keeps room for its value");
        assert!(value.x + value.width <= 160.5, "{value:?} inside 160px");
    }

    /// A depth past the most indented level.
    const MAX_TEST_DEPTH: u16 = 20;

    #[test]
    fn given_a_wide_tree_when_laid_out_then_values_sit_right_of_their_keys() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(tree_in(&palette, 400.0));

        let key = ui.find("Status").expect("key").bounds();
        let value = ui.find("running").expect("value").bounds();

        assert!(
            (value.x - key.x - size::KEY_VALUE_KEY - crate::spacing::SPACE_3)
                .abs()
                < 0.5,
            "{value:?} right of {key:?}"
        );
    }

    #[test]
    fn given_a_narrow_tree_when_laid_out_then_values_sit_below_their_keys() {
        let palette = UiPalette::otty_dark();
        let mut ui =
            simulator(tree_in(&palette, size::KEY_VALUE_STACK_BELOW - 20.0));

        let key = ui.find("Status").expect("key").bounds();
        let value = ui.find("running").expect("value").bounds();

        assert!((value.x - key.x).abs() < 0.5, "{value:?} under {key:?}");
        assert!(
            value.y >= key.y + key.height - 0.5,
            "{value:?} below {key:?}"
        );
    }

    #[test]
    fn given_a_width_just_enough_for_the_top_level_then_a_deeper_row_stacks() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(tree_in(&palette, size::KEY_VALUE_STACK_BELOW));

        let top_key = ui.find("Status").expect("top key").bounds();
        let top_value = ui.find("running").expect("top value").bounds();
        let deep_key = ui.find("FailingStreak").expect("deep key").bounds();
        let deep_value = ui.find("0").expect("deep value").bounds();

        assert!(top_value.x > top_key.x + 1.0, "{top_value:?} two columns");
        assert!(
            (deep_value.x - deep_key.x).abs() < 0.5,
            "{deep_value:?} under {deep_key:?}"
        );
    }

    #[test]
    fn given_a_long_value_when_laid_out_then_it_wraps_inside_the_width() {
        let palette = UiPalette::otty_dark();
        let long = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef";
        let tree: Element<'_, ()> = container(
            DataTree::new(&palette)
                .group("General", vec![DataRow::new(0, "Id", long)]),
        )
        .width(320)
        .into();
        let mut ui = simulator(tree);

        let value = ui.find(long).expect("value").bounds();

        assert!(value.x + value.width <= 320.5, "{value:?} inside 320px");
        assert!(value.height > 2.0 * 12.5, "{value:?} should wrap");
    }

    #[test]
    fn given_a_tree_in_a_scrollable_when_laid_out_then_it_sizes_to_its_content()
    {
        let palette = UiPalette::otty_dark();
        let tree: Element<'_, Message> = container(scrollable(tree(&palette)))
            .width(320)
            .height(600)
            .into();
        let mut ui = simulator(tree);

        let first = ui.find("General").expect("first title").bounds();
        let last = ui.find("Config").expect("last title").bounds();

        assert!(last.y > first.y, "{last:?} below {first:?}");
        assert!(last.y < 300.0, "{last:?} sits near the top, not centered");
    }

    #[test]
    fn given_each_theme_when_row_colors_are_drawn_on_panels_then_they_reach_4_5()
     {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            for surface in [palette.surface_panel(), palette.surface_canvas()] {
                for color in [title_color(&palette), heading_color(&palette)] {
                    let ratio = crate::contrast::contrast_ratio(color, surface);

                    assert!(
                        ratio >= 4.5,
                        "{color:?} on {surface:?}: {ratio:.2}"
                    );
                }
            }
        }
    }
}
