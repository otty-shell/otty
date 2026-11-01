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
/// With [`LogViewState::max_lines`] the oldest lines are dropped once there
/// are more, so a long-running log does not grow without end.
pub struct LogViewState {
    content: Content,
    pending: Vec<String>,
    is_following: bool,
    has_lines: bool,
    max_lines: Option<usize>,
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
        self.pending.extend(lines.into_iter().map(Into::into));
        if let Some(max) = self.line_limit()
            && self.pending.len() > max
        {
            let dropped = self.pending.len() - max;
            self.pending.drain(..dropped);
        }

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

    /// The most lines kept, at least one; `None` keeps every line.
    fn line_limit(&self) -> Option<usize> {
        self.max_lines.map(|max| max.max(1))
    }

    /// Adds the held-back lines at the end and moves the cursor there, which
    /// brings the bottom into view.
    fn show_pending(&mut self) {
        if self.pending.is_empty() {
            return;
        }

        let total = self.line_count() + self.pending.len();
        if let Some(max) = self.line_limit()
            && total > max
        {
            self.keep_last_lines(max);
            return;
        }

        let mut text = self.pending.join("\n");
        if self.has_lines {
            text.insert(0, '\n');
        }
        self.pending.clear();
        self.has_lines = true;

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

    /// Replaces the text with the last `max` of the shown and held-back
    /// lines and moves the cursor to the end. `text_editor` cannot delete
    /// whole lines from the top without a layout, so the text is rebuilt.
    fn keep_last_lines(&mut self, max: usize) {
        let mut lines: Vec<String> = if self.has_lines {
            self.content
                .lines()
                .map(|line| line.text.into_owned())
                .collect()
        } else {
            Vec::new()
        };
        lines.append(&mut self.pending);

        let first = lines.len().saturating_sub(max);
        self.content = Content::with_text(&lines[first..].join("\n"));
        self.has_lines = true;

        self.content.perform(Action::Move(Motion::DocumentEnd));
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
