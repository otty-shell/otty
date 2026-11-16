//! `BlockStatusLine`: the status of a terminal command block on the empty
//! line the terminal keeps above the command, with the fold arrow of the
//! block in the space left of the text.

use iced::widget::{Row, Space, container};
use iced::{Alignment, Element, alignment};

use super::block_fit::{StatusLineParts, StatusLineWidths};
use super::block_outcome::{
    BlockOutcome, label, label_width, outcome_icon, outcome_icon_width,
};
use super::glyph;
use super::icon_button::{IconButton, IconButtonSize};
use super::tag::{tag, tag_width};
use crate::palette::UiPalette;
use crate::{spacing, typography};

/// Space between the icon, the word and the duration of a status.
const STATUS_GAP: f32 = spacing::SPACE_2;
/// Space between the status and the tag of a folded block.
const BADGE_GAP: f32 = spacing::SPACE_4;
/// Space between the left edge of the line and the fold arrow.
const ARROW_X: f32 = 1.0;

/// The status line of a command block: the outcome icon, the word (`exit
/// 101` of a failed command, `running` of a running one), the duration and,
/// for a folded block, a [`tag`] such as `20 lines hidden` 12 px after
/// them. The status starts `text_inset` in from the left edge, where the
/// text of the terminal starts, and the fold arrow stands 1 px in from that
/// edge.
///
/// The line is as wide as the pane and `line_height` high, with every part
/// in its middle. Nothing in it wraps: when the room, `width` less the
/// inset at both sides, runs out, the tag is left out first, then the
/// duration; the word stays as long as it fits, because it says the
/// command failed. The icon always stays.
///
/// Only the arrow takes the pointer, on its own 14 px; presses, drags and
/// the wheel elsewhere go through to what lies under the line.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::block_outcome::BlockOutcome;
/// use otty_ui_kit::components::block_status_line::BlockStatusLine;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     Unfold,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let line: Element<'_, Message> =
///     BlockStatusLine::new(BlockOutcome::Failed, "0.2 s", &palette)
///         .word("exit 101")
///         .hidden_lines("20 lines hidden")
///         .fold_arrow(true, "Expand", Message::Unfold)
///         .line_height(16.0)
///         .width(400.0)
///         .text_inset(16.0)
///         .into();
/// ```
pub struct BlockStatusLine<Message> {
    outcome: BlockOutcome,
    duration: String,
    palette: UiPalette,
    word: Option<String>,
    hidden_lines: Option<String>,
    fold_arrow: Option<FoldArrow<Message>>,
    line_height: f32,
    width: f32,
    text_inset: f32,
}

impl<Message> BlockStatusLine<Message> {
    /// Creates the status line of a block whose command ended as `outcome`
    /// after `duration`, already written (`0.2 s`, `1 m 12 s`). It is 20 px
    /// high, the line of the terminal font, has no limit on its width and
    /// starts the status 16 px in, the side space of a block.
    pub fn new(
        outcome: BlockOutcome,
        duration: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            outcome,
            duration: duration.into(),
            palette: palette.clone(),
            word: None,
            hidden_lines: None,
            fold_arrow: None,
            line_height: typography::TERM.line_height(),
            width: f32::INFINITY,
            text_inset: spacing::SPACE_5,
        }
    }

    /// Shows `word` after the icon: `exit 101` in `status-error` for a
    /// failed command, `running` in `text-body` for a running one.
    pub fn word(mut self, word: impl Into<String>) -> Self {
        self.word = Some(word.into());
        self
    }

    /// Shows the tag of a folded block, such as `20 lines hidden`, after
    /// the status.
    pub fn hidden_lines(mut self, hidden_lines: impl Into<String>) -> Self {
        self.hidden_lines = Some(hidden_lines.into());
        self
    }

    /// Shows the fold arrow, named `name` in its tooltip, that sends
    /// `on_press`: the kit caret in a tiny [`IconButton`] without a
    /// background, right while the block `is_folded`, down while it is
    /// open. It is `text-muted`, and `text-strong` under the pointer.
    pub fn fold_arrow(
        mut self,
        is_folded: bool,
        name: impl Into<String>,
        on_press: Message,
    ) -> Self {
        self.fold_arrow = Some(FoldArrow {
            is_folded,
            name: name.into(),
            on_press,
        });
        self
    }

    /// Sets the height of the line: the place the terminal keeps for the
    /// status, the status row with the gap above the block, 22 px with
    /// 16 px lines of text. Every part stands in the middle of it, and the
    /// tag takes 16 px of a 22 px line.
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    /// Sets the width of the line, from the left edge of the pane to the
    /// right one, which decides which parts fit.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets how far in the status starts: where the text of the terminal
    /// starts. The status keeps as far from the right edge too.
    pub fn text_inset(mut self, text_inset: f32) -> Self {
        self.text_inset = text_inset;
        self
    }

    /// The widths of the parts of the status, measured with the font and
    /// the size they are drawn with.
    fn widths(&self) -> StatusLineWidths {
        StatusLineWidths {
            icon: outcome_icon_width(&self.outcome),
            word: self.word.as_deref().map(label_width),
            duration: label_width(&self.duration),
            badge: self.hidden_lines.as_deref().map(tag_width),
            gap: STATUS_GAP,
            badge_gap: BADGE_GAP,
        }
    }
}

