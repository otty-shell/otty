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

/// What the notifications reducer asks its route to do.
#[derive(Debug, Clone)]
pub(crate) enum NotificationsEffect {
    /// The list opened: it takes the keyboard from every widget, as any
    /// menu does.
    ListOpened,
    /// The list closed: the active pane gets the keyboard back.
    ListClosed,
}

/// An event of the notifications widget as the app loop carries it.
#[derive(Debug, Clone)]
pub(crate) enum NotificationsEvent {
    /// An intent for the reducer.
    Intent(NotificationsIntent),
    /// An effect for the route.
    Effect(NotificationsEffect),
}
