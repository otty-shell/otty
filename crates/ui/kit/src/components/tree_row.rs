//! `TreeRow`: one row of a folder or session tree.

use iced::widget::text::Alignment as TextAlignment;
use iced::widget::{Row, Space, button, container, responsive, row, svg};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use super::ellipsis_text::{ELLIPSIS, EllipsisText, text_width};
use super::glyph;
use super::letter_icon::letter_icon;
use super::session_row::SessionTone;
use super::tag::{tag, tag_width};
use super::target_dot::{TargetKind, target_dot};
use super::tooltip::{TooltipText, tooltip};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Edge of the caret glyph.
const CARET_SIZE: f32 = 12.0;
/// Width of one column before the label: the caret, an icon or a dot.
const MARK_WIDTH: f32 = 14.0;
/// Gap between caret, label and tag (`.ot-tree { gap: 7px }`).
const GAP: f32 = 7.0;
/// The least room between the label, or the tag after it, and the note on
/// the right, at any width.
const NOTE_GAP: f32 = spacing::SPACE_3;
/// Horizontal padding of the row (`padding: 0 8px`).
const PADDING_X: f32 = spacing::SPACE_3;

/// Row of a tree. A row without a message is shown static.
///
/// Before the label the row has columns of one width: the caret, the icon
/// and the dot, whichever it has. So a folder with a caret and a file with an
/// icon of one level start their labels together. A row with none of them
/// starts its label where the caret would be. Each level moves the row right
/// by one indent, so whatever a row starts with stands one indent to the
/// right of the caret of its folder.
///
/// The label comes first: it keeps its width and ends with "…" only when it
/// does not fit. A tag, such as `OTTY`, follows the label. The state word and
/// count on the right give way first, ending with "…" or nothing when little
/// room is left. A row that [keeps its note](TreeRow::keep_note) turns this
/// around: the note stays whole at the right and the label gives way, with
/// its [whole label](TreeRow::whole_label) in a tooltip when it is cut.
/// Either way the label, or its tag, ends at least `SPACE_3` before the
/// note: the label is cut earlier to leave that gap, and nothing draws over
/// its neighbor.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::tree_row::TreeRow;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Toggle,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let folder: Element<'_, Message> = TreeRow::new("Servers", &palette)
///     .depth(1)
///     .caret(true)
///     .count(4)
///     .on_press(Message::Toggle)
///     .into();
/// ```
#[derive(Clone)]
pub struct TreeRow<Message> {
    label: String,
    palette: UiPalette,
    depth: u16,
    caret: Option<bool>,
    count: Option<usize>,
    count_tone: SessionTone,
    is_selected: bool,
    is_hovered: bool,
    is_drop_target: bool,
    icon: Option<(svg::Handle, Option<Color>)>,
    letter: Option<String>,
    dot: Option<TargetKind>,
    tag: Option<String>,
    state: Option<String>,
    keeps_note: bool,
    whole_label: Option<TooltipText>,
    on_press: Option<Message>,
}

