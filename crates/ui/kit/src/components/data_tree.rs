//! `DataTree`: structured data as readable rows in groups and sections.

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, responsive, row, svg, text};
use iced::{Alignment, Color, Element, Length, Padding};

use super::glyph;
use super::key_value_list::{self, TEXT_SIZE, ValuePart};
use super::wrapped_text::WrappedText;
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
/// the deeper rows after it. A value can hold links, and a key that titles
/// deeper rows can be a link. Rows are plain data; the caller decides what
/// they say.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::data_tree::DataRow;
/// use otty_ui_kit::components::key_value_list::ValuePart;
///
/// let rows: [DataRow<&str>; 4] = [
///     DataRow::heading(0, "Health"),
///     DataRow::new(1, "Status", "healthy"),
///     DataRow::with_parts(
///         0,
///         "Image",
///         [ValuePart::link("nginx:1.27", "open image")],
///     ),
///     DataRow::heading_link(0, "otty-net", "open network"),
/// ];
/// assert_ne!(rows[0], rows[1]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRow<Message> {
    depth: u16,
    key: String,
    content: RowContent<Message>,
}

impl<Message> DataRow<Message> {
    /// A `key` and its plain `value`, indented by `depth` levels.
    pub fn new(
        depth: u16,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self::with_parts(depth, key, [ValuePart::text(value)])
    }

    /// A `key` whose value is `parts` in order, indented by `depth` levels:
    /// plain text and links, each link pressed on its own. The value wraps
    /// like a plain one.
    pub fn with_parts(
        depth: u16,
        key: impl Into<String>,
        parts: impl IntoIterator<Item = ValuePart<Message>>,
    ) -> Self {
        Self {
            depth,
            key: key.into(),
            content: RowContent::Value(parts.into_iter().collect()),
        }
    }

    /// A `key` without a value, titling the deeper rows after it, indented
    /// by `depth` levels.
    pub fn heading(depth: u16, key: impl Into<String>) -> Self {
        Self {
            depth,
            key: key.into(),
            content: RowContent::Heading(None),
        }
    }

    /// A [`heading`](Self::heading) whose key is a link that sends
    /// `on_press` when pressed.
    pub fn heading_link(
        depth: u16,
        key: impl Into<String>,
        on_press: Message,
    ) -> Self {
        Self {
            depth,
            key: key.into(),
            content: RowContent::Heading(Some(on_press)),
        }
    }

    /// The same row with the messages of its links changed by `change`.
    pub fn map<Mapped>(
        self,
        change: impl Fn(Message) -> Mapped,
    ) -> DataRow<Mapped> {
        let content = match self.content {
            RowContent::Value(parts) => RowContent::Value(
                parts.into_iter().map(|part| part.map(&change)).collect(),
            ),
            RowContent::Heading(on_press) => {
                RowContent::Heading(on_press.map(change))
            },
        };

        DataRow {
            depth: self.depth,
            key: self.key,
            content,
        }
    }
}

