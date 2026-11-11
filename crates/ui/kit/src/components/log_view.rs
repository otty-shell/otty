//! `LogView`: read-only, selectable mono text that can follow new lines.

use std::sync::Arc;

use iced::widget::text::Wrapping;
use iced::widget::text_editor::{self, Action, Content, Edit, Motion};
use iced::{Border, Element, Length, Padding, Pixels};

use crate::palette::UiPalette;
use crate::{fonts, radius};

/// Text size (`font-size: 11.5px`).
const TEXT_SIZE: f32 = 11.5;
/// Line height (`line-height: 18px`).
const LINE_HEIGHT: f32 = 18.0;
/// Vertical padding (`padding: 10px 12px`).
const PADDING_Y: f32 = 10.0;
/// Horizontal padding (`padding: 10px 12px`).
const PADDING_X: f32 = 12.0;
/// Opacity of the selection color over the log background.
const SELECTION_OPACITY: f32 = 0.35;

/// Lines shown by a [`LogView`], and whether the view follows new lines.
///
/// While following, appended lines are added at once and the view stays at
/// the bottom. While not following, appended lines are held back and the view
/// does not move; they are added when following is switched on again.
/// Scrolling up stops following, so the user can read while lines arrive.
///
/// With [`LogViewState::max_lines`] and [`LogViewState::max_bytes`] the oldest
/// lines are dropped once there are more, so a long-running log does not grow
/// without end. Shown lines that go over a limit are cut to about 80% of the
/// limits, so a full log can take more lines before it is cut again.
///
/// While not following, the shown text is not touched, and the two limits work
/// differently for the held-back lines. They get only what the shown text
/// leaves of the byte limit, so the two together stay under it, plus at most
/// the newest line, which is always kept. The line limit holds for them on
/// their own, so a full log does not lose every line that arrives in a pause.
///
/// With [`LogViewState::skipped_output_line`], the place where held-back lines
/// were dropped gets one line that says so, once there is shown text above it.
/// That line counts toward the limits like any shown line.
pub struct LogViewState {
    content: Content,
    pending: Vec<String>,
    is_following: bool,
    has_lines: bool,
    max_lines: Option<usize>,
    max_bytes: Option<usize>,
    shown_bytes: usize,
    pending_bytes: usize,
    skipped_output_line: Option<String>,
    has_dropped_pending: bool,
}

impl LogViewState {
    /// Creates an empty log that follows new lines.
    pub fn new() -> Self {
        Self {
            content: Content::new(),
            pending: Vec::new(),
            is_following: true,
            has_lines: false,
            max_lines: None,
            max_bytes: None,
            shown_bytes: 0,
            pending_bytes: 0,
            skipped_output_line: None,
            has_dropped_pending: false,
        }
    }

    /// Keeps at most `max` lines (at least one): the oldest shown and
    /// held-back lines are dropped first.
    ///
    /// ```
    /// use otty_ui_kit::components::log_view::LogViewState;
    ///
    /// let mut state = LogViewState::new().max_lines(2);
    /// state.push_lines(["one", "two", "three"]);
    ///
    /// assert_eq!(state.text(), "two\nthree");
    /// ```
    pub fn max_lines(mut self, max: usize) -> Self {
        self.max_lines = Some(max);
        self
    }

    /// Keeps at most `max` bytes of text, each line break counted, for the
    /// shown and the held-back lines together: the oldest lines are dropped
    /// first, but while not following only held-back lines go. The newest line
    /// always stays, whatever its length.
    ///
    /// ```
    /// use otty_ui_kit::components::log_view::LogViewState;
    ///
    /// let mut state = LogViewState::new().max_bytes(10);
    /// state.push_lines(["one", "two", "three"]);
    ///
    /// assert_eq!(state.text(), "two\nthree");
    /// ```
    pub fn max_bytes(mut self, max: usize) -> Self {
        self.max_bytes = Some(max);
        self
    }

    /// Shows `line`, a single line of text, between the shown lines and the
    /// held-back lines that follow them when some held-back lines in between
    /// were dropped. Without it the two simply join.
    ///
    /// ```
    /// use otty_ui_kit::components::log_view::LogViewState;
    ///
    /// let mut state = LogViewState::new()
    ///     .max_lines(100)
    ///     .max_bytes(40)
    ///     .skipped_output_line("(skipped)");
    /// state.push_lines(["shown"]);
    /// state.set_following(false);
    /// state.push_lines(["dropped because it is too long", "kept"]);
    /// state.set_following(true);
    ///
    /// assert_eq!(state.text(), "shown\n(skipped)\nkept");
    /// ```
    pub fn skipped_output_line(mut self, line: impl Into<String>) -> Self {
        self.skipped_output_line = Some(line.into());
        self
    }

    /// Returns whether new lines are shown at once and kept in view.
    pub fn is_following(&self) -> bool {
        self.is_following
    }

    /// Returns how many appended lines are held back while not following.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Returns the shown text, lines separated by a line break.
    pub fn text(&self) -> String {
        self.content.text()
    }

    /// Returns the number of shown lines.
    pub fn line_count(&self) -> usize {
        if self.has_lines {
            self.content.line_count()
        } else {
            0
        }
    }

    /// Appends `lines` after the last one. While not following they are held
    /// back so the view does not move.
    pub fn push_lines<I, S>(&mut self, lines: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for line in lines {
            let line = line.into();
            self.pending_bytes += line_bytes(&line);
            self.pending.push(line);
        }
        self.drop_oldest_pending();

        if self.is_following {
            self.show_pending();
        }
    }

