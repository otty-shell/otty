//! What watching an engine tells the app, and the lists it names.

/// One of the four lists of an engine that an event can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListKind {
    /// The container list.
    Containers,
    /// The image list.
    Images,
    /// The volume list.
    Volumes,
    /// The network list.
    Networks,
}

/// What watching an engine tells the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// This list changed: reload it. Comes at most once per 500 ms per list.
    Changed(ListKind),
    /// The container with this full id was started or restarted. Never held
    /// back: each start is one event, however fast the restarts come.
    ///
    /// ```
    /// use otty_containers::WatchEvent;
    ///
    /// let event = WatchEvent::ContainerStarted(String::from("c178aa0f"));
    ///
    /// assert!(matches!(
    ///     event,
    ///     WatchEvent::ContainerStarted(id) if id == "c178aa0f"
    /// ));
    /// ```
    ContainerStarted(String),
    /// The events stream broke and is open again. Changes in between were
    /// missed, so every list should be reloaded.
    Reconnected,
    /// The engine does not answer any more: look for engines again. The
    /// watch keeps trying, with a growing delay.
    Unreachable,
}
