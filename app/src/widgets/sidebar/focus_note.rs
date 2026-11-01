use iced::time::{Duration, Instant};

/// How long the note stays fully visible after focus mode turns on.
const NOTE_HOLD: Duration = Duration::from_millis(2000);
/// How long the note takes to fade out after the hold.
const NOTE_FADE: Duration = Duration::from_millis(400);

/// The focus mode note shown right after focus mode turns on. It stays for a
/// moment, then fades out so it does not cover the terminal.
#[derive(Debug, Clone, Copy)]
pub(super) struct FocusNote {
    shown_at: Instant,
    opacity: f32,
}

impl FocusNote {
    /// How visible the note is now, from 0.0 (gone) to 1.0 (fully shown).
    pub(super) fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Show the note at full opacity, starting at `now`.
    pub(super) fn new(now: Instant) -> Self {
        Self {
            shown_at: now,
            opacity: 1.0,
        }
    }

    /// Move the note to the frame at `now`. Returns `false` once the note
    /// has faded out completely.
    pub(super) fn advance(&mut self, now: Instant) -> bool {
        let elapsed = now.saturating_duration_since(self.shown_at);

        self.opacity = opacity_after(elapsed);
        self.opacity > 0.0
    }
}

/// Opacity of the note `elapsed` after it was shown: full during the hold,
/// then falling evenly to zero over the fade.
fn opacity_after(elapsed: Duration) -> f32 {
    let Some(fading) = elapsed.checked_sub(NOTE_HOLD) else {
        return 1.0;
    };

    let progress = fading.as_secs_f32() / NOTE_FADE.as_secs_f32();
    (1.0 - progress).max(0.0)
}

#[cfg(test)]
mod tests {
    use iced::time::{Duration, Instant};

    use super::{FocusNote, NOTE_FADE, NOTE_HOLD};

    #[test]
    fn new_note_is_fully_visible() {
        let note = FocusNote::new(Instant::now());

        assert_eq!(note.opacity(), 1.0);
    }

    #[test]
    fn note_stays_fully_visible_during_the_hold() {
        let start = Instant::now();
        let mut note = FocusNote::new(start);

        let visible = note.advance(start + NOTE_HOLD);

        assert!(visible);
        assert_eq!(note.opacity(), 1.0);
    }

    #[test]
    fn note_is_half_faded_in_the_middle_of_the_fade() {
        let start = Instant::now();
        let mut note = FocusNote::new(start);

        let visible = note.advance(start + NOTE_HOLD + NOTE_FADE / 2);

        assert!(visible);
        assert!((note.opacity() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn note_is_gone_after_the_fade() {
        let start = Instant::now();
        let mut note = FocusNote::new(start);

        let visible = note.advance(start + NOTE_HOLD + NOTE_FADE);

        assert!(!visible);
        assert_eq!(note.opacity(), 0.0);
    }

    #[test]
    fn frame_before_the_start_keeps_the_note_fully_visible() {
        let start = Instant::now() + Duration::from_secs(1);
        let mut note = FocusNote::new(start);

        let visible = note.advance(start - Duration::from_millis(500));

        assert!(visible);
        assert_eq!(note.opacity(), 1.0);
    }
}