impl<Message> TreeRow<Message> {
    /// Creates a root-level row without caret and count.
    pub fn new(label: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            label: label.into(),
            palette: palette.clone(),
            depth: 0,
            caret: None,
            count: None,
            count_tone: SessionTone::Muted,
            is_selected: false,
            is_hovered: false,
            is_drop_target: false,
            icon: None,
            letter: None,
            dot: None,
            tag: None,
            state: None,
            keeps_note: false,
            whole_label: None,
            on_press: None,
        }
    }

    /// Sets the nesting level. Each level indents the row by
    /// [`size::TREE_INDENT`].
    pub fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// Shows a caret for a row that can open; `is_open` picks its direction.
    pub fn caret(mut self, is_open: bool) -> Self {
        self.caret = Some(is_open);
        self
    }

    /// Shows the number of children on the right.
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Colors the count and the state word on the right, for example a
    /// failing count in `status-error`. The default is
    /// [`SessionTone::Muted`].
    pub fn count_tone(mut self, tone: SessionTone) -> Self {
        self.count_tone = tone;
        self
    }

    /// Marks the row as the selected one.
    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    /// Shows the hover background. Use it for a row without a message whose
    /// hover is tracked by the caller.
    pub fn hovered(mut self, is_hovered: bool) -> Self {
        self.is_hovered = is_hovered;
        self
    }

    /// Marks the row as the target of a drag and drop (`accent` background).
    pub fn drop_target(mut self, is_drop_target: bool) -> Self {
        self.is_drop_target = is_drop_target;
        self
    }

    /// Shows an icon before the label, such as the logo of a tool; `color`
    /// defaults to `text-muted`, and to `text-strong` on the selected row.
    /// In a row without a caret the icon stands where the caret would.
    pub fn icon(mut self, icon: svg::Handle, color: Option<Color>) -> Self {
        self.icon = Some((icon, color));
        self
    }

    /// Shows the first letter of `name` on a rounded square before the
    /// label, in place of an icon, for something with no icon of its own
    /// ([`letter_icon`](super::letter_icon::letter_icon)). It takes the
    /// colors an icon with no color of its own takes.
    pub fn letter_icon(mut self, name: impl Into<String>) -> Self {
        self.letter = Some(name.into());
        self
    }

    /// Shows a target dot before the label, for a row that starts a session
    /// on that target. The label next to it tells the target apart. In a row
    /// without a caret the dot stands where the caret would.
    pub fn dot(mut self, kind: TargetKind) -> Self {
        self.dot = Some(kind);
        self
    }

    /// Shows a short muted tag right after the label, such as `OTTY`.
    pub fn tag(mut self, label: impl Into<String>) -> Self {
        self.tag = Some(label.into());
        self
    }

    /// Shows a short muted word on the right, such as `running`.
    pub fn state(mut self, word: impl Into<String>) -> Self {
        self.state = Some(word.into());
        self
    }

    /// Keeps the note on the right whole: the state word and the count keep
    /// their width at the right edge, and the label takes what is left,
    /// ending with "…" when that is too little. A tag stays whole after the
    /// label. For a row whose number matters more than its name, which can
    /// still be found by a search or a press.
    pub fn keep_note(mut self) -> Self {
        self.keeps_note = true;
        self
    }

    /// Shows `label` in the kit tooltip while the pointer is over the row
    /// and its label is cut, such as the whole name of an account with its
    /// provider. A label that fits shows no tooltip.
    pub fn whole_label(mut self, label: impl Into<TooltipText>) -> Self {
        self.whole_label = Some(label.into());
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The note on the right: the state word, the count, or both.
    fn note(&self) -> Option<String> {
        match (&self.state, self.count) {
            (Some(word), Some(count)) => Some(format!("{word} {count}")),
            (Some(word), None) => Some(word.clone()),
            (None, Some(count)) => Some(count.to_string()),
            (None, None) => None,
        }
    }

    /// Width the tag takes after the label, with the gap before it.
    fn tag_room(&self) -> f32 {
        self.tag.as_deref().map_or(0.0, |tag| GAP + tag_width(tag))
    }

    /// Width the row keeps after the label: the tag with its gap, then the
    /// gap before the note and the note, whole when the row keeps it, or
    /// only its "…" when the note gives way.
    fn room_after_label(&self) -> f32 {
        let note = self.note().map_or(0.0, |note| {
            let shown = if self.keeps_note { &note } else { ELLIPSIS };

            NOTE_GAP + text_width(shown, typography::CAPTION.size(), fonts::UI)
        });

        self.tag_room() + note
    }

    /// Whether the row, `width` wide, draws its label cut. It counts the
    /// room the layout of [`TreeRow::row`] leaves the label: the padding,
    /// the indent and the columns before it, and the room after it.
    fn cuts_label(&self, width: f32) -> bool {
        let marks = [
            self.caret.is_some(),
            self.icon.is_some(),
            self.letter.is_some(),
            self.dot.is_some(),
        ]
        .into_iter()
        .filter(|has| *has)
        .count() as f32;

        let room = width
            - 2.0 * PADDING_X
            - f32::from(self.depth) * size::TREE_INDENT
            - marks * (MARK_WIDTH + GAP)
            - self.room_after_label();

        text_width(&self.label, typography::BODY.size(), fonts::UI) > room
    }

    /// The row itself, without the tooltip of its whole label.
    fn row<'a>(self) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        let value = self;
        let note = value.note();
        let keeps_note = value.keeps_note;
        // A row that keeps its note lays the note out first, so its label
        // leaves only the tag free; otherwise the label leaves all the room
        // after it.
        let label_reserve = if keeps_note {
            value.tag_room()
        } else {
            value.room_after_label()
        };
        let palette = value.palette;
        let is_selected = value.is_selected;
        let is_hovered = value.is_hovered;
        let is_drop_target = value.is_drop_target;
        let body = typography::BODY;

        let muted = palette.text_muted();
        let mut marks: Vec<Element<'a, Message>> = Vec::new();
        if let Some(is_open) = value.caret {
            let caret = svg(glyph::caret(is_open))
                .width(CARET_SIZE)
                .height(CARET_SIZE)
                .style(move |_, _| svg::Style { color: Some(muted) });
            let caret = container(caret)
                .width(MARK_WIDTH)
                .align_x(Alignment::Center);
            marks.push(caret.into());
        }
        if let Some((handle, color)) = value.icon {
            // The icon follows the label: brighter on the selected row.
            let default_color = if is_selected {
                palette.text_strong()
            } else {
                muted
            };
            let color = color.unwrap_or(default_color);
            let icon = svg(handle)
                .width(MARK_WIDTH)
                .height(MARK_WIDTH)
                .style(move |_, _| svg::Style { color: Some(color) });
            marks.push(icon.into());
        }
        if let Some(name) = value.letter {
            let color = if is_selected {
                palette.text_strong()
            } else {
                muted
            };
            marks.push(letter_icon(&name, MARK_WIDTH, color, &palette));
        }
        if let Some(kind) = value.dot {
            // The dot is half as wide as its column. It stands at the right
            // edge, one gap before the label, as in a session row.
            let dot = container(target_dot(kind, &palette))
                .width(MARK_WIDTH)
                .align_x(Alignment::End);
            marks.push(dot.into());
        }

        let indent =
            Space::new().width(f32::from(value.depth) * size::TREE_INDENT);
        let mut content = Row::with_children(marks)
            .spacing(GAP)
            .height(Length::Fill)
            .align_y(Alignment::Center);
        let label_color = if is_drop_target {
            palette.on_accent()
        } else if is_selected {
            palette.text_strong()
        } else {
            palette.text_body()
        };
        let label =
            EllipsisText::new(value.label, body.size(), fonts::UI, label_color);
        let note = note.map(|note| {
            EllipsisText::new(
                note,
                typography::CAPTION.size(),
                fonts::UI,
                value.count_tone.color(&palette),
            )
        });

        // The label and its tag. The label leaves the room it reserves
        // free, so the tag stays whole and the note keeps its gap.
        let mut named = Row::new()
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .push(clipped(label.shrink(label_reserve)));
        if let Some(label) = value.tag {
            named = named.push(Space::new().width(GAP)).push(tag(
                label,
                size::ROW,
                &palette,
            ));
        }

        // A kept note keeps its width at the right and the label and tag
        // take what is left. Otherwise the label takes the room it needs
        // first, and the note takes what is left, ending with "…".
        let mut body = Row::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center);
        if keeps_note {
            body = body.push(clipped(named.width(Length::Fill)));
        } else {
            body = body.push(clipped(named));
        }
        if let Some(note) = note {
            let note = if keeps_note {
                note.shrink(0.0)
            } else {
                note.align_x(TextAlignment::Right)
            };
            body = body.push(Space::new().width(NOTE_GAP)).push(clipped(note));
        }
        content = content.push(body);

        button(row![indent, content])
            .width(Length::Fill)
            .height(size::ROW)
            .padding(Padding::from([0.0, PADDING_X]))
            .on_press_maybe(value.on_press)
            .style(move |_, status| {
                let hovered = is_hovered || status == button::Status::Hovered;

                button::Style {
                    background: if is_drop_target {
                        Some(palette.accent().into())
                    } else if is_selected {
                        Some(palette.surface_selected().into())
                    } else {
                        hovered.then(|| palette.surface_raised().into())
                    },
                    text_color: if is_drop_target {
                        palette.on_accent()
                    } else if is_selected {
                        palette.text_strong()
                    } else {
                        palette.text_body()
                    },
                    border: Border {
                        radius: radius::RADIUS_SM.into(),
                        ..Border::default()
                    },
                    ..button::Style::default()
                }
            })
            .into()
    }
}

