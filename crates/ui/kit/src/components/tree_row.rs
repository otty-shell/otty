//! `TreeRow`: one row of a folder or session tree.

use iced::widget::text::Alignment as TextAlignment;
use iced::widget::{Row, Space, button, container, row, svg};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use super::ellipsis_text::EllipsisText;
use super::glyph;
use super::session_row::SessionTone;
use super::target_dot::{TargetKind, target_dot};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Edge of the caret glyph.
const CARET_SIZE: f32 = 12.0;
/// Width of one column before the label: the caret, an icon or a dot.
const MARK_WIDTH: f32 = 14.0;
/// Gap between caret, label and count (`.ot-tree { gap: 7px }`).
const GAP: f32 = 7.0;
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
/// does not fit. The state word and count on the right give way first, ending
/// with "…" or nothing when little room is left.
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
    dot: Option<TargetKind>,
    state: Option<String>,
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
            dot: None,
            state: None,
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

    /// Shows an icon before the label; `color` defaults to `text-muted`. In
    /// a row without a caret the icon stands where the caret would.
    pub fn icon(mut self, icon: svg::Handle, color: Option<Color>) -> Self {
        self.icon = Some((icon, color));
        self
    }

    /// Shows a target dot before the label, for a row that starts a session
    /// on that target. The label next to it tells the target apart. In a row
    /// without a caret the dot stands where the caret would.
    pub fn dot(mut self, kind: TargetKind) -> Self {
        self.dot = Some(kind);
        self
    }

    /// Shows a short muted word on the right, such as `running`.
    pub fn state(mut self, word: impl Into<String>) -> Self {
        self.state = Some(word.into());
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<TreeRow<Message>> for Element<'a, Message> {
    fn from(value: TreeRow<Message>) -> Self {
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
            let color = color.unwrap_or(muted);
            let icon = svg(handle)
                .width(MARK_WIDTH)
                .height(MARK_WIDTH)
                .style(move |_, _| svg::Style { color: Some(color) });
            marks.push(icon.into());
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
        // The label takes the room it needs first; the note on the right
        // takes what is left and ends with "…" when that is too little.
        content = content.push(
            EllipsisText::new(value.label, body.size(), fonts::UI, label_color)
                .shrink(0.0),
        );
        let note = match (value.state, value.count) {
            (Some(word), Some(count)) => Some(format!("{word} {count}")),
            (Some(word), None) => Some(word),
            (None, Some(count)) => Some(count.to_string()),
            (None, None) => None,
        };
        if let Some(note) = note {
            content = content.push(
                EllipsisText::new(
                    note,
                    typography::CAPTION.size(),
                    fonts::UI,
                    value.count_tone.color(&palette),
                )
                .align_x(TextAlignment::Right),
            );
        }

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

#[cfg(test)]
mod tests {
    use iced_test::selector::Candidate;
    use iced_test::simulator;

    use super::*;
    use crate::test_support::TempDir;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Open,
    }

    /// Draws a "Pods" row with a count of 2 and the state word "failing" in
    /// `tone` (the default when `None`), and checks it against the picture
    /// saved in `reference`; the first call saves the picture.
    fn drawn_row_matches(
        tone: Option<SessionTone>,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let mut row = TreeRow::<Message>::new("Pods", &palette)
            .count(2)
            .state("failing");
        if let Some(tone) = tone {
            row = row.count_tone(tone);
        }
        let mut ui = simulator(container(row).width(260));

        ui.snapshot(&iced::Theme::Dark)
            .expect("row should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
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
