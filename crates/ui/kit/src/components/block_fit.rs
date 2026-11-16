//! What a [`BlockStatusLine`] and a [`BlockHeader`] show in a narrow pane.
//! Nothing drawn over a terminal block may wrap to a second line or be cut
//! in the middle, so when the room runs out its parts give way, whole, in
//! a fixed order. The components measure the parts with the font and the
//! size they draw them with; this module only decides which of them fit.
//!
//! [`BlockStatusLine`]: super::block_status_line::BlockStatusLine
//! [`BlockHeader`]: super::block_header::BlockHeader

/// How many characters of the command the sticky header keeps before it
/// leaves out other parts: the command is cut to them first.
const COMMAND_START_CHARS: usize = 8;
/// The mark at the end of a cut command.
const ELLIPSIS: &str = "…";

/// The widths of the parts of a status line, in pixels. A part the status
/// has not got is `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct StatusLineWidths {
    /// The status icon: a check, a cross or the spinner.
    pub(super) icon: Option<f32>,
    /// `exit 101` of a failed command, or `running`.
    pub(super) word: Option<f32>,
    /// How long the command took.
    pub(super) duration: f32,
    /// The `20 lines hidden` badge of a folded block.
    pub(super) badge: Option<f32>,
    /// The space between the icon, the word and the duration.
    pub(super) gap: f32,
    /// The space between the duration and the badge.
    pub(super) badge_gap: f32,
}

/// Which parts of a status line are shown. The icon always is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StatusLineParts {
    word: bool,
    duration: bool,
    badge: bool,
}

impl StatusLineParts {
    /// Return whether `exit 101` or `running` is shown.
    pub(super) fn shows_word(&self) -> bool {
        self.word
    }

    /// Return whether the duration is shown.
    pub(super) fn shows_duration(&self) -> bool {
        self.duration
    }

    /// Return whether the badge of a folded block is shown.
    pub(super) fn shows_badge(&self) -> bool {
        self.badge
    }

    /// Choose the parts of a status line that fit in `available` pixels.
    /// The icon stays. The badge is left out first, then the duration;
    /// `exit 101` stays as long as it fits, because it says the command
    /// failed.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let widths = StatusLineWidths {
    ///     icon: Some(12.0),
    ///     word: Some(40.0),
    ///     duration: 30.0,
    ///     badge: Some(90.0),
    ///     gap: 6.0,
    ///     badge_gap: 12.0,
    /// };
    ///
    /// let parts = StatusLineParts::fit(120.0, &widths);
    ///
    /// assert!(parts.shows_word() && parts.shows_duration());
    /// assert!(!parts.shows_badge());
    /// ```
    pub(super) fn fit(available: f32, widths: &StatusLineWidths) -> Self {
        let candidates = [
            Self::new(true, true, true),
            Self::new(true, true, false),
            Self::new(true, false, false),
        ];

        let parts = candidates
            .into_iter()
            .find(|parts| parts.width(widths) <= available)
            .unwrap_or(Self::new(false, false, false));

        // A part the status has not got is not shown.
        Self::new(
            parts.word && widths.word.is_some(),
            parts.duration,
            parts.badge && widths.badge.is_some(),
        )
    }

    fn new(word: bool, duration: bool, badge: bool) -> Self {
        Self {
            word,
            duration,
            badge,
        }
    }

    /// The width the shown parts take, with the space between them.
    fn width(&self, widths: &StatusLineWidths) -> f32 {
        let status = [
            widths.icon,
            widths.word.filter(|_| self.word),
            Some(widths.duration).filter(|_| self.duration),
        ];
        let status_width = row_width(&status, widths.gap);

        match widths.badge.filter(|_| self.badge) {
            Some(badge) if status_width > 0.0 => {
                status_width + widths.badge_gap + badge
            },
            Some(badge) => badge,
            None => status_width,
        }
    }
}

