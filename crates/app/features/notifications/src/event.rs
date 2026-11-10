use std::time::Instant;

/// What the notifications feature is asked to do.
#[derive(Debug, Clone)]
pub enum NotificationsIntent {
    /// Tell the user about an error: a toast at the top of the window.
    /// Carries the text, already in the user's language.
    Push(String),
    /// The cross of a toast was pressed; carries the toast's id.
    DismissToast(u64),
    /// A tick of the toast timer: toasts shown long enough go away.
    Tick,
    /// The bell or the palette entry asked for the list.
    OpenList,
    /// Esc, a press outside the list, a shortcut or another menu closed
    /// the list.
    CloseList,
    /// The broom of the list was pressed: every notification goes.
    Clear,
    /// The cross of a row in the list was pressed; carries the id of the
    /// notification it removes.
    Remove(u64),
}

/// What the notifications feature reports back to the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationsOutcome {
    /// The list opened: it takes the keyboard from every widget, as any
    /// menu does.
    ListOpened,
    /// The list closed: in Terminal mode the active pane gets the keyboard
    /// back.
    ListClosed,
}

/// What the notifications feature reads from outside while it reduces: the
/// clock, which the binary reads, so that `reduce` stays plain and tests can
/// pick the time.
#[derive(Debug, Clone, Copy)]
pub struct NotificationsCtx {
    /// The time the intent is handled at.
    pub now: Instant,
}