impl<'a, Message: Clone + 'a> From<BlockStatusLine<Message>>
    for Element<'a, Message>
{
    fn from(line: BlockStatusLine<Message>) -> Self {
        let room = line.width - 2.0 * line.text_inset;
        let parts = StatusLineParts::fit(room, &line.widths());
        let BlockStatusLine {
            outcome,
            duration,
            palette,
            word,
            hidden_lines,
            fold_arrow,
            line_height,
            text_inset,
            ..
        } = line;

        let mut status = status_row(
            &outcome,
            word.filter(|_| parts.shows_word()),
            duration,
            parts,
            &palette,
        );
        // The tag follows the status in one row, so it moves with the
        // width of the status.
        if let Some(hidden_lines) = hidden_lines.filter(|_| parts.shows_badge())
        {
            status = Row::with_children([
                status,
                tag(hidden_lines, line_height, &palette),
            ])
            .spacing(BADGE_GAP)
            .align_y(Alignment::Center)
            .into();
        }

        // The arrow stands in the inset, `ARROW_X` from the left edge, and
        // the status after it, so the status starts `text_inset` in. A row
        // cuts nothing at the edges of the line: in a line of an odd height
        // the spinner reaches one pixel below it, and draws whole.
        let inset: Element<'a, Message> = match fold_arrow {
            Some(arrow) => Row::with_children([
                Space::new().width(ARROW_X).into(),
                container(arrow_button(arrow, &palette))
                    .width(text_inset - ARROW_X)
                    .into(),
            ])
            .into(),
            None => Space::new().width(text_inset).into(),
        };

        Row::with_children([inset, status])
            .height(line_height)
            .align_y(alignment::Vertical::Center)
            .into()
    }
}

/// The fold arrow a [`BlockStatusLine`] shows.
struct FoldArrow<Message> {
    is_folded: bool,
    name: String,
    on_press: Message,
}

/// One status: the icon, `word` and the duration, as far as `parts` shows
/// them.
fn status_row<'a, Message: 'a>(
    outcome: &BlockOutcome,
    word: Option<String>,
    duration: String,
    parts: StatusLineParts,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut row: Vec<Element<'a, Message>> =
        outcome_icon(outcome, palette).into_iter().collect();

    if let Some(word) = word {
        let color = match outcome {
            BlockOutcome::Failed => palette.status_error(),
            _ => palette.text_body(),
        };
        row.push(label(word, color));
    }
    if parts.shows_duration() {
        row.push(label(duration, palette.text_muted()));
    }

    Row::with_children(row)
        .spacing(STATUS_GAP)
        .align_y(Alignment::Center)
        .into()
}