/// The widths of the parts of a sticky header, in pixels. A part the
/// header has not got is `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HeaderWidths {
    /// The status icon: a check, a cross or the spinner.
    pub(super) icon: Option<f32>,
    /// The start of the command the header keeps while it shows other
    /// parts, see [`command_start`].
    pub(super) command_start: f32,
    /// The command cut to "…" alone.
    pub(super) command_cut: f32,
    /// `412 lines above`.
    pub(super) lines_above: Option<f32>,
    /// `exit 101` of a failed command and the duration, with the space
    /// between them.
    pub(super) outcome: f32,
    /// The button that goes to the start of the block; `None` for a
    /// header without it.
    pub(super) button: Option<f32>,
    /// The space between two parts.
    pub(super) gap: f32,
}

/// Which parts of a sticky header are shown. The icon always is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HeaderParts {
    command: bool,
    lines_above: bool,
    outcome: bool,
    button: bool,
}

impl HeaderParts {
    /// Return whether the command is shown, cut with "…" to the room left.
    pub(super) fn shows_command(&self) -> bool {
        self.command
    }

    /// Return whether `412 lines above` is shown.
    pub(super) fn shows_lines_above(&self) -> bool {
        self.lines_above
    }

    /// Return whether `exit 101` and the duration are shown.
    pub(super) fn shows_outcome(&self) -> bool {
        self.outcome
    }

    /// Return whether the button that goes to the start of the block is
    /// shown.
    pub(super) fn shows_button(&self) -> bool {
        self.button
    }

    /// Choose the parts of a sticky header that fit in `available` pixels.
    /// The command is cut with "…" first, down to its start. Then
    /// `412 lines above` is left out, then `exit 101` and the duration.
    /// Then the command is cut further and left out. The button goes last,
    /// when even the icon and the button do not fit.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let widths = HeaderWidths {
    ///     icon: Some(12.0),
    ///     command_start: 70.0,
    ///     command_cut: 8.0,
    ///     lines_above: Some(90.0),
    ///     outcome: 50.0,
    ///     button: Some(24.0),
    ///     gap: 10.0,
    /// };
    ///
    /// let parts = HeaderParts::fit(200.0, &widths);
    ///
    /// assert!(parts.shows_command() && parts.shows_outcome());
    /// assert!(!parts.shows_lines_above());
    /// ```
    pub(super) fn fit(available: f32, widths: &HeaderWidths) -> Self {
        let candidates = [
            Self::new(true, true, true, true),
            Self::new(true, false, true, true),
            Self::new(true, false, false, true),
            Self::new(false, false, false, true),
        ];

        let parts = candidates
            .into_iter()
            .find(|parts| parts.width(widths) <= available)
            .unwrap_or(Self::new(false, false, false, false));

        // A part the header has not got is not shown.
        Self {
            lines_above: parts.lines_above && widths.lines_above.is_some(),
            button: parts.button && widths.button.is_some(),
            ..parts
        }
    }

    fn new(
        command: bool,
        lines_above: bool,
        outcome: bool,
        button: bool,
    ) -> Self {
        Self {
            command,
            lines_above,
            outcome,
            button,
        }
    }

    /// The width the shown parts take, with the space between them. While
    /// `412 lines above` or the outcome is shown, the command keeps its
    /// start; without them it may be cut down to "…".
    fn width(&self, widths: &HeaderWidths) -> f32 {
        let command = if self.lines_above || self.outcome {
            widths.command_start
        } else {
            widths.command_cut
        };
        let parts = [
            widths.icon,
            Some(command).filter(|_| self.command),
            widths.lines_above.filter(|_| self.lines_above),
            Some(widths.outcome).filter(|_| self.outcome),
            widths.button.filter(|_| self.button),
        ];

        row_width(&parts, widths.gap)
    }
}

/// Return the start of `command` the sticky header keeps before it leaves
/// out other parts: its first 8 characters and "…", or all of it when it
/// is not longer.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(command_start("cargo build --release"), "cargo bu…");
/// assert_eq!(command_start("ls"), "ls");
/// ```
pub(super) fn command_start(command: &str) -> String {
    match command.char_indices().nth(COMMAND_START_CHARS) {
        Some((end, _)) => format!("{}{ELLIPSIS}", &command[..end]),
        None => command.to_string(),
    }
}

