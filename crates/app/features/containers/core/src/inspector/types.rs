//! What the logs and the inspect answer of the inspector are about: the
//! container whose logs a stream follows and the row whose inspect JSON is
//! shown. Both travel in intents and effects.

use otty_containers::{EngineAddress, EngineInfo, LogPosition};

use crate::types::ResourceKind;

/// Which lines a logs stream reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogRead {
    /// The last lines, as when the logs open.
    LastLines,
    /// The lines of the latest run: the container came back while its logs
    /// were open.
    LatestRun,
    /// The lines after this position: the engine reconnected and the lines
    /// up to here are on screen.
    After(LogPosition),
}

/// One logs stream: the container whose logs are followed, its engine, which
/// lines it reads and how many streams came before it while the logs were
/// open. It keys the logs subscription: any change is another subscription,
/// and iced stops the old stream and starts the new one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LogTarget {
    engine: EngineInfo,
    container_id: String,
    read: LogRead,
    stream_number: u32,
}

impl LogTarget {
    /// The engine that runs the container.
    pub fn engine(&self) -> &EngineInfo {
        &self.engine
    }

    /// The container's full id.
    pub fn container_id(&self) -> &str {
        &self.container_id
    }

    /// Which lines the stream reads.
    pub fn read(&self) -> LogRead {
        self.read
    }

    /// Whether the logs are for the same container on the same engine, from
    /// any stream.
    pub(crate) fn is_same_container(&self, other: &Self) -> bool {
        self.engine == other.engine && self.container_id == other.container_id
    }

    /// Whether the container runs on the engine at `address`.
    pub(crate) fn is_on_engine(&self, address: &EngineAddress) -> bool {
        self.engine.address() == address
    }

    /// The first stream of the container `container_id` on `engine`: it reads
    /// the last lines.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_app_containers::{LogRead, LogTarget};
    /// use otty_containers::{EngineKind, fixtures};
    ///
    /// let target = LogTarget::new(fixtures::engine(EngineKind::Docker), "c178");
    ///
    /// assert_eq!(target.container_id(), "c178");
    /// assert_eq!(target.read(), LogRead::LastLines);
    /// ```
    pub fn new(engine: EngineInfo, container_id: &str) -> Self {
        Self {
            engine,
            container_id: container_id.to_string(),
            read: LogRead::LastLines,
            stream_number: 0,
        }
    }

    /// The same container, reading `read`.
    pub fn reading(mut self, read: LogRead) -> Self {
        self.read = read;
        self
    }

    /// Whether this is the container `container_id` on the engine at
    /// `address`.
    pub(super) fn is_container(
        &self,
        address: &EngineAddress,
        container_id: &str,
    ) -> bool {
        self.is_on_engine(address) && self.container_id == container_id
    }

    /// The next stream of the same container, reading `read`.
    pub(super) fn next_stream(&mut self, read: LogRead) {
        self.read = read;
        self.stream_number = self.stream_number.saturating_add(1);
    }
}

/// The row whose inspect JSON is shown: engine, list and id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectTarget {
    engine: EngineAddress,
    resource: ResourceKind,
    id: String,
}

impl InspectTarget {
    /// The list of the row.
    pub fn resource(&self) -> ResourceKind {
        self.resource
    }

    /// The row's id: container, image or network id, or volume name.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The row `id` of the `resource` list of the engine at `engine`.
    pub fn new(
        engine: EngineAddress,
        resource: ResourceKind,
        id: &str,
    ) -> Self {
        Self {
            engine,
            resource,
            id: id.to_string(),
        }
    }
}