impl<'a, Message: Clone + 'a> From<TreeRow<Message>> for Element<'a, Message> {
    fn from(mut value: TreeRow<Message>) -> Self {
        let Some(whole) = value.whole_label.take() else {
            return value.row();
        };
        let palette = value.palette.clone();

        // Whether the label is cut depends on the width of the row, known
        // only during layout.
        responsive(move |space| {
            let row = value.clone();
            if row.cuts_label(space.width) {
                tooltip(row.row(), whole.clone(), &palette)
            } else {
                row.row()
            }
        })
        .height(size::ROW)
        .into()
    }
}

/// `content` drawn only inside its own bounds, so a "…" or a tag pressed
/// into too little room never draws over its neighbor.
fn clipped<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(content).clip(true).into()
}

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced_test::selector::Candidate;
    use iced_test::simulator;

    use super::*;
    use crate::testing::pictures::draws;
    use crate::testing::temp_dir::TempDir;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Open,
    }

    /// A window with room for a row 260 px wide.
    const WINDOW: iced::Size = iced::Size::new(340.0, 120.0);

    /// Draws a "Pods" row with a count of 2 and the state word "failing" in
    /// `tone` (the default when `None`), and checks it against the picture
    /// saved in `reference`; the first call saves the picture.
    fn drawn_row_matches(
        tone: Option<SessionTone>,
        reference: &std::path::Path,
    ) -> bool {
        let view = move || {
            let palette = UiPalette::otty_dark();
            let mut row =
                TreeRow::new("Pods", &palette).count(2).state("failing");
            if let Some(tone) = tone {
                row = row.count_tone(tone);
            }

            container(row).width(260).into()
        };

        draws(&view, WINDOW, None, reference)
    }

    #[test]
    fn given_count_tone_when_row_drawn_then_count_takes_the_tone_color() {
        let folder = TempDir::new("tree-row");
        let reference = folder.path().join("count-tone");

        assert!(drawn_row_matches(None, &reference));
        assert!(drawn_row_matches(Some(SessionTone::Muted), &reference));
        assert!(!drawn_row_matches(Some(SessionTone::Failed), &reference));
        assert!(!drawn_row_matches(Some(SessionTone::Done), &reference));
    }

    /// Draws a "Redis" row with an icon in `color` (the default when
    /// `None`), selected or not, and checks it against the picture saved in
    /// `reference`; the first call saves the picture.
    fn drawn_icon_matches(
        color: Option<Color>,
        is_selected: bool,
        reference: &std::path::Path,
    ) -> bool {
        let view = move || {
            let palette = UiPalette::otty_dark();
            let row = TreeRow::new("Redis", &palette)
                .icon(glyph::plus(), color)
                .selected(is_selected);

            container(row).width(260).into()
        };

        draws(&view, WINDOW, None, reference)
    }

    #[test]
    fn given_an_icon_without_a_color_then_it_is_muted_and_strong_on_the_selected_row()
     {
        let palette = UiPalette::otty_dark();
        let muted = Some(palette.text_muted());
        let strong = Some(palette.text_strong());
        let folder = TempDir::new("tree-row");
        let idle = folder.path().join("icon-idle");
        let selected = folder.path().join("icon-selected");

        assert!(drawn_icon_matches(muted, false, &idle));
        assert!(drawn_icon_matches(None, false, &idle));
        assert!(drawn_icon_matches(strong, true, &selected));
        assert!(drawn_icon_matches(None, true, &selected));
        assert!(!drawn_icon_matches(muted, true, &selected));
    }

    #[test]
    fn given_an_icon_with_a_color_then_the_selected_row_keeps_that_color() {
        let palette = UiPalette::otty_dark();
        let error = Some(palette.status_error());
        let folder = TempDir::new("tree-row");
        let reference = folder.path().join("icon-color");

        assert!(drawn_icon_matches(error, true, &reference));
        assert!(!drawn_icon_matches(None, true, &reference));
    }

    #[test]
    fn given_a_narrow_row_with_an_icon_then_the_label_keeps_its_width_and_the_note_gives_way()
     {
        let palette = UiPalette::otty_dark();
        let widths = |width: f32| {
            let row = TreeRow::<Message>::new("Postgres 16", &palette)
                .icon(glyph::plus(), None)
                .state("needs you")
                .count_tone(SessionTone::Waiting);
            let mut ui = simulator(container(row).width(width));
            let label = ui.find("Postgres 16").expect("label").bounds().width;
            let note = ui.find("needs you").expect("note").bounds().width;

            (label, note)
        };
        let (full_label, full_note) = widths(400.0);

        let (label, note) = widths(150.0);

        assert!(
            (label - full_label).abs() < 0.5,
            "{label} against {full_label}"
        );
        assert!(note < full_note, "{note} against {full_note}");
    }

    /// A tag stands right after the label, before the note on the right,
    /// and the label keeps its width.
    #[test]
    fn given_a_tag_then_it_follows_the_label_before_the_note() {
        let palette = UiPalette::otty_dark();
        let row = TreeRow::<Message>::new("you@example.com", &palette)
            .tag("OTTY")
            .state("42 % used");
        let mut ui = simulator(container(row).width(400));

        let label = ui.find("you@example.com").expect("label").bounds();
        let tag = ui.find("OTTY").expect("the tag shows").bounds();
        let note = ui.find("42 % used").expect("note").bounds();

        assert!(tag.x >= label.x + label.width, "{tag:?} after {label:?}");
        assert!(note.x >= tag.x + tag.width, "{note:?} after {tag:?}");
    }

    /// A "Containers" row with the note "running" and a count of 2, in a
    /// space `width` wide.
    fn containers_row(width: f32) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();

        container(
            TreeRow::new("Containers", &palette)
                .caret(true)
                .count(2)
                .state("running"),
        )
        .width(width)
        .into()
    }

    /// Width of the label and of the note and count of the row.
    fn label_and_note_widths(width: f32) -> (f32, f32) {
        let mut ui = simulator(containers_row(width));
        let label = ui.find("Containers").expect("label").bounds().width;
        let note = ui.find("running 2").expect("note").bounds().width;

        (label, note)
    }

    #[test]
    fn given_narrow_row_when_laid_out_then_the_label_keeps_its_width_and_the_note_gives_way()
     {
        let (full_label, full_note) = label_and_note_widths(400.0);

        let (label, note) = label_and_note_widths(150.0);

        assert!(
            (label - full_label).abs() < 0.5,
            "the label should keep its width: {label} against {full_label}"
        );
        assert!(
            note < full_note,
            "the note should shrink: {note} against {full_note}"
        );
    }

    #[test]
    fn given_very_narrow_row_when_laid_out_then_the_label_ends_with_an_ellipsis()
     {
        let (full_label, _) = label_and_note_widths(400.0);

        let (label, note) = label_and_note_widths(90.0);

        assert!(
            label < full_label,
            "the label should be cut: {label} against {full_label}"
        );
        assert!(note < 20.0, "the note should be nearly gone: {note}");
    }

    /// An account row that keeps its note: a logo, a long email, the tag
    /// "OTTY" and the note "42 % used".
    fn account_row(label: &str) -> TreeRow<Message> {
        TreeRow::new(label, &UiPalette::otty_dark())
            .icon(glyph::plus(), None)
            .tag("OTTY")
            .state("42 % used")
            .keep_note()
    }

    /// Bounds of the label, the tag and the note of `row` in a space
    /// `width` wide.
    fn account_parts(
        row: TreeRow<Message>,
        label: &str,
        width: f32,
    ) -> (iced::Rectangle, iced::Rectangle, iced::Rectangle) {
        let mut ui = simulator(container(row).width(width));
        let mut at =
            |text: &str| ui.find(text).expect("the text shows").bounds();

        (at(label), at("OTTY"), at("42 % used"))
    }

    /// Issue 20 of spec 2026-11-23-accounts-apart-from-profiles: a row
    /// that keeps its note draws the whole note at the right; the label
    /// gives way and the tag stays whole after it.
    #[test]
    fn given_a_narrow_row_that_keeps_its_note_then_the_label_gives_way() {
        let email = "someone.with.a.long.name@example.com";
        let width = 220.0;
        let (full_label, full_tag, full_note) =
            account_parts(account_row(email), email, 600.0);

        let (label, tag, note) =
            account_parts(account_row(email), email, width);

        assert!(label.width < full_label.width, "{label:?} {full_label:?}");
        assert!((tag.width - full_tag.width).abs() < 0.5, "{tag:?}");
        assert!((note.width - full_note.width).abs() < 0.5, "{note:?}");
        assert!(tag.x >= label.x + label.width, "{tag:?} after {label:?}");
        assert!(note.x >= tag.x + tag.width, "{note:?} after {tag:?}");
        assert!(note.x + note.width <= width - PADDING_X + 0.5, "{note:?}");
    }

    #[test]
    fn given_a_row_that_keeps_its_note_and_fits_then_both_are_whole() {
        let email = "you@example.com";
        let (full_label, _, full_note) =
            account_parts(account_row(email), email, 600.0);

        let (label, tag, note) =
            account_parts(account_row(email), email, 320.0);

        assert!((label.width - full_label.width).abs() < 0.5, "{label:?}");
        assert!((note.width - full_note.width).abs() < 0.5, "{note:?}");
        assert!(tag.x >= label.x + label.width, "the tag follows the label");
        assert!(
            note.x > tag.x + tag.width + GAP,
            "the note stands at the right: {note:?} {tag:?}"
        );
    }

    /// The box of the tag whose text is drawn in `text`: the text with the
    /// tag's own padding at its left and its right.
    fn tag_box(text: Rectangle, label: &str) -> Rectangle {
        let padding = (tag_width(label)
            - text_width(label, typography::CAPTION.size(), fonts::UI))
            / 2.0;

        Rectangle {
            x: text.x - padding,
            width: text.width + 2.0 * padding,
            ..text
        }
    }

    /// Checks a row `width` wide: the label and the tag that show end at
    /// least `SPACE_3` before the note that shows, the label ends before
    /// the tag, and the note ends inside the row. A part with no width
    /// shows nothing. `tag` is the drawn text of the tag `tag_label`.
    fn assert_name_keeps_a_gap(
        label: Rectangle,
        tag: Option<(Rectangle, &str)>,
        note: Rectangle,
        width: f32,
    ) {
        let shows = |part: &Rectangle| part.width > 0.0;
        let tag = tag
            .filter(|(text, _)| shows(text))
            .map(|(text, tag_label)| tag_box(text, tag_label));
        let label_end = label.x + label.width;

        if shows(&label)
            && let Some(tag) = tag
        {
            assert!(
                label_end <= tag.x + 0.01,
                "width {width}: the label {label:?} runs into the tag {tag:?}"
            );
        }
        if !shows(&note) {
            return;
        }
        assert!(
            note.x + note.width <= width - PADDING_X + 0.5,
            "width {width}: the note {note:?} leaves the row"
        );
        if shows(&label) {
            assert!(
                label_end + spacing::SPACE_3 <= note.x + 0.01,
                "width {width}: the label {label:?} is too close to the note {note:?}"
            );
        }
        if let Some(tag) = tag {
            assert!(
                tag.x + tag.width + spacing::SPACE_3 <= note.x + 0.01,
                "width {width}: the tag {tag:?} is too close to the note {note:?}"
            );
        }
    }

    /// Issue 23 of spec 2026-11-23-accounts-apart-from-profiles: at any
    /// width, the label and the tag of a row that keeps its note end at
    /// least `SPACE_3` before the note, and nothing overlaps.
    #[test]
    fn given_any_width_when_the_row_keeps_its_note_then_the_name_ends_a_gap_before_it()
     {
        let email = "someone.with.a.long.name@example.com";

        for width in (40..=400).step_by(4).map(|width| width as f32) {
            let (label, tag, note) =
                account_parts(account_row(email), email, width);

            assert_name_keeps_a_gap(label, Some((tag, "OTTY")), note, width);
        }
    }

    /// Issue 23: a row whose note gives way, such as a folder of the Files
    /// tree, keeps the same gap at any width, with a tag or without.
    #[test]
    fn given_any_width_when_the_note_gives_way_then_the_name_ends_a_gap_before_it()
     {
        let palette = UiPalette::otty_dark();
        let email = "you@example.com";

        for width in (40..=400).step_by(4).map(|width| width as f32) {
            let mut ui = simulator(containers_row(width));
            let label = ui.find("Containers").expect("label").bounds();
            let note = ui.find("running 2").expect("note").bounds();

            assert_name_keeps_a_gap(label, None, note, width);

            let row = TreeRow::<Message>::new(email, &palette)
                .tag("OTTY")
                .state("42 % used");
            let mut ui = simulator(container(row).width(width));
            let label = ui.find(email).expect("label").bounds();
            let tag = ui.find("OTTY").expect("tag").bounds();
            let note = ui.find("42 % used").expect("note").bounds();

            assert_name_keeps_a_gap(label, Some((tag, "OTTY")), note, width);
        }
    }

    /// Whether the label of `row` is drawn cut in a space `width` wide.
    fn drawn_cut(row: TreeRow<Message>, label: &str, width: f32) -> bool {
        let full = simulator(container(TreeRow::<Message>::new(
            label,
            &UiPalette::otty_dark(),
        )))
        .find(label)
        .expect("label")
        .bounds()
        .width;
        let mut ui = simulator(container(row).width(width));

        ui.find(label).expect("label").bounds().width < full - 0.5
    }

    /// The row tells a cut label, for its tooltip, by the same rule its
    /// layout cuts it, whether it keeps its note or not, at any width: it
    /// counts the gap before the note too (issue 23).
    #[test]
    fn given_any_width_then_the_row_knows_whether_its_label_is_cut() {
        let palette = UiPalette::otty_dark();
        let email = "someone@example.com";
        let rows = |keeps_note: bool| {
            let row = TreeRow::<Message>::new(email, &palette)
                .depth(1)
                .caret(true)
                .icon(glyph::plus(), None)
                .tag("OTTY")
                .state("42 % used")
                .count(3);
            if keeps_note { row.keep_note() } else { row }
        };

        for keeps_note in [false, true] {
            for width in (40..=400).step_by(4).map(|width| width as f32) {
                let is_cut = rows(keeps_note).cuts_label(width);

                assert_eq!(
                    is_cut,
                    drawn_cut(rows(keeps_note), email, width),
                    "keeps the note: {keeps_note}, width {width}"
                );
            }
        }
    }

    /// Draws a row `width` wide with the label `label` and the whole label
    /// `whole`, with the pointer over it or not, and checks it against the
    /// picture saved in `reference`; the first call saves the picture.
    fn drawn_whole_label_matches(
        label: &'static str,
        whole: &'static str,
        is_hovered: bool,
        reference: &std::path::Path,
    ) -> bool {
        let view = move || {
            let palette = UiPalette::otty_dark();
            let row = TreeRow::new(label, &palette)
                .state("42 % used")
                .keep_note()
                .whole_label(whole);

            container(row).width(160).into()
        };
        let pointer = is_hovered.then(|| iced::Point::new(20.0, 10.0));

        draws(&view, WINDOW, pointer, reference)
    }

    /// A cut label shows its whole label in the kit tooltip while the
    /// pointer is over the row; a label that fits shows none.
    #[test]
    fn given_a_cut_label_when_hovered_then_the_tooltip_shows_the_whole_label() {
        let long = "someone.with.a.long.name@example.com";
        let folder = TempDir::new("tree-row-whole-label");
        let cut = folder.path().join("cut");
        let cut_idle = folder.path().join("cut-idle");
        let short = folder.path().join("short");

        assert!(drawn_whole_label_matches(long, "OpenAI (a)", true, &cut));
        assert!(
            !drawn_whole_label_matches(long, "OpenAI (b)", true, &cut),
            "the tooltip of a cut label shows"
        );
        assert!(drawn_whole_label_matches(
            long,
            "OpenAI (a)",
            false,
            &cut_idle
        ));
        assert!(
            drawn_whole_label_matches(long, "OpenAI (b)", false, &cut_idle),
            "no tooltip while the pointer is away"
        );
        assert!(drawn_whole_label_matches("you", "OpenAI (a)", true, &short));
        assert!(
            drawn_whole_label_matches("you", "OpenAI (b)", true, &short),
            "a label that fits shows no tooltip"
        );
    }

    /// Left edge of the label of `row`.
    fn label_x(row: TreeRow<Message>, label: &str) -> f32 {
        let mut ui = simulator(container(row).width(260));

        ui.find(label).expect("label should be visible").bounds().x
    }

    #[test]
    fn given_rows_of_one_level_when_laid_out_then_their_labels_start_together()
    {
        let palette = UiPalette::otty_dark();

        let folder = label_x(TreeRow::new("src", &palette).caret(true), "src");
        let file = label_x(
            TreeRow::new("Cargo.toml", &palette).icon(glyph::plus(), None),
            "Cargo.toml",
        );
        let command = label_x(
            TreeRow::new("deploy", &palette).dot(TargetKind::Ssh),
            "deploy",
        );

        assert_eq!(file, folder, "a file should start where a folder does");
        assert_eq!(command, folder, "a dot should take the caret column");
    }

    #[test]
    fn given_a_bare_row_when_laid_out_then_its_label_starts_where_a_caret_would()
     {
        let palette = UiPalette::otty_dark();
        // Left edge of the caret column of a root folder.
        let caret = label_x(TreeRow::new("src", &palette).caret(true), "src")
            - MARK_WIDTH
            - GAP;

        let root = label_x(TreeRow::new("Images", &palette), "Images");
        let nested =
            label_x(TreeRow::new("Volumes", &palette).depth(1), "Volumes");

        assert_eq!(root, caret);
        assert_eq!(nested, caret + size::TREE_INDENT);
    }

    #[test]
    fn given_a_deeper_row_when_laid_out_then_its_label_moves_by_one_indent() {
        let palette = UiPalette::otty_dark();

        let parent = label_x(TreeRow::new("src", &palette).caret(true), "src");
        let child = label_x(
            TreeRow::new("main.rs", &palette)
                .depth(1)
                .icon(glyph::plus(), None),
            "main.rs",
        );

        assert_eq!(child, parent + size::TREE_INDENT);
    }

    #[test]
    fn given_a_dot_when_laid_out_then_it_stands_one_gap_before_the_label() {
        let palette = UiPalette::otty_dark();
        let mut dot_right = None;
        let mut ui = simulator(
            container(
                TreeRow::<Message>::new("deploy", &palette)
                    .dot(TargetKind::Ssh),
            )
            .width(260),
        );

        // The selector never matches, so it is shown every widget of the row.
        let _ = ui.find(|candidate: Candidate<'_>| {
            if let Candidate::Container { bounds, .. } = candidate
                && bounds.width == size::DOT
            {
                dot_right = Some(bounds.x + bounds.width);
            }
            None::<()>
        });
        let label = ui.find("deploy").expect("label").bounds().x;

        assert_eq!(dot_right.map(|right| label - right), Some(GAP));
    }

    #[test]
    fn given_row_with_message_when_pressed_then_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            container(
                TreeRow::new("Containers", &palette)
                    .depth(1)
                    .caret(false)
                    .icon(glyph::plus(), None)
                    .dot(TargetKind::Local)
                    .count(3)
                    .count_tone(SessionTone::Waiting)
                    .on_press(Message::Open),
            )
            .width(260),
        );

        ui.click("Containers").expect("label should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Open]);
    }
}