/// Return the command cut to "…" alone.
pub(super) fn command_cut() -> &'static str {
    ELLIPSIS
}

/// The width of the shown `parts` in one row, `gap` apart.
fn row_width(parts: &[Option<f32>], gap: f32) -> f32 {
    let shown: Vec<f32> = parts.iter().flatten().copied().collect();
    let gaps = shown.len().saturating_sub(1) as f32 * gap;

    shown.iter().sum::<f32>() + gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A failed folded block: `✗ exit 101 0.2 s` and `20 lines hidden`.
    /// The whole line is 12 + 40 + 30 + 90 with 6 + 6 + 12 between them:
    /// 196 px.
    fn failed_folded_status() -> StatusLineWidths {
        StatusLineWidths {
            icon: Some(12.0),
            word: Some(40.0),
            duration: 30.0,
            badge: Some(90.0),
            gap: 6.0,
            badge_gap: 12.0,
        }
    }

    /// `cargo build --release`, failed after 48.2 s, 412 lines above. The
    /// whole header at its start is 12 + 70 + 90 + 80 + 24 with four gaps
    /// of 10: 316 px.
    fn failed_cargo_header() -> HeaderWidths {
        HeaderWidths {
            icon: Some(12.0),
            command_start: 70.0,
            command_cut: 8.0,
            lines_above: Some(90.0),
            outcome: 80.0,
            button: Some(24.0),
            gap: 10.0,
        }
    }

    #[test]
    fn a_status_line_shows_no_part_it_has_not_got() {
        let widths = StatusLineWidths {
            word: None,
            badge: None,
            ..failed_folded_status()
        };

        let parts = StatusLineParts::fit(500.0, &widths);

        assert!(!parts.shows_word());
        assert!(parts.shows_duration());
        assert!(!parts.shows_badge());
    }

    #[test]
    fn a_wide_status_line_shows_every_part() {
        let parts = StatusLineParts::fit(500.0, &failed_folded_status());

        assert!(parts.shows_word());
        assert!(parts.shows_duration());
        assert!(parts.shows_badge());
    }

    #[test]
    fn in_a_narrow_status_line_the_badge_goes_first() {
        let parts = StatusLineParts::fit(195.0, &failed_folded_status());

        assert!(parts.shows_word());
        assert!(parts.shows_duration());
        assert!(!parts.shows_badge());
    }

    #[test]
    fn then_the_duration_goes_and_the_exit_code_stays() {
        // Without the badge the line is 12 + 40 + 30 with two gaps: 94 px.
        let parts = StatusLineParts::fit(93.0, &failed_folded_status());

        assert!(parts.shows_word());
        assert!(!parts.shows_duration());
        assert!(!parts.shows_badge());
    }

    #[test]
    fn the_exit_code_stays_as_long_as_it_fits_then_only_the_icon_is_left() {
        let widths = failed_folded_status();

        let exit_fits = StatusLineParts::fit(58.0, &widths);
        let only_icon = StatusLineParts::fit(57.0, &widths);

        assert!(exit_fits.shows_word());
        assert!(!only_icon.shows_word());
        assert!(!only_icon.shows_duration());
    }

    #[test]
    fn a_status_line_with_every_part_that_fits_is_never_wider_than_its_room() {
        let widths = failed_folded_status();

        for available in 0..300 {
            let available = available as f32;
            let parts = StatusLineParts::fit(available, &widths);

            let only_icon = !parts.shows_word() && !parts.shows_duration();
            assert!(
                parts.width(&widths) <= available || only_icon,
                "{parts:?} at {available} px"
            );
        }
    }

    #[test]
    fn a_status_without_icon_keeps_its_duration_while_it_fits() {
        let widths = StatusLineWidths {
            icon: None,
            word: None,
            badge: None,
            ..failed_folded_status()
        };

        assert!(StatusLineParts::fit(30.0, &widths).shows_duration());
        assert!(!StatusLineParts::fit(29.0, &widths).shows_duration());
    }

    #[test]
    fn a_wide_header_shows_every_part() {
        let parts = HeaderParts::fit(600.0, &failed_cargo_header());

        assert_eq!(parts, HeaderParts::new(true, true, true, true));
    }

    #[test]
    fn in_a_middle_header_the_lines_above_go_after_the_command_is_cut() {
        // The command has its start only: everything fits at 316 px.
        let cut = HeaderParts::fit(316.0, &failed_cargo_header());
        let narrower = HeaderParts::fit(315.0, &failed_cargo_header());

        assert_eq!(cut, HeaderParts::new(true, true, true, true));
        assert_eq!(narrower, HeaderParts::new(true, false, true, true));
    }

    #[test]
    fn in_a_narrow_header_the_outcome_goes_next() {
        // Without the lines above: 12 + 70 + 80 + 24 with three gaps.
        let parts = HeaderParts::fit(215.0, &failed_cargo_header());

        assert_eq!(parts, HeaderParts::new(true, false, false, true));
    }

    #[test]
    fn in_a_very_narrow_header_the_command_goes_and_the_button_stays() {
        // The icon, "…" and the button: 12 + 8 + 24 with two gaps.
        let with_cut = HeaderParts::fit(64.0, &failed_cargo_header());
        let without = HeaderParts::fit(63.0, &failed_cargo_header());

        assert_eq!(with_cut, HeaderParts::new(true, false, false, true));
        assert_eq!(without, HeaderParts::new(false, false, false, true));
    }

    #[test]
    fn the_button_goes_only_when_even_the_icon_and_the_button_do_not_fit() {
        let both = HeaderParts::fit(46.0, &failed_cargo_header());
        let icon_only = HeaderParts::fit(45.0, &failed_cargo_header());

        assert!(both.shows_button());
        assert_eq!(icon_only, HeaderParts::new(false, false, false, false));
    }

    #[test]
    fn a_header_is_never_wider_than_its_room_while_it_has_more_than_its_icon() {
        let widths = failed_cargo_header();

        for available in 0..400 {
            let available = available as f32;
            let parts = HeaderParts::fit(available, &widths);

            let only_icon =
                parts == HeaderParts::new(false, false, false, false);
            assert!(
                parts.width(&widths) <= available || only_icon,
                "{parts:?} at {available} px"
            );
        }
    }

    #[test]
    fn a_header_without_lines_above_goes_straight_to_the_outcome() {
        let widths = HeaderWidths {
            lines_above: None,
            ..failed_cargo_header()
        };

        // 12 + 70 + 80 + 24 with three gaps: 216 px.
        assert_eq!(
            HeaderParts::fit(216.0, &widths),
            HeaderParts::new(true, false, true, true)
        );
        assert_eq!(
            HeaderParts::fit(215.0, &widths),
            HeaderParts::new(true, false, false, true)
        );
    }

    #[test]
    fn a_header_without_its_button_shows_none() {
        let widths = HeaderWidths {
            button: None,
            ..failed_cargo_header()
        };

        // 12 + 70 + 90 + 80 with three gaps: 282 px.
        assert_eq!(
            HeaderParts::fit(282.0, &widths),
            HeaderParts::new(true, true, true, false)
        );
        assert_eq!(
            HeaderParts::fit(12.0, &widths),
            HeaderParts::new(false, false, false, false)
        );
    }

    #[test]
    fn the_start_of_a_long_command_is_its_first_eight_characters() {
        assert_eq!(command_start("cargo build --release"), "cargo bu…");
        assert_eq!(command_start("ls -la"), "ls -la");
        assert_eq!(command_start("12345678"), "12345678");
        // Non-ASCII on purpose: each "é" takes two bytes.
        assert_eq!(command_start("éééééééééé"), "éééééééé…");
    }
}