    /// Switches following on or off. Switching it on adds the held-back lines
    /// and moves the view to the bottom.
    pub fn set_following(&mut self, is_following: bool) {
        self.is_following = is_following;

        if is_following {
            self.show_pending();
            self.move_to_end();
        }
    }

    /// Applies a selection, cursor or scroll action from the view. Edits are
    /// ignored because the log is read-only. Scrolling up stops following.
    pub fn perform(&mut self, action: Action) {
        if action.is_edit() {
            return;
        }

        if let Action::Scroll { lines } = action
            && lines < 0
        {
            self.is_following = false;
        }
        self.content.perform(action);
    }

    /// Drops the oldest held-back lines that are over the limits. They stay at
    /// the limits themselves, not at [`trim_target`]: dropping only shifts the
    /// list of waiting lines and copies no text, so it costs little for each
    /// batch, and as many waiting lines as fit stay for the reader.
    fn drop_oldest_pending(&mut self) {
        let first = first_kept_line(
            &self.pending,
            self.max_lines,
            self.pending_byte_limit(),
        );

        for line in self.pending.drain(..first) {
            self.pending_bytes -= line_bytes(&line);
        }
        self.has_dropped_pending |= first > 0;
    }

    /// How many bytes the held-back lines may take. While not following, the
    /// shown text must stay where the user reads it, so they get what it
    /// leaves of the limit. While following, they are shown at once, and
    /// showing them keeps the two together under the limit.
    fn pending_byte_limit(&self) -> Option<usize> {
        if self.is_following {
            return self.max_bytes;
        }

        self.max_bytes
            .map(|max| max.saturating_sub(self.shown_bytes))
    }

    /// The line to add before the held-back lines: only when some of them were
    /// dropped and there is shown text above, so the gap is in the middle.
    fn skipped_line_to_show(&self) -> Option<&str> {
        if self.has_dropped_pending && self.has_lines {
            self.skipped_output_line.as_deref()
        } else {
            None
        }
    }

    /// Whether the shown and the held-back lines together, with the line for
    /// skipped output when it is due, are over a limit.
    fn is_over_limits(&self) -> bool {
        let skipped_line = self.skipped_line_to_show();
        let lines = self.line_count()
            + self.pending.len()
            + usize::from(skipped_line.is_some());
        let bytes = self.shown_bytes
            + self.pending_bytes
            + skipped_line.map_or(0, line_bytes);

        self.max_lines.is_some_and(|max| lines > max.max(1))
            || self.max_bytes.is_some_and(|max| bytes > max)
    }

    /// Adds the held-back lines at the end and moves the cursor there, which
    /// brings the bottom into view.
    fn show_pending(&mut self) {
        if self.pending.is_empty() {
            return;
        }

        if self.is_over_limits() {
            self.keep_last_lines();
            return;
        }

        let skipped_bytes = self.skipped_line_to_show().map_or(0, line_bytes);
        let mut text =
            String::with_capacity(self.pending_bytes + skipped_bytes);
        if let Some(skipped_line) = self.skipped_line_to_show() {
            text.push('\n');
            text.push_str(skipped_line);
        }
        for line in self.pending.drain(..) {
            if self.has_lines {
                text.push('\n');
            }
            text.push_str(&line);
            self.has_lines = true;
        }
        self.shown_bytes += self.pending_bytes + skipped_bytes;
        self.pending_bytes = 0;
        self.has_dropped_pending = false;

        self.content.perform(Action::Move(Motion::DocumentEnd));
        self.content
            .perform(Action::Edit(Edit::Paste(Arc::new(text))));
    }

    /// Moves the cursor to the end, which brings the bottom into view. The
    /// editor scrolls to the cursor only when the cursor moves, and after the
    /// user scrolled it may already be at the end, so it goes to the start
    /// first.
    fn move_to_end(&mut self) {
        self.content.perform(Action::Move(Motion::DocumentStart));
        self.content.perform(Action::Move(Motion::DocumentEnd));
    }

    /// Replaces the text with the newest of the shown and held-back lines
    /// that fit [`trim_target`] of the limits and moves the cursor to the end.
    /// `text_editor` cannot delete whole lines from the top without a layout,
    /// so the text is rebuilt. It is pasted, because `Content::with_text` lays
    /// out every line at once, which takes seconds for a full log: a paste lays
    /// out only what is on screen.
    fn keep_last_lines(&mut self) {
        let mut lines: Vec<String> = if self.has_lines {
            self.content
                .lines()
                .map(|line| line.text.into_owned())
                .collect()
        } else {
            Vec::new()
        };
        if let Some(skipped_line) = self.skipped_line_to_show() {
            lines.push(skipped_line.to_string());
        }
        lines.append(&mut self.pending);

        let mut first = first_kept_line(
            &lines,
            self.max_lines.map(trim_target),
            self.max_bytes.map(trim_target),
        );
        if self.is_skipped_line_on_top(&lines[first..]) {
            first += 1;
        }
        let kept = &lines[first..];
        let text = kept.join("\n");
        self.shown_bytes = kept.iter().map(|line| line_bytes(line)).sum();
        self.pending_bytes = 0;
        self.has_lines = true;
        self.has_dropped_pending = false;

        self.content = Content::new();
        self.content
            .perform(Action::Edit(Edit::Paste(Arc::new(text))));
        self.content.perform(Action::Move(Motion::DocumentEnd));
    }

    /// Whether `kept` lines start with the line for skipped output: with nothing
    /// above it, it marks no gap in the middle any more, whether it was added
    /// just now or in an earlier pause. The last line is never counted, because
    /// the newest line always stays.
    fn is_skipped_line_on_top(&self, kept: &[String]) -> bool {
        kept.len() > 1
            && kept.first().map(String::as_str)
                == self.skipped_output_line.as_deref()
    }
}

