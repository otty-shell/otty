/// What the notifications widget is asked to do.
#[derive(Debug, Clone)]
pub(crate) enum NotificationsIntent {
    /// Tell the user about an error: a toast at the top of the window.
    /// Carries the text, already in the user's language.
    Push(String),
    /// The cross of a toast was pressed; carries the toast's id.
    DismissToast(u64),
    /// A tick of the toast timer: toasts shown long enough go away.
    Tick,
}

/// What the notifications reducer asks its route to do.
#[derive(Debug, Clone)]
pub(crate) enum NotificationsEffect {}

/// An event of the notifications widget as the app loop carries it.
#[derive(Debug, Clone)]
pub(crate) enum NotificationsEvent {
    /// An intent for the reducer.
    Intent(NotificationsIntent),
    /// An effect for the route.
    #[expect(
        dead_code,
        reason = "toasts need no effect; the list's effects will use it"
    )]
    Effect(NotificationsEffect),
}
