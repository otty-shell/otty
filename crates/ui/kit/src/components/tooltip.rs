//! `tooltip`: the kit's tooltip bubble over any content, such as a short
//! text whose whole value is too long to show, an icon button or a bar of
//! a chart. Every control of the kit that shows a tooltip shows this one.
//! What it says is a [`TooltipText`]: lines of text, and rows that name
//! the parts of a number with each part's number at the right.

use iced::widget::{Column, Space, container, row, tooltip as iced_tooltip};
use iced::{Alignment, Border, Element, Padding};

use super::small_text::small_text;
use crate::palette::UiPalette;
use crate::{radius, spacing};

/// Widest the bubble grows; a longer label wraps onto more lines.
const TOOLTIP_MAX_WIDTH: f32 = 360.0;

/// What a tooltip bubble says: its lines of text, then groups of rows,
/// each row a name at the left and its number at the right, such as the
/// kinds of tokens a total is made of. A group stands apart from the one
/// before it, so the parts of two numbers do not run together.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::tooltip::TooltipText;
///
/// let tip = TooltipText::new("Nov 13: 120K tokens").rows(vec![
///     (String::from("in"), String::from("20K")),
///     (String::from("cached"), String::from("100K")),
/// ]);
///
/// assert!(!tip.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TooltipText {
    lines: Vec<String>,
    groups: Vec<Vec<(String, String)>>,
}

impl TooltipText {
    /// Whether it says nothing: no line and no row.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.groups.is_empty()
    }

    /// A tooltip of the one line `line`.
    pub fn new(line: impl Into<String>) -> Self {
        Self {
            lines: vec![line.into()],
            groups: Vec::new(),
        }
    }

    /// Adds a group of `rows` under what it says, one row per line: each a
    /// name and its number. No rows add nothing.
    pub fn rows(mut self, rows: Vec<(String, String)>) -> Self {
        if !rows.is_empty() {
            self.groups.push(rows);
        }
        self
    }
}

impl From<String> for TooltipText {
    fn from(line: String) -> Self {
        Self::new(line)
    }
}

impl From<&str> for TooltipText {
    fn from(line: &str) -> Self {
        Self::new(line)
    }
}

impl From<&String> for TooltipText {
    fn from(line: &String) -> Self {
        Self::new(line.as_str())
    }
}

/// Shows `label` in the kit's tooltip bubble under `content` while the
/// pointer is over it, as the icon buttons show theirs. A long label wraps.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::tooltip::tooltip;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let path: Element<'_, ()> = tooltip(
///     text("~/…/ottyrun/otty"),
///     "/home/user/projects/main/ottyrun/otty",
///     &palette,
/// );
/// ```
pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: impl Into<TooltipText>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    placed(
        content,
        label.into(),
        iced_tooltip::Position::Bottom,
        palette,
    )
}

/// Shows `label` in the kit's tooltip bubble over `content`, for content
/// near the bottom of its place, such as the bars of a chart.
pub(super) fn tooltip_above<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: TooltipText,
    palette: &UiPalette,
) -> Element<'a, Message> {
    placed(content, label, iced_tooltip::Position::Top, palette)
}

/// Shows the bubble of `label` at `position` of `content`, a small gap
/// away from it.
fn placed<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: TooltipText,
    position: iced_tooltip::Position,
    palette: &UiPalette,
) -> Element<'a, Message> {
    iced_tooltip(content, bubble(label, palette), position)
        .gap(spacing::SPACE_1)
        .into()
}

/// The bubble: `label` in small text on the overlay surface, no wider than
/// [`TOOLTIP_MAX_WIDTH`]: its lines, then each group of rows.
fn bubble<'a, Message: 'a>(
    label: TooltipText,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let background = palette.surface_overlay();
    let color = palette.text_body();
    let shadow = palette.shadow_popover();

    let mut said = Column::new().spacing(spacing::SPACE_1);
    for line in label.lines {
        said = said.push(small_text(line, color));
    }
    if !label.groups.is_empty() {
        said = said.push(rows_view(label.groups, color));
    }

    container(said)
        .max_width(TOOLTIP_MAX_WIDTH)
        .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3]))
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_XS.into(),
                ..Border::default()
            },
            shadow,
            ..container::Style::default()
        })
        .into()
}

/// The rows of every group in one grid, one row per line: the names in a
/// column at the left, the numbers in a column at the right, so every
/// name starts and every number ends where the others of all groups do,
/// and a little space between groups. The grid is as wide as its widest
/// row: a wider line above does not stretch it, since a filling row would
/// stretch the bubble to its widest.
fn rows_view<'a, Message: 'a>(
    groups: Vec<Vec<(String, String)>>,
    color: iced::Color,
) -> Element<'a, Message> {
    let mut names = Column::new();
    let mut numbers = Column::new().align_x(Alignment::End);
    for (index, rows) in groups.into_iter().enumerate() {
        if index > 0 {
            names = names.push(Space::new().height(spacing::SPACE_1));
            numbers = numbers.push(Space::new().height(spacing::SPACE_1));
        }
        for (name, number) in rows {
            names = names.push(small_text(name, color));
            numbers = numbers.push(small_text(number, color));
        }
    }

    row![names, numbers].spacing(spacing::SPACE_3).into()
}