impl Default for LogViewState {
    fn default() -> Self {
        Self::new()
    }
}

/// Log text on `surface-chrome` with `radius-sm`, in the mono font. Long lines
/// wrap at any character, the text can be selected and copied, and the wheel
/// scrolls it.
///
/// Selecting and scrolling need the caller to pass the view's actions to
/// [`LogViewState::perform`] through [`LogView::on_action`].
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text_editor::Action;
/// use otty_ui_kit::components::log_view::{LogView, LogViewState};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Log(Action),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let mut state = LogViewState::new();
/// state.push_lines(["starting", "ready"]);
///
/// let log: Element<'_, Message> = LogView::new(&state, &palette)
///     .on_action(Message::Log)
///     .into();
/// ```
pub struct LogView<'a, Message> {
    state: &'a LogViewState,
    palette: UiPalette,
    height: Length,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
}

impl<'a, Message> LogView<'a, Message> {
    /// Creates a view of `state` that fills its parent.
    pub fn new(state: &'a LogViewState, palette: &UiPalette) -> Self {
        Self {
            state,
            palette: palette.clone(),
            height: Length::Fill,
            on_action: None,
        }
    }

    /// Sets the height of the view; the text scrolls inside it.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sends the selection, cursor and scroll actions of the view as
    /// messages. Without it the text can be neither selected nor scrolled.
    pub fn on_action(
        mut self,
        on_action: impl Fn(Action) -> Message + 'a,
    ) -> Self {
        self.on_action = Some(Box::new(on_action));
        self
    }
}

impl<'a, Message: 'a> From<LogView<'a, Message>> for Element<'a, Message> {
    fn from(value: LogView<'a, Message>) -> Self {
        let palette = value.palette;
        let background = palette.surface_chrome();
        let text_color = palette.text_body();
        let selection = palette.accent().scale_alpha(SELECTION_OPACITY);

        let mut editor = text_editor::TextEditor::new(&value.state.content)
            .font(fonts::MONO)
            .size(TEXT_SIZE)
            .line_height(Pixels(LINE_HEIGHT))
            .wrapping(Wrapping::Glyph)
            .padding(Padding::from([PADDING_Y, PADDING_X]))
            .height(value.height)
            .style(move |_, _| text_editor::Style {
                background: background.into(),
                border: Border {
                    radius: radius::RADIUS_SM.into(),
                    ..Border::default()
                },
                placeholder: text_color,
                value: text_color,
                selection,
            });
        if let Some(on_action) = value.on_action {
            editor = editor.on_action(on_action);
        }

        editor.into()
    }
}

/// What a line counts for against the byte limit: its text and a line break.
fn line_bytes(line: &str) -> usize {
    line.len() + 1
}

/// What a shown log over `limit` is cut down to: about 80% of it, rounded up.
/// Cutting rebuilds the whole text on the UI thread, which takes most of a
/// second for a full log. Cutting below the limit leaves room for about 20%
/// more text, so a full log is rebuilt only now and then, not for each batch.
fn trim_target(limit: usize) -> usize {
    limit - limit / 5
}

