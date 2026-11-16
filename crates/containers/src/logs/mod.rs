//! A container's log lines and runs, as the inspector's Logs tab shows them
//! (spec Decision 12).

#[cfg(feature = "client")]
mod follow;

/// The most text one log line holds, in bytes. A longer run becomes several
/// lines, so the newest output stays visible.
pub const MAX_LOG_LINE_BYTES: usize = 64 * 1024;

/// Where a line stands in a container's log: the engine's time of the line
/// and, because the engines can give several lines one time (Podman's
/// `k8s-file` gives one time to everything written at once), which line of
/// that time it is. What follows that line without a time of its own, such as
/// the pieces of a long line, counts on as `piece`. A newer position sorts
/// after an older one. A stream can go on from here without repeating or
/// losing a line, because a later stream cuts the same text in the same places.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogPosition {
    time: i64,
    nth_at_time: u32,
    piece: u32,
}

/// One line of a container's log, without its time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    text: String,
    position: Option<LogPosition>,
}

impl LogLine {
    /// The text, without the line break.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Where the stream stands after this line; `None` when no line so far
    /// had a time.
    pub fn position(&self) -> Option<LogPosition> {
        self.position
    }

    /// The text, taken out of the line.
    pub fn into_text(self) -> String {
        self.text
    }
}

/// The latest run of a container, as inspect tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerRun {
    started_at: Option<i64>,
    cutoff: Option<i64>,
    is_running: bool,
}

impl ContainerRun {
    /// Whether the container is running now. The engines count a paused one
    /// as running.
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Whether `other` is the same run of the container: every start gives a
    /// new `StartedAt`.
    pub fn is_same_run(&self, other: &Self) -> bool {
        self.started_at == other.started_at
    }
}