/// What a [`DataRow`] shows beside its key.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RowContent<Message> {
    /// A value of plain text and links.
    Value(Vec<ValuePart<Message>>),
    /// No value: the key titles the deeper rows. With a message the key is
    /// a link.
    Heading(Option<Message>),
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
        rows: Vec<DataRow<Message>>,
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
        rows: Vec<DataRow<Message>>,
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
    rows: Vec<DataRow<Message>>,
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
    data_row: &DataRow<Message>,
    width: f32,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let indent = (f32::from(data_row.depth) * LEVEL_INDENT).min(MAX_INDENT);
    let stacked = width - indent < size::KEY_VALUE_STACK_BELOW;

    let content: Element<'a, Message> = match &data_row.content {
        RowContent::Value(parts) => {
            key_value_list::entry_view(&data_row.key, parts, stacked, palette)
        },
        RowContent::Heading(Some(on_press)) => WrappedText::new(
            vec![(data_row.key.clone(), Some(on_press.clone()))],
            TEXT_SIZE,
            fonts::UI,
            heading_color(palette),
            key_value_list::link_color(palette),
        )
        .into(),
        RowContent::Heading(None) => text(data_row.key.clone())
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
    use crate::test_support::TempDir;

    /// Indent of one level, in the tests.
    const ONE_LEVEL: f32 = 12.0;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Toggle(&'static str),
    }

    fn state_rows() -> Vec<DataRow<Message>> {
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum LinkMessage {
        Open(&'static str),
    }

    fn open(name: &'static str) -> LinkMessage {
        LinkMessage::Open(name)
    }

    /// Rows with two links in one value, a value of text after a link, a
    /// heading that is a link and a plain heading.
    fn linked_rows() -> Vec<DataRow<LinkMessage>> {
        vec![
            DataRow::with_parts(
                0,
                "Ports",
                [
                    ValuePart::link("8080:80/tcp", open("8080")),
                    ValuePart::text(", "),
                    ValuePart::link("8443:443/tcp", open("8443")),
                ],
            ),
            DataRow::with_parts(
                0,
                "Name",
                [
                    ValuePart::link("otty-fx-data", open("volume")),
                    ValuePart::text(" → /data"),
                ],
            ),
            DataRow::heading_link(1, "otty-fx-net", open("network")),
            DataRow::new(2, "IPAddress", "172.18.0.2"),
            DataRow::heading(0, "Plain"),
        ]
    }

    /// The linked rows inside a container `width` pixels wide.
    fn linked_tree(
        palette: &UiPalette,
        width: f32,
    ) -> Element<'_, LinkMessage> {
        container(DataTree::new(palette).group("General", linked_rows()))
            .width(width)
            .into()
    }

    #[test]
    fn given_a_row_with_two_links_when_each_is_pressed_then_each_sends_its_own_message()
     {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let mut ui = simulator(linked_tree(&palette, 400.0));

            ui.click("8443:443/tcp").expect("second link");
            ui.click("8080:80/tcp").expect("first link");

            let messages: Vec<LinkMessage> = ui.into_messages().collect();
            assert_eq!(messages, vec![open("8443"), open("8080")]);
        }
    }

    #[test]
    fn given_a_narrow_tree_when_links_are_pressed_then_each_sends_its_message_under_its_key()
     {
        let palette = UiPalette::otty_dark();
        let width = size::KEY_VALUE_STACK_BELOW - 20.0;
        let mut ui = simulator(linked_tree(&palette, width));

        let key = ui.find("Name").expect("key").bounds();
        let link = ui.find("otty-fx-data").expect("volume link").bounds();
        ui.click("8080:80/tcp").expect("port link");
        ui.click("otty-fx-data").expect("volume link");

        assert!((link.x - key.x).abs() < 0.5, "{link:?} under {key:?}");
        assert!(link.y >= key.y + key.height - 0.5, "{link:?} below {key:?}");
        let messages: Vec<LinkMessage> = ui.into_messages().collect();
        assert_eq!(messages, vec![open("8080"), open("volume")]);
    }

    #[test]
    fn given_a_long_linked_value_when_laid_out_then_it_wraps_and_the_link_still_works()
     {
        let palette = UiPalette::otty_dark();
        let long = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef";
        let rows = vec![DataRow::with_parts(
            0,
            "Image",
            [
                ValuePart::link(long, open("image")),
                ValuePart::text(" tail"),
            ],
        )];
        let tree: Element<'_, LinkMessage> =
            container(DataTree::new(&palette).group("General", rows))
                .width(200)
                .into();
        let mut ui = simulator(tree);

        // A query for the link finds its first line; the whole value is
        // found by its whole text.
        let value = ui.find(format!("{long} tail")).expect("value").bounds();
        ui.click(long).expect("link");

        assert!(value.x + value.width <= 200.5, "{value:?} inside 200px");
        assert!(value.height > 2.0 * 12.5, "{value:?} should wrap");
        let messages: Vec<LinkMessage> = ui.into_messages().collect();
        assert_eq!(messages, vec![open("image")]);
    }

    #[test]
    fn given_text_beside_a_link_and_a_key_when_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_tree(&palette, 400.0));
        let link = ui.find("otty-fx-data").expect("volume link").bounds();
        // Inside " → /data", right after the link.
        let beside = iced::Point::new(
            link.x + link.width + 3.0 * TEXT_SIZE,
            link.center_y(),
        );

        ui.point_at(beside);
        let _ = ui.simulate(simulator::click());
        ui.click("Name").expect("key");
        ui.click("IPAddress").expect("key");
        ui.click("172.18.0.2").expect("plain value");
        ui.click("Plain").expect("plain heading");

        let messages: Vec<LinkMessage> = ui.into_messages().collect();
        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn given_a_heading_link_when_pressed_then_it_sends_its_message() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let mut ui = simulator(linked_tree(&palette, 400.0));

            ui.click("otty-fx-net").expect("heading link");

            let messages: Vec<LinkMessage> = ui.into_messages().collect();
            assert_eq!(messages, vec![open("network")]);
        }
    }

    #[test]
    fn given_a_heading_link_when_laid_out_then_it_is_indented_with_its_level() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(linked_tree(&palette, 400.0));

        let top = ui.find("Plain").expect("level 0").bounds();
        let heading = ui.find("otty-fx-net").expect("level 1").bounds();
        let deeper = ui.find("IPAddress").expect("level 2").bounds();

        assert!((heading.x - top.x - ONE_LEVEL).abs() < 0.5, "{heading:?}");
        assert!((deeper.x - heading.x - ONE_LEVEL).abs() < 0.5, "{deeper:?}");
    }

    /// Draws one heading with the pointer at `pointer` and checks it against
    /// the picture saved in `reference`; the first call saves it.
    fn hovered_heading_matches(
        row: DataRow<LinkMessage>,
        pointer: iced::Point,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let tree: Element<'_, LinkMessage> =
            container(DataTree::new(&palette).group("General", vec![row]))
                .width(400)
                .into();
        let mut ui = simulator(tree);
        ui.point_at(pointer);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved { position: pointer },
        )]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("tree should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn given_a_heading_link_then_it_draws_in_the_link_color_and_underlines_under_the_pointer()
     {
        let folder = TempDir::new("data-tree");
        let reference = folder.path().join("heading");
        let heading = || DataRow::heading_link(0, "net", open("network"));
        let palette = UiPalette::otty_dark();
        let tree: Element<'_, LinkMessage> = container(
            DataTree::new(&palette).group("General", vec![heading()]),
        )
        .width(400)
        .into();
        let mut ui = simulator(tree);
        let on_link = ui.find("net").expect("heading").bounds().center();
        let away = iced::Point::new(390.0, 2.0);

        assert!(hovered_heading_matches(heading(), away, &reference));
        assert!(!hovered_heading_matches(heading(), on_link, &reference));
        assert!(!hovered_heading_matches(
            DataRow::heading(0, "net"),
            away,
            &reference
        ));
    }

    #[test]
    fn given_rows_with_links_when_mapped_then_only_the_messages_change() {
        let parts = |first: String, second: String| {
            [
                ValuePart::link("a", first),
                ValuePart::text(", "),
                ValuePart::link("b", second),
            ]
        };
        let rows = [
            DataRow::with_parts(
                1,
                "Ports",
                parts(String::from("x"), String::from("y")),
            ),
            DataRow::heading_link(2, "net", String::from("z")),
            DataRow::heading(3, "Plain"),
            DataRow::new(4, "Id", "850b"),
        ];

        let mapped: Vec<DataRow<String>> = rows
            .into_iter()
            .map(|row| row.map(|message| format!("mapped {message}")))
            .collect();

        assert_eq!(
            mapped,
            vec![
                DataRow::with_parts(
                    1,
                    "Ports",
                    parts(String::from("mapped x"), String::from("mapped y")),
                ),
                DataRow::heading_link(2, "net", String::from("mapped z")),
                DataRow::heading(3, "Plain"),
                DataRow::new(4, "Id", "850b"),
            ]
        );
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
