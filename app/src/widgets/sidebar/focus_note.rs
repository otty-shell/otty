use iced::time::{Duration, Instant};

/// How long the note stays fully visible from its first frame.
const NOTE_HOLD: Duration = Duration::from_millis(2000);
/// How long the note takes to fade out after the hold.
const NOTE_FADE: Duration = Duration::from_millis(400);

/// The focus mode note shown right after focus mode turns on. It stays for a
/// moment, then fades out so it does not cover the terminal. Time counts
/// from the first frame the note sees, so making the note needs no clock.
#[derive(Debug, Clone, Copy)]
pub(super) struct FocusNote {
    /// Time of the first frame; `None` until that frame arrives.
    shown_at: Option<Instant>,
    opacity: f32,
}

impl FocusNote {
    /// How visible the note is now, from 0.0 (gone) to 1.0 (fully shown).
    pub(super) fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Show the note at full opacity. It starts counting at its first
    /// frame.
    pub(super) fn new() -> Self {
        Self {
            shown_at: None,
            opacity: 1.0,
        }
    }

    /// Move the note to the frame at `now`; the first frame starts the hold.
    /// Returns `false` once the note has faded out completely.
    pub(super) fn advance(&mut self, now: Instant) -> bool {
        let shown_at = *self.shown_at.get_or_insert(now);

        let elapsed = now.saturating_duration_since(shown_at);

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
        let note = FocusNote::new();

        assert_eq!(note.opacity(), 1.0);
    }

    #[test]
    fn note_stays_fully_visible_for_the_hold_from_its_first_frame() {
        let first_frame = Instant::now() + Duration::from_secs(60);
        let mut note = FocusNote::new();

        let visible_at_first = note.advance(first_frame);
        let visible_after_hold = note.advance(first_frame + NOTE_HOLD);

        assert!(visible_at_first);
        assert!(visible_after_hold);
        assert_eq!(note.opacity(), 1.0);
    }

    #[test]
    fn note_is_half_faded_in_the_middle_of_the_fade() {
        let first_frame = Instant::now();
        let mut note = FocusNote::new();
        note.advance(first_frame);

        let visible = note.advance(first_frame + NOTE_HOLD + NOTE_FADE / 2);

        assert!(visible);
        assert!((note.opacity() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn note_is_gone_after_the_fade() {
        let first_frame = Instant::now();
        let mut note = FocusNote::new();
        note.advance(first_frame);

        let visible = note.advance(first_frame + NOTE_HOLD + NOTE_FADE);

        assert!(!visible);
        assert_eq!(note.opacity(), 0.0);
    }

    #[test]
    fn frame_before_the_first_frame_keeps_the_note_fully_visible() {
        let first_frame = Instant::now() + Duration::from_secs(1);
        let mut note = FocusNote::new();
        note.advance(first_frame);

        let visible = note.advance(first_frame - Duration::from_millis(500));

        assert!(visible);
        assert_eq!(note.opacity(), 1.0);
    }
}