#[cfg(test)]
mod tests {
    use iced::widget::text;
    use iced_test::simulator;

    use super::*;
    use crate::testing::pictures::{SMALL_WINDOW, draws};
    use crate::testing::temp_dir::TempDir;
    use crate::typography;

    /// A word as wide as the hover point below needs.
    fn hinted(label: &'static str) -> impl Fn() -> Element<'static, ()> {
        move || tooltip(text("Path"), label, &UiPalette::otty_dark())
    }

    #[test]
    fn given_pointer_over_the_content_then_its_label_shows_under_it() {
        let folder = TempDir::new("tooltip");
        let idle = folder.path().join("idle");
        let hovered = folder.path().join("hovered");
        let over = iced::Point::new(5.0, 8.0);

        assert!(
            draws(&hinted("/home/user/otty"), SMALL_WINDOW, None, &idle),
            "save"
        );
        assert!(
            draws(&hinted("/srv/otty"), SMALL_WINDOW, None, &idle),
            "no label away"
        );
        assert!(draws(
            &hinted("/home/user/otty"),
            SMALL_WINDOW,
            Some(over),
            &hovered
        ));
        assert!(
            !draws(&hinted("/srv/otty"), SMALL_WINDOW, Some(over), &hovered),
            "the label shows under the pointer"
        );
        assert!(
            !draws(&hinted("/home/user/otty"), SMALL_WINDOW, Some(over), &idle),
            "the bubble shows under the pointer"
        );
    }

    /// A long label wraps inside the widest bubble instead of running
    /// across the window.
    #[test]
    fn a_long_label_wraps_inside_the_widest_bubble() {
        let label = "The container stopped because its main process ended \
                     with exit code 137 after the engine sent it a kill \
                     signal";
        let hint: Element<'_, ()> =
            bubble(label.into(), &UiPalette::otty_dark());
        let mut ui = simulator(hint);

        let words = ui.find(label).expect("the label should show").bounds();

        assert!(words.width <= TOOLTIP_MAX_WIDTH, "{words:?}");
        assert!(words.height > 2.0 * typography::SMALL.size(), "{words:?}");
    }

    /// The rows of a tooltip, such as the kinds of tokens of a total, are
    /// one per line under its lines: each name at the left and its number
    /// at the right, the numbers ending together.
    #[test]
    fn rows_name_each_part_at_the_left_and_its_number_at_the_right() {
        let label = TooltipText::new("Nov 13: 120K tokens").rows(vec![
            (String::from("in"), String::from("20K")),
            (String::from("cached"), String::from("100K")),
        ]);
        let hint: Element<'_, ()> = bubble(label, &UiPalette::otty_dark());
        let mut ui = simulator(hint);
        let mut at =
            |text: &str| ui.find(text).expect("the text should show").bounds();

        let line = at("Nov 13: 120K tokens");
        let input = at("in");
        let cached = at("cached");
        let few = at("20K");
        let many = at("100K");

        assert!(input.y > line.y, "{input:?} under {line:?}");
        assert!(cached.y > input.y, "one row per line");
        assert!((input.y - few.y).abs() < 1.0, "{input:?} {few:?}");
        assert!(few.x > input.x + input.width, "the number at the right");
        assert!((input.x - cached.x).abs() < 1.0, "the names start together");
        assert!(
            (few.x + few.width - many.x - many.width).abs() < 1.0,
            "the numbers end together: {few:?} {many:?}"
        );
    }

    /// Rows of several groups, such as the tokens and the money of a bar,
    /// share one grid: every name starts at the left edge of the bubble and
    /// the numbers of all groups end together.
    #[test]
    fn rows_of_all_groups_share_one_grid() {
        let label =
            TooltipText::new("Nov 13: 120K tokens, ~$3.10 at API prices")
                .rows(vec![
                    (String::from("in"), String::from("20K")),
                    (String::from("cache write"), String::from("8K")),
                ])
                .rows(vec![(String::from("cached"), String::from("~$0.40"))]);
        let hint: Element<'_, ()> = bubble(label, &UiPalette::otty_dark());
        let mut ui = simulator(hint);
        let mut at =
            |text: &str| ui.find(text).expect("the text should show").bounds();

        let line = at("Nov 13: 120K tokens, ~$3.10 at API prices");
        let input = at("in");
        let written = at("cache write");
        let cached = at("cached");
        let few = at("20K");
        let some = at("8K");
        let money = at("~$0.40");
        let right = few.x + few.width;

        assert!((input.x - line.x).abs() < 1.0, "{input:?} {line:?}");
        assert!((written.x - line.x).abs() < 1.0, "{written:?}");
        assert!((cached.x - line.x).abs() < 1.0, "{cached:?}");
        assert!((some.x + some.width - right).abs() < 1.0, "{some:?}");
        assert!((money.x + money.width - right).abs() < 1.0, "{money:?}");
    }

    /// Text alone is a tooltip of one line with no rows.
    #[test]
    fn text_alone_is_a_tooltip_of_one_line() {
        let label = TooltipText::from("/srv/otty");

        assert!(!label.is_empty());
        assert!(TooltipText::default().is_empty());
        assert!(
            TooltipText::default().rows(Vec::new()).is_empty(),
            "no rows add nothing"
        );
    }
}