/// The index in `lines` of the first line that fits `max_lines` and
/// `max_bytes`, counted from the newest. The newest line always fits.
fn first_kept_line(
    lines: &[String],
    max_lines: Option<usize>,
    max_bytes: Option<usize>,
) -> usize {
    let max_lines = max_lines.unwrap_or(usize::MAX);
    let max_bytes = max_bytes.unwrap_or(usize::MAX);

    let mut kept = 0;
    let mut bytes = 0;
    for line in lines.iter().rev() {
        bytes += line_bytes(line);
        if kept > 0 && (kept >= max_lines || bytes > max_bytes) {
            break;
        }
        kept += 1;
    }

    lines.len() - kept
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    #[test]
    fn given_following_log_when_lines_appended_then_cursor_is_on_last_line() {
        let mut state = LogViewState::new();

        state.push_lines(["one", "two"]);
        state.push_lines(["three"]);

        assert_eq!(state.text(), "one\ntwo\nthree");
        assert_eq!(state.line_count(), 3);
        assert_eq!(state.content.cursor().position.line, 2);
    }

    #[test]
    fn given_not_following_when_lines_appended_then_view_does_not_move() {
        let mut state = LogViewState::new();
        state.push_lines(["one", "two"]);
        state.content.perform(Action::Move(Motion::DocumentStart));
        state.set_following(false);
        let cursor = state.content.cursor();

        state.push_lines(["three", "four"]);

        assert_eq!(state.text(), "one\ntwo");
        assert_eq!(state.content.cursor(), cursor);
        assert_eq!(state.pending_count(), 2);
    }

    #[test]
    fn given_held_back_lines_when_following_resumes_then_they_are_shown() {
        let mut state = LogViewState::new();
        state.push_lines(["one"]);
        state.set_following(false);
        state.push_lines(["two", "three"]);

        state.set_following(true);

        assert_eq!(state.text(), "one\ntwo\nthree");
        assert_eq!(state.pending_count(), 0);
        assert_eq!(state.content.cursor().position.line, 2);
    }

    #[test]
    fn given_line_limit_when_more_lines_arrive_then_the_oldest_are_dropped() {
        let mut state = LogViewState::new().max_lines(3);
        state.push_lines(["one", "two"]);

        state.push_lines(["three", "four", "five"]);

        assert_eq!(state.text(), "three\nfour\nfive");
        assert_eq!(state.line_count(), 3);
        assert_eq!(state.content.cursor().position.line, 2);
    }

    /// Lines of `bytes` bytes each, every line starting with its number so a
    /// test can tell which ones stayed.
    fn long_lines(count: usize, bytes: usize) -> Vec<String> {
        (0..count)
            .map(|number| format!("{number:04}{}", "x".repeat(bytes - 4)))
            .collect()
    }

    /// One to seven lines of a length that depends on `round`.
    fn varied_batch(round: usize) -> Vec<String> {
        (0..round % 7 + 1)
            .map(|number| {
                format!("{round}-{number}-{}", "x".repeat(round % 97))
            })
            .collect()
    }

    #[test]
    fn given_byte_limit_when_text_grows_past_it_then_the_oldest_lines_go() {
        let mut state = LogViewState::new().max_bytes(100);

        state.push_lines(long_lines(5, 29));
        state.push_lines(long_lines(1, 29));

        assert_eq!(state.line_count(), 2);
        assert!(state.text().len() <= 100, "{}", state.text().len());
        assert!(state.text().starts_with("0004"), "{}", state.text());
        assert!(state.text().ends_with('x'));
        assert_eq!(state.content.cursor().position.line, 1);
    }

    #[test]
    fn given_byte_limit_when_lines_fit_then_all_of_them_stay() {
        let mut state = LogViewState::new().max_bytes(100);

        state.push_lines(long_lines(1, 29));
        state.push_lines(long_lines(2, 29));

        assert_eq!(state.line_count(), 3);
    }

    #[test]
    fn given_byte_limit_when_a_later_small_batch_overflows_then_it_still_holds()
    {
        let mut state = LogViewState::new().max_bytes(100);

        for _ in 0..40 {
            state.push_lines(["ten bytes."]);
        }

        assert!(state.text().len() <= 100, "{}", state.text().len());
        assert_eq!(state.line_count(), 7);
    }

    #[test]
    fn given_byte_limit_when_not_following_then_held_back_lines_are_capped() {
        let mut state = LogViewState::new().max_bytes(100);
        state.push_lines(["first line of twenty"]);
        state.set_following(false);

        state.push_lines(long_lines(5, 29));

        assert_eq!(state.text(), "first line of twenty");
        assert_eq!(state.pending_count(), 2);
        assert_eq!(state.shown_bytes + state.pending_bytes, 81);
        state.set_following(true);
        assert!(state.text().starts_with("first"), "{}", state.text());
        assert!(state.text().contains("\n0003"), "{}", state.text());
        assert!(state.text().len() <= 100, "{}", state.text().len());
        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn given_shown_text_at_the_byte_limit_when_lines_wait_then_only_the_newest_stays()
     {
        let mut state = LogViewState::new().max_bytes(100);
        state.push_lines(long_lines(10, 9));
        state.content.perform(Action::Move(Motion::DocumentStart));
        state.set_following(false);
        let shown = state.text();
        let cursor = state.content.cursor();

        state.push_lines(["old waiting", "new waiting"]);
        state.push_lines(["newest"]);

        assert_eq!(state.text(), shown);
        assert_eq!(state.content.cursor(), cursor);
        assert_eq!(state.pending, ["newest"]);
        assert_eq!(state.shown_bytes, 100);
        assert_eq!(state.pending_bytes, 7);
    }

    #[test]
    fn given_room_left_by_the_shown_text_when_lines_wait_then_the_oldest_go_first()
     {
        let mut state = LogViewState::new().max_bytes(100);
        state.push_lines(long_lines(7, 9));
        state.set_following(false);

        state.push_lines(["first!!!!", "second!!!"]);
        state.push_lines(["third!!!!", "fourth!!!"]);

        assert_eq!(state.pending, ["second!!!", "third!!!!", "fourth!!!"]);
        assert_eq!(state.shown_bytes + state.pending_bytes, 100);
    }

    #[test]
    fn given_full_line_limit_when_a_short_pause_passes_then_every_line_waits() {
        let mut state = LogViewState::new().max_lines(5).max_bytes(1000);
        state.push_lines(long_lines(5, 9));
        state.set_following(false);

        state.push_lines(["one", "two", "three"]);

        assert_eq!(state.pending, ["one", "two", "three"]);
        assert_eq!(state.line_count(), 5);
    }

    #[test]
    fn given_dropped_waiting_lines_when_following_resumes_then_old_and_new_text_join()
     {
        let mut state = LogViewState::new().max_bytes(100);
        state.push_lines(long_lines(10, 9));
        state.set_following(false);
        state.push_lines(["dropped", "newest"]);

        state.set_following(true);

        let text = state.text();
        assert_eq!(state.pending_count(), 0);
        assert!(text.ends_with("xxxxx\nnewest"), "{text}");
        assert!(!text.contains("dropped"), "{text}");
        assert!(text.len() <= 80, "{}", text.len());
        assert_eq!(state.shown_bytes, text.len() + 1);
    }

    #[test]
    fn given_following_again_when_lines_arrive_then_the_whole_byte_limit_counts()
     {
        let mut state = LogViewState::new().max_bytes(100);
        state.push_lines(long_lines(5, 9));
        state.set_following(false);
        state.set_following(true);

        state.push_lines(long_lines(5, 9));

        assert_eq!(state.line_count(), 10);
        assert_eq!(state.shown_bytes, 100);
    }

    #[test]
    fn given_no_byte_limit_when_not_following_then_every_line_waits() {
        let mut state = LogViewState::new();
        state.push_lines(long_lines(5, 9));
        state.set_following(false);

        state.push_lines(long_lines(5, 9));

        assert_eq!(state.pending_count(), 5);
    }

    #[test]
    fn given_byte_limit_when_one_line_is_longer_then_the_newest_line_stays() {
        let mut state = LogViewState::new().max_bytes(10);

        state.push_lines(["old", "much longer than ten bytes"]);

        assert_eq!(state.text(), "much longer than ten bytes");
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn given_both_limits_when_lines_arrive_then_the_tighter_one_counts() {
        let mut by_lines = LogViewState::new().max_lines(2).max_bytes(1000);
        let mut by_bytes = LogViewState::new().max_lines(100).max_bytes(40);

        by_lines.push_lines(long_lines(4, 9));
        by_bytes.push_lines(long_lines(4, 9));

        assert_eq!(by_lines.line_count(), 2);
        assert!(by_lines.text().starts_with("0002"));
        assert_eq!(by_bytes.line_count(), 4);
        by_bytes.push_lines(long_lines(1, 9));
        assert_eq!(by_bytes.line_count(), 3);
    }

    #[test]
    fn given_shown_lines_when_more_are_added_then_their_bytes_are_counted_once()
    {
        let mut state = LogViewState::new().max_bytes(100);
        state.push_lines(long_lines(3, 29));
        state.set_following(false);
        state.push_lines(long_lines(1, 29));

        state.set_following(true);
        state.push_lines(long_lines(1, 29));

        assert_eq!(state.line_count(), 3);
        assert!(state.text().len() <= 100, "{}", state.text().len());
    }

    #[test]
    fn given_full_line_limit_when_it_goes_over_then_about_80_percent_stay() {
        let mut state = LogViewState::new().max_lines(100);
        state.push_lines(long_lines(100, 9));
        assert_eq!(state.line_count(), 100);

        state.push_lines(["newest"]);

        assert_eq!(state.line_count(), 80);
        assert!(state.text().starts_with("0021"), "{}", state.text());
        assert!(state.text().ends_with("\nnewest"), "{}", state.text());
        assert_eq!(state.content.cursor().position.line, 79);
    }

    #[test]
    fn given_full_byte_limit_when_it_goes_over_then_about_80_percent_stay() {
        let mut state = LogViewState::new().max_bytes(1000);
        state.push_lines(long_lines(10, 99));
        assert_eq!(state.line_count(), 10);

        state.push_lines(long_lines(1, 99));

        assert_eq!(state.line_count(), 8);
        assert_eq!(state.shown_bytes, 800);
        assert!(state.text().len() <= 800, "{}", state.text().len());
        assert!(state.text().starts_with("0003"), "{}", state.text());
        assert_eq!(state.content.cursor().position.line, 7);
    }

    #[test]
    fn given_trimmed_line_limit_when_small_batches_arrive_then_the_text_grows()
    {
        let mut state = LogViewState::new().max_lines(100);
        state.push_lines(long_lines(101, 9));
        state.push_lines(["over the limit"]);
        let trimmed = state.text();
        assert_eq!(state.line_count(), 80);

        for number in 1..=20 {
            state.push_lines([format!("next {number}")]);

            assert_eq!(state.line_count(), 80 + number);
            assert!(state.text().starts_with(&trimmed));
        }
        state.push_lines(["over again"]);

        assert_eq!(state.line_count(), 80);
    }

    #[test]
    fn given_trimmed_byte_limit_when_small_batches_arrive_then_the_text_grows()
    {
        let mut state = LogViewState::new().max_bytes(1000);
        state.push_lines(long_lines(11, 99));
        state.push_lines(long_lines(1, 99));
        let trimmed = state.text();
        assert_eq!(state.line_count(), 8);

        for number in 1..=2 {
            state.push_lines(long_lines(1, 99));

            assert_eq!(state.line_count(), 8 + number);
            assert!(state.text().starts_with(&trimmed));
        }
        state.push_lines(long_lines(1, 99));

        assert_eq!(state.line_count(), 8);
    }

    #[test]
    fn given_one_line_batches_when_the_log_stays_full_then_it_trims_rarely() {
        let mut state = LogViewState::new().max_lines(100);
        let mut trims = 0;

        for _ in 0..1000 {
            let before = state.line_count();
            state.push_lines(["one line"]);
            if state.line_count() < before {
                trims += 1;
            }
        }

        // The first trim comes at line 101, then one every 21 lines.
        assert_eq!(trims, 43);
    }

    #[test]
    fn given_line_limit_goes_over_when_trimmed_then_the_bytes_leave_room_too() {
        let mut state = LogViewState::new().max_lines(100).max_bytes(1000);
        state.push_lines(vec!["s"; 90]);
        state.push_lines(long_lines(10, 79));
        assert_eq!(state.line_count(), 100);

        state.push_lines(["new"]);

        assert_eq!(state.line_count(), 10);
        assert!(state.text().len() <= 800, "{}", state.text().len());
    }

    #[test]
    fn given_newest_line_over_the_target_when_trimmed_then_it_alone_stays() {
        let newest = format!("newest{}", "x".repeat(89));
        let mut state = LogViewState::new().max_bytes(100);
        state.push_lines(long_lines(3, 29));

        state.push_lines([newest.as_str()]);

        assert_eq!(state.text(), newest);
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn given_line_limit_of_one_when_lines_arrive_then_the_newest_stays() {
        let mut state = LogViewState::new().max_lines(1);

        state.push_lines(["one"]);
        state.push_lines(["two"]);

        assert_eq!(state.text(), "two");
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn given_many_batches_when_following_then_the_limits_hold_after_each_one() {
        let mut state = LogViewState::new().max_lines(50).max_bytes(2000);

        for round in 0..1000 {
            let batch = varied_batch(round);
            let newest = batch.last().expect("batch").clone();

            state.push_lines(batch);

            let text = state.text();
            assert!(state.line_count() <= 50, "{}", state.line_count());
            assert!(text.len() <= 2000, "{}", text.len());
            assert!(text.ends_with(&newest), "{text}");
            assert_eq!(state.shown_bytes, text.len() + 1);
        }
    }

    #[test]
    fn given_many_batches_when_follow_changes_then_the_limits_hold_after_each_one()
     {
        let mut state = LogViewState::new().max_lines(50).max_bytes(2000);

        for round in 0..1000 {
            state.set_following(round % 20 < 10);
            let batch = varied_batch(round);
            let newest = line_bytes(batch.last().expect("batch"));

            state.push_lines(batch);

            let held_back: usize =
                state.pending.iter().map(|line| line_bytes(line)).sum();
            let together = state.shown_bytes + held_back;
            assert!(state.line_count() <= 50, "{}", state.line_count());
            assert!(state.text().len() <= 2000, "{}", state.text().len());
            assert!(state.pending_count() <= 50, "{}", state.pending_count());
            assert!(together <= 2000 + newest, "{together}");
            assert_eq!(state.pending_bytes, held_back);
        }
    }

    #[test]
    fn given_not_following_when_lines_go_over_then_exactly_the_limit_is_held() {
        let mut state = LogViewState::new().max_lines(100).max_bytes(100_000);
        state.push_lines(["shown"]);
        state.set_following(false);

        for _ in 0..3 {
            state.push_lines(long_lines(70, 9));

            assert!(state.pending_count() <= 100);
        }

        assert_eq!(state.pending_count(), 100);
        assert_eq!(state.text(), "shown");
    }

    #[test]
    fn given_held_back_lines_over_the_limit_when_following_resumes_then_they_are_trimmed()
     {
        let mut state = LogViewState::new().max_lines(100);
        state.push_lines(["shown"]);
        state.set_following(false);
        state.push_lines(long_lines(100, 9));

        state.set_following(true);

        assert_eq!(state.pending_count(), 0);
        assert_eq!(state.line_count(), 80);
        assert!(state.text().starts_with("0020"), "{}", state.text());
        assert!(state.text().ends_with('x'), "{}", state.text());
        assert_eq!(state.content.cursor().position.line, 79);
    }

    #[test]
    fn given_log_at_the_limits_when_nothing_goes_over_then_nothing_is_dropped()
    {
        let mut state = LogViewState::new().max_lines(100).max_bytes(1000);

        for _ in 0..100 {
            state.push_lines(long_lines(1, 9));
        }

        assert_eq!(state.line_count(), 100);
        assert_eq!(state.shown_bytes, 1000);
        assert_eq!(state.pending_count(), 0);
    }

    const SKIPPED: &str = "-skipped-";

    /// A log of up to 100 bytes that names skipped output, with "shown" on
    /// screen and Follow off.
    fn paused_log() -> LogViewState {
        let mut state = LogViewState::new()
            .max_bytes(100)
            .skipped_output_line(SKIPPED);
        state.push_lines(["shown"]);
        state.set_following(false);
        state
    }

    fn line_of(letter: char, bytes: usize) -> String {
        letter.to_string().repeat(bytes)
    }

    #[test]
    fn given_dropped_waiting_lines_when_they_are_pasted_then_one_marker_is_between()
     {
        let mut state = paused_log();
        let (b, c) = (line_of('b', 29), line_of('c', 29));

        state.push_lines([line_of('a', 40), b.clone(), c.clone()]);
        state.set_following(true);

        let text = state.text();
        assert_eq!(text, format!("shown\n{SKIPPED}\n{b}\n{c}"));
        assert_eq!(state.shown_bytes, text.len() + 1);
        assert_eq!(state.line_count(), 4);
        assert_eq!(state.content.cursor().position.line, 3);
    }

    #[test]
    fn given_several_batches_dropped_in_one_pause_then_there_is_one_marker() {
        let mut state = paused_log();

        for letter in ['a', 'b', 'c', 'd', 'e'] {
            state.push_lines([line_of(letter, 39)]);
        }
        state.set_following(true);

        let (d, e) = (line_of('d', 39), line_of('e', 39));
        assert_eq!(state.text(), format!("shown\n{SKIPPED}\n{d}\n{e}"));
    }

    #[test]
    fn given_two_pauses_with_dropped_lines_then_each_gap_has_its_own_marker() {
        let mut state = LogViewState::new()
            .max_bytes(1000)
            .skipped_output_line(SKIPPED);
        state.push_lines(["shown"]);

        state.set_following(false);
        state.push_lines([line_of('a', 1000), String::from("first")]);
        state.set_following(true);
        state.set_following(false);
        state.push_lines([line_of('b', 1000), String::from("second")]);
        state.set_following(true);

        assert_eq!(
            state.text(),
            format!("shown\n{SKIPPED}\nfirst\n{SKIPPED}\nsecond")
        );
    }

    #[test]
    fn given_no_dropped_lines_when_following_resumes_then_there_is_no_marker() {
        let mut state = paused_log();

        state.push_lines(["one", "two"]);
        state.set_following(true);

        assert_eq!(state.text(), "shown\none\ntwo");
    }

    #[test]
    fn given_a_gap_already_marked_when_more_lines_come_then_no_marker_is_added()
    {
        let mut state = paused_log();
        state.push_lines([line_of('a', 100), String::from("kept")]);
        state.set_following(true);

        state.push_lines(["following"]);
        state.set_following(false);
        state.push_lines(["paused"]);
        state.set_following(true);

        assert_eq!(
            state.text(),
            format!("shown\n{SKIPPED}\nkept\nfollowing\npaused")
        );
    }

    #[test]
    fn given_nothing_shown_when_following_resumes_then_there_is_no_marker() {
        let mut state = LogViewState::new()
            .max_bytes(100)
            .skipped_output_line(SKIPPED);
        state.set_following(false);

        state.push_lines([line_of('a', 100), String::from("kept")]);
        state.set_following(true);

        assert_eq!(state.text(), "kept");
    }

    #[test]
    fn given_no_marker_text_when_lines_were_dropped_then_there_is_no_marker() {
        let mut state = LogViewState::new().max_bytes(100);
        state.push_lines(["shown"]);
        state.set_following(false);

        state.push_lines([line_of('a', 100), String::from("kept")]);
        state.set_following(true);

        assert_eq!(state.text(), "shown\nkept");
        assert_eq!(state.shown_bytes, state.text().len() + 1);
    }

    #[test]
    fn given_dropped_waiting_lines_when_the_text_is_rebuilt_then_one_marker_is_between()
     {
        let mut state = LogViewState::new()
            .max_bytes(100)
            .skipped_output_line(SKIPPED);
        state.push_lines(long_lines(10, 9));
        state.set_following(false);
        state.push_lines(["dropped", "newest"]);

        state.set_following(true);

        let text = state.text();
        assert!(text.starts_with("0004"), "{text}");
        assert!(
            text.ends_with(&format!("xxxxx\n{SKIPPED}\nnewest")),
            "{text}"
        );
        assert!(!text.contains("dropped"), "{text}");
        assert!(text.len() <= 80, "{}", text.len());
        assert_eq!(state.shown_bytes, text.len() + 1);
        assert_eq!(state.line_count(), 8);
        assert_eq!(state.content.cursor().position.line, 7);
    }

    #[test]
    fn given_a_rebuild_that_keeps_no_old_text_then_the_marker_goes_too() {
        let mut state = LogViewState::new()
            .max_bytes(100)
            .skipped_output_line(SKIPPED);
        state.push_lines([line_of('s', 60)]);
        state.set_following(false);
        let kept = line_of('k', 30);

        state.push_lines([line_of('a', 30), kept.clone()]);
        state.set_following(true);

        assert_eq!(state.text(), kept);
        assert_eq!(state.shown_bytes, kept.len() + 1);
    }

    #[test]
    fn given_the_line_limit_keeps_only_waiting_lines_then_there_is_no_marker() {
        let mut state = LogViewState::new()
            .max_lines(3)
            .skipped_output_line(SKIPPED);
        state.push_lines(["one"]);
        state.set_following(false);

        state.push_lines(["two", "three", "four", "five"]);
        state.set_following(true);

        assert_eq!(state.text(), "three\nfour\nfive");
    }

    #[test]
    fn given_text_trimmed_from_the_top_then_there_is_no_marker() {
        let mut state = LogViewState::new()
            .max_lines(100)
            .max_bytes(1000)
            .skipped_output_line(SKIPPED);

        for _ in 0..30 {
            state.push_lines(long_lines(10, 9));
        }

        assert!(!state.text().contains(SKIPPED), "{}", state.text());
        assert!(state.line_count() <= 100);
    }

    #[test]
    fn given_a_pause_over_the_limit_without_drops_then_there_is_no_marker() {
        let mut state = LogViewState::new()
            .max_lines(5)
            .skipped_output_line(SKIPPED);
        state.push_lines(["one", "two", "three", "four", "five"]);
        state.set_following(false);
        state.push_lines(["six", "seven"]);

        state.set_following(true);

        assert_eq!(state.text(), "four\nfive\nsix\nseven");
    }

    #[test]
    fn given_an_old_marker_left_on_top_by_a_trim_then_it_goes() {
        let mut state = LogViewState::new()
            .max_lines(10)
            .max_bytes(100)
            .skipped_output_line(SKIPPED);
        state.push_lines(["o1", "o2", "o3"]);
        state.set_following(false);
        state.push_lines([line_of('a', 100), String::from("kept")]);
        state.set_following(true);
        assert_eq!(state.text(), format!("o1\no2\no3\n{SKIPPED}\nkept"));

        state.push_lines(["n1", "n2", "n3", "n4", "n5"]);
        state.push_lines(["n6"]);

        assert_eq!(state.text(), "kept\nn1\nn2\nn3\nn4\nn5\nn6");
        assert_eq!(state.shown_bytes, state.text().len() + 1);
    }

    #[test]
    fn given_a_marker_when_lines_wait_then_the_waiting_count_leaves_it_out() {
        let mut state = paused_log();

        state.push_lines([line_of('a', 100), String::from("kept")]);

        assert_eq!(state.pending_count(), 1);
        assert_eq!(state.text(), "shown");
        assert_eq!(state.line_count(), 1);

        state.set_following(true);

        assert_eq!(state.pending_count(), 0);
        assert_eq!(state.line_count(), 3);
    }

    #[test]
    fn given_the_marker_tips_the_byte_limit_when_following_resumes_then_it_holds()
     {
        let mut state = LogViewState::new()
            .max_bytes(100)
            .skipped_output_line(SKIPPED);
        state.push_lines(["s0000", "s1111", "s2222", "s3333", "s4444"]);
        state.push_lines(["s5555", "s6666", "s7777", "s8888", "s9999"]);
        state.set_following(false);

        state.push_lines(long_lines(5, 9));
        assert_eq!(state.shown_bytes + state.pending_bytes, 100);
        state.set_following(true);

        let text = state.text();
        assert!(text.starts_with("s5555\n"), "{text}");
        assert!(text.contains(&format!("s9999\n{SKIPPED}\n0001")), "{text}");
        assert_eq!(state.shown_bytes, 80);
        assert_eq!(state.shown_bytes, text.len() + 1);
    }

    #[test]
    fn given_the_marker_tips_the_line_limit_when_following_resumes_then_it_holds()
     {
        let mut state = LogViewState::new()
            .max_lines(10)
            .max_bytes(100)
            .skipped_output_line(SKIPPED);
        state.push_lines(["s1", "s2", "s3", "s4"]);
        state.set_following(false);

        state.push_lines([line_of('a', 100)]);
        state.push_lines(["p1", "p2", "p3", "p4", "p5", "p6"]);
        assert_eq!(state.line_count() + state.pending_count(), 10);
        state.set_following(true);

        assert_eq!(
            state.text(),
            format!("s4\n{SKIPPED}\np1\np2\np3\np4\np5\np6")
        );
    }

    #[test]
    fn given_following_when_a_batch_drops_a_line_below_shown_text_then_it_is_marked()
     {
        let mut state = LogViewState::new()
            .max_bytes(100)
            .skipped_output_line(SKIPPED);
        state.push_lines(["shown"]);

        state.push_lines([line_of('a', 100), String::from("kept")]);

        assert_eq!(state.text(), format!("shown\n{SKIPPED}\nkept"));
    }

    #[test]
    fn given_many_gaps_when_follow_changes_then_the_limits_hold_after_each_one()
    {
        let mut state = LogViewState::new()
            .max_lines(50)
            .max_bytes(2000)
            .skipped_output_line(SKIPPED);
        let mut marked_gaps = 0;

        for round in 0..1000 {
            state.set_following(round % 20 < 10);
            let batch = varied_batch(round);
            let newest = line_bytes(batch.last().expect("batch"));

            state.push_lines(batch);

            let text = state.text();
            let together = state.shown_bytes + state.pending_bytes;
            assert!(state.line_count() <= 50, "{}", state.line_count());
            assert!(text.len() <= 2000, "{}", text.len());
            assert!(state.pending_count() <= 50, "{}", state.pending_count());
            assert!(together <= 2000 + newest, "{together}");
            assert_eq!(state.shown_bytes, text.len() + 1);
            assert!(!text.starts_with(SKIPPED), "{text}");
            assert!(!text.contains(&format!("{SKIPPED}\n{SKIPPED}")), "{text}");
            marked_gaps = marked_gaps.max(text.matches(SKIPPED).count());
        }

        assert!(marked_gaps > 0, "no gap was marked");
    }

    #[test]
    fn given_following_log_when_scrolled_up_then_following_stops() {
        let mut state = LogViewState::new();
        state.push_lines(["one", "two"]);

        state.perform(Action::Scroll { lines: -3 });
        state.push_lines(["three"]);

        assert!(!state.is_following());
        assert_eq!(state.text(), "one\ntwo");
        assert_eq!(state.pending_count(), 1);
    }

    #[test]
    fn given_following_log_when_scrolled_down_then_it_keeps_following() {
        let mut state = LogViewState::new();
        state.push_lines(["one", "two"]);

        state.perform(Action::Scroll { lines: 3 });
        state.push_lines(["three"]);

        assert!(state.is_following());
        assert_eq!(state.text(), "one\ntwo\nthree");
    }

    #[test]
    fn given_no_held_back_lines_when_following_resumes_then_view_goes_to_end() {
        let mut state = LogViewState::new();
        state.push_lines(["one", "two", "three"]);
        state.set_following(false);
        state.content.perform(Action::Move(Motion::DocumentStart));

        state.set_following(true);

        assert_eq!(state.text(), "one\ntwo\nthree");
        assert_eq!(state.content.cursor().position.line, 2);
    }

    #[test]
    fn given_line_limit_when_one_batch_is_longer_then_its_last_lines_stay() {
        let mut state = LogViewState::new().max_lines(2);

        state.push_lines(["one", "two", "three", "four"]);
        state.push_lines(["five"]);

        assert_eq!(state.text(), "four\nfive");
        assert_eq!(state.line_count(), 2);
    }

    #[test]
    fn given_line_limit_when_not_following_then_held_back_lines_are_capped() {
        let mut state = LogViewState::new().max_lines(3);
        state.push_lines(["one"]);
        state.set_following(false);

        state.push_lines(["two", "three", "four", "five"]);

        assert_eq!(state.text(), "one");
        assert_eq!(state.pending_count(), 3);
        state.set_following(true);
        assert_eq!(state.text(), "three\nfour\nfive");
        assert_eq!(state.content.cursor().position.line, 2);
    }

    #[test]
    fn given_line_limit_when_lines_are_empty_then_they_count_too() {
        let mut state = LogViewState::new().max_lines(3);

        state.push_lines(["one", "", "three", ""]);

        assert_eq!(state.text(), "\nthree\n");
        assert_eq!(state.line_count(), 3);
    }

    #[test]
    fn given_empty_log_when_created_then_it_follows_and_has_no_lines() {
        let state = LogViewState::new();

        assert!(state.is_following());
        assert_eq!(state.line_count(), 0);
    }

    #[test]
    fn given_edit_action_when_performed_then_text_is_unchanged() {
        let mut state = LogViewState::new();
        state.push_lines(["one"]);

        state.perform(Action::Edit(Edit::Insert('x')));
        state.perform(Action::Edit(Edit::Enter));

        assert_eq!(state.text(), "one");
    }

    #[test]
    fn given_log_when_rendered_then_view_builds_in_both_themes() {
        let mut state = LogViewState::new();
        state.push_lines(["starting", "ready"]);

        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let view: Element<'_, Action> = LogView::new(&state, &palette)
                .height(120)
                .on_action(|action| action)
                .into();
            let _ui = simulator(view);
        }
    }
}