/// The fold arrow: the kit caret in a tiny button without a background.
fn arrow_button<'a, Message: Clone + 'a>(
    arrow: FoldArrow<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    IconButton::new(glyph::caret(!arrow.is_folded), arrow.name, palette)
        .size(IconButtonSize::Tiny)
        .hover_background(false)
        .on_press(arrow.on_press)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::{Space, mouse_area, stack};
    use iced::{Event, Length, Point, Size, mouse};
    use iced_test::Simulator;

    use super::*;

    const LINE_HEIGHT: f32 = 20.0;
    const PANE: Size = Size::new(400.0, 100.0);
    const INSET: f32 = 16.0;
    /// A point over the fold arrow, in the left inset of the line.
    const OVER_THE_ARROW: Point = Point::new(8.0, 10.0);

    #[derive(Debug, Clone, PartialEq)]
    enum Reached {
        Under,
        Arrow,
    }

    /// `ls`, failed with 101 after 0.1 s, folded with 3 lines hidden.
    fn failed_folded_line(palette: &UiPalette) -> BlockStatusLine<Reached> {
        BlockStatusLine::new(BlockOutcome::Failed, "0.1 s", palette)
            .word("exit 101")
            .hidden_lines("3 lines hidden")
            .line_height(LINE_HEIGHT)
            .text_inset(INSET)
    }

    /// `line` `pane_width` wide over something that reports a press.
    fn over_a_pane(
        line: BlockStatusLine<Reached>,
        pane_width: f32,
    ) -> Simulator<'static, Reached> {
        let under =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(Reached::Under);
        let content: Element<'static, Reached> =
            stack![under, line.width(pane_width)]
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

        Simulator::with_size(
            iced::Settings::default(),
            Size::new(pane_width, PANE.height),
            content,
        )
    }

    fn press_at(ui: &mut Simulator<'_, Reached>, position: Point) {
        ui.point_at(position);
        let _ = ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved { position }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]);
    }

    #[test]
    fn a_wide_line_shows_the_word_the_duration_and_the_tag() {
        let palette = UiPalette::otty_dark();

        let mut ui = over_a_pane(failed_folded_line(&palette), PANE.width);

        assert!(ui.find("exit 101").is_ok());
        assert!(ui.find("0.1 s").is_ok());
        assert!(ui.find("3 lines hidden").is_ok());
    }

    #[test]
    fn a_running_command_shows_its_word() {
        let palette = UiPalette::otty_light();
        let line =
            BlockStatusLine::new(BlockOutcome::Running, "12 s", &palette)
                .word("running");

        let mut ui = over_a_pane(line, PANE.width);

        assert!(ui.find("running").is_ok());
        assert!(ui.find("12 s").is_ok());
    }

    #[test]
    fn the_status_starts_at_the_text_inset() {
        let palette = UiPalette::otty_dark();
        let line =
            BlockStatusLine::new(BlockOutcome::Unknown, "0.1 s", &palette)
                .text_inset(INSET);

        let mut ui = over_a_pane(line, PANE.width);
        let duration = ui.find("0.1 s").expect("the duration").bounds();

        assert_eq!(duration.x, INSET);
    }

    #[test]
    fn the_tag_is_12_px_after_the_status_in_the_line_and_lets_presses_through()
    {
        let palette = UiPalette::otty_dark();
        let mut ui = over_a_pane(failed_folded_line(&palette), PANE.width);

        let duration = ui.find("0.1 s").expect("the duration").bounds();
        let badge = ui.find("3 lines hidden").expect("the tag").bounds();

        // The text is in the middle of the tag, so the tag starts half of
        // the space around the text before it.
        let tag_x = badge.x - (tag_width("3 lines hidden") - badge.width) / 2.0;
        let gap = tag_x - (duration.x + duration.width);
        assert!((gap - 12.0).abs() < 0.5, "the gap is {gap} px");
        assert!(badge.y >= 0.0 && badge.y + badge.height <= LINE_HEIGHT);
        press_at(&mut ui, badge.center());

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert_eq!(reached, [Reached::Under]);
    }

    /// Owner, issue 07: the status line takes the 6 px gap above its block
    /// as well as the 16 px status row, and its parts stand in the middle
    /// of those 22 px.
    #[test]
    fn in_a_line_with_the_gap_above_the_block_the_parts_are_in_its_middle() {
        let palette = UiPalette::otty_dark();
        let line = failed_folded_line(&palette).line_height(22.0);

        let mut ui = over_a_pane(line, PANE.width);
        let duration = ui.find("0.1 s").expect("the duration").bounds();
        let badge = ui.find("3 lines hidden").expect("the tag").bounds();

        for part in [duration, badge] {
            let middle = part.y + part.height / 2.0;
            assert!((middle - 11.0).abs() < 0.5, "{part:?}");
        }
    }

    /// Owner, issue 11: in a narrow pane the status line stays one line;
    /// the tag is left out first, then the duration. The exit code stays.
    #[test]
    fn in_a_narrow_line_the_tag_goes_first_then_the_duration() {
        let palette = UiPalette::otty_dark();
        let widths = failed_folded_line(&palette).widths();
        let without_badge = widths.icon.unwrap_or_default()
            + widths.word.unwrap_or_default()
            + widths.duration
            + 2.0 * widths.gap;
        let insets = 2.0 * INSET;

        let narrow_width = insets + without_badge + 1.0;
        let mut narrow =
            over_a_pane(failed_folded_line(&palette), narrow_width);
        let duration = narrow.find("0.1 s").expect("the duration stays");
        assert!(narrow.find("3 lines hidden").is_err());
        assert!(narrow.find("exit 101").is_ok());
        assert!(duration.bounds().height <= LINE_HEIGHT, "one line");

        let narrower_width = insets + without_badge - 1.0;
        let mut narrower =
            over_a_pane(failed_folded_line(&palette), narrower_width);
        assert!(narrower.find("0.1 s").is_err());
        assert!(narrower.find("exit 101").is_ok());
    }

    #[test]
    fn the_fold_arrow_takes_its_press_and_the_rest_of_the_line_lets_it_through()
    {
        let palette = UiPalette::otty_dark();
        let line = failed_folded_line(&palette).fold_arrow(
            true,
            "Expand",
            Reached::Arrow,
        );
        let mut ui = over_a_pane(line, PANE.width);
        let duration = ui.find("0.1 s").expect("the duration").bounds();

        press_at(&mut ui, OVER_THE_ARROW);
        press_at(&mut ui, duration.center());

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert_eq!(reached, [Reached::Arrow, Reached::Under]);
    }

    #[test]
    fn without_a_fold_arrow_a_press_in_the_inset_goes_through() {
        let palette = UiPalette::otty_dark();
        let mut ui = over_a_pane(failed_folded_line(&palette), PANE.width);

        press_at(&mut ui, OVER_THE_ARROW);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert_eq!(reached, [Reached::Under]);
    }
}
