//! State of the inspector tabs (spec Decision 12): the open tab, the
//! followed logs of the selected container and the inspect JSON of the
//! selected row.

use std::fmt;

use iced::widget::text_editor::{Action, Motion};
use otty_docker::{EngineAddress, EngineInfo};
use otty_ui_kit::components::log_view::LogViewState;

use super::types::ResourceKind;
use crate::i18n::{self, Key};

/// Lines asked for when the logs open; then new lines follow.
pub(crate) const LOG_TAIL: usize = 500;
/// The most log lines kept; older lines are dropped.
pub(crate) const MAX_LOG_LINES: usize = 5000;

/// The tabs of a container.
const CONTAINER_TABS: [InspectorTab; 3] = [
    InspectorTab::Overview,
    InspectorTab::Logs,
    InspectorTab::Inspect,
];
/// The tabs of an image, a volume or a network: they have no logs.
const OTHER_TABS: [InspectorTab; 2] =
    [InspectorTab::Overview, InspectorTab::Inspect];

/// One tab of the inspector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum InspectorTab {
    /// Key facts of the selected row.
    #[default]
    Overview,
    /// The followed logs of the selected container.
    Logs,
    /// The engine's inspect JSON of the selected row.
    Inspect,
}

impl InspectorTab {
    /// The tabs of `resource`, in order: only containers have Logs.
    pub(crate) fn of(resource: ResourceKind) -> &'static [Self] {
        match resource {
            ResourceKind::Containers => &CONTAINER_TABS,
            _ => &OTHER_TABS,
        }
    }

    /// Translation key of the tab label.
    fn label_key(self) -> Key {
        match self {
            Self::Overview => Key::DockerTabOverview,
            Self::Logs => Key::DockerTabLogs,
            Self::Inspect => Key::DockerTabInspect,
        }
    }
}

impl fmt::Display for InspectorTab {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(i18n::t(self.label_key()))
    }
}

/// The container whose logs are followed, and its engine. It keys the logs
/// subscription: another container or engine is another subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LogTarget {
    engine: EngineInfo,
    container_id: String,
}

impl LogTarget {
    /// The engine that runs the container.
    pub(crate) fn engine(&self) -> &EngineInfo {
        &self.engine
    }

    /// The container's full id.
    pub(crate) fn container_id(&self) -> &str {
        &self.container_id
    }

    /// The container `container_id` on `engine`.
    pub(crate) fn new(engine: EngineInfo, container_id: &str) -> Self {
        Self {
            engine,
            container_id: container_id.to_string(),
        }
    }
}

/// The row whose inspect JSON is shown: engine, list and id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectTarget {
    engine: EngineAddress,
    resource: ResourceKind,
    id: String,
}

impl InspectTarget {
    /// The list of the row.
    pub(crate) fn resource(&self) -> ResourceKind {
        self.resource
    }

    /// The row's id: container, image or network id, or volume name.
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// The row `id` of the `resource` list of the engine at `engine`.
    pub(crate) fn new(
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

/// The followed logs of one container.
pub(crate) struct ContainerLogs {
    target: LogTarget,
    lines: LogViewState,
    error: Option<String>,
}

impl ContainerLogs {
    /// The container the lines come from.
    pub(crate) fn target(&self) -> &LogTarget {
        &self.target
    }

    /// The lines, at most [`MAX_LOG_LINES`].
    pub(crate) fn lines(&self) -> &LogViewState {
        &self.lines
    }

    /// The engine's message when the logs could not be read.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// No lines yet for `target`.
    fn new(target: LogTarget) -> Self {
        Self {
            target,
            lines: LogViewState::new().max_lines(MAX_LOG_LINES),
            error: None,
        }
    }
}

impl fmt::Debug for ContainerLogs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContainerLogs")
            .field("target", &self.target)
            .field("lines", &self.lines.line_count())
            .field("error", &self.error)
            .finish()
    }
}

/// Load state of the inspect JSON.
pub(crate) enum InspectLoad {
    /// Asked for, not here yet.
    Loading,
    /// The pretty-printed JSON, shown from its first line.
    Loaded(LogViewState),
    /// The engine's message.
    Failed(String),
}

impl fmt::Debug for InspectLoad {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Loading => formatter.write_str("Loading"),
            Self::Loaded(json) => formatter
                .debug_tuple("Loaded")
                .field(&json.line_count())
                .finish(),
            Self::Failed(message) => {
                formatter.debug_tuple("Failed").field(message).finish()
            },
        }
    }
}

/// The inspect JSON of one row.
#[derive(Debug)]
pub(crate) struct InspectData {
    target: InspectTarget,
    load: InspectLoad,
}

impl InspectData {
    /// The row the JSON is about.
    pub(crate) fn target(&self) -> &InspectTarget {
        &self.target
    }

    /// Whether it loads, loaded or failed.
    pub(crate) fn load(&self) -> &InspectLoad {
        &self.load
    }
}

/// The inspector's open tab, the followed logs and the last inspect JSON.
#[derive(Debug, Default)]
pub(crate) struct InspectorState {
    tab: InspectorTab,
    logs: Option<ContainerLogs>,
    inspect: Option<InspectData>,
}

impl InspectorState {
    /// The open tab.
    pub(crate) fn tab(&self) -> InspectorTab {
        self.tab
    }

    /// The followed logs; `None` while no logs are followed.
    pub(crate) fn logs(&self) -> Option<&ContainerLogs> {
        self.logs.as_ref()
    }

    /// The last inspect JSON asked for; kept when the tab closes, so it
    /// loads once per selection.
    pub(crate) fn inspect(&self) -> Option<&InspectData> {
        self.inspect.as_ref()
    }

    /// Open `tab`.
    pub(super) fn select_tab(&mut self, tab: InspectorTab) {
        self.tab = tab;
    }

    /// Follow the logs of `target`, or of nothing. Logs of another target
    /// are dropped; the same target keeps its lines.
    pub(super) fn follow_logs(&mut self, target: Option<LogTarget>) {
        let followed = self.logs.as_ref().map(ContainerLogs::target);
        if followed == target.as_ref() {
            return;
        }

        self.logs = target.map(ContainerLogs::new);
    }

    /// Add lines of `target`. Returns `false` when its logs are no longer
    /// followed.
    pub(super) fn push_log_lines(
        &mut self,
        target: &LogTarget,
        lines: Vec<String>,
    ) -> bool {
        let Some(logs) = self.followed_logs(target) else {
            return false;
        };

        logs.lines.push_lines(lines);
        true
    }

    /// Show that the logs of `target` could not be read. Returns `false`
    /// when its logs are no longer followed.
    pub(super) fn fail_logs(
        &mut self,
        target: &LogTarget,
        message: String,
    ) -> bool {
        let Some(logs) = self.followed_logs(target) else {
            return false;
        };

        logs.error = Some(message);
        true
    }

    /// Apply a selection or scroll action of the log view.
    pub(super) fn perform_log_action(&mut self, action: Action) {
        if let Some(logs) = self.logs.as_mut() {
            logs.lines.perform(action);
        }
    }

    /// Start loading the JSON of `target`, unless it is the one already
    /// loaded, loading or failed. Returns whether a load starts.
    pub(super) fn start_inspect(&mut self, target: InspectTarget) -> bool {
        let is_known = self
            .inspect
            .as_ref()
            .is_some_and(|inspect| inspect.target == target);
        if is_known {
            return false;
        }

        self.inspect = Some(InspectData {
            target,
            load: InspectLoad::Loading,
        });
        true
    }

    /// Take the JSON of `target`. Returns `false` when another row was
    /// asked for meanwhile.
    pub(super) fn finish_inspect(
        &mut self,
        target: &InspectTarget,
        result: Result<String, String>,
    ) -> bool {
        let Some(inspect) = self
            .inspect
            .as_mut()
            .filter(|inspect| inspect.target == *target)
        else {
            return false;
        };

        inspect.load = match result {
            Ok(json) => InspectLoad::Loaded(json_view(&json)),
            Err(message) => InspectLoad::Failed(message),
        };
        true
    }

    /// Apply a selection or scroll action of the inspect view.
    pub(super) fn perform_inspect_action(&mut self, action: Action) {
        if let Some(InspectData {
            load: InspectLoad::Loaded(json),
            ..
        }) = self.inspect.as_mut()
        {
            json.perform(action);
        }
    }

    /// The logs of `target`, when they are the followed ones.
    fn followed_logs(
        &mut self,
        target: &LogTarget,
    ) -> Option<&mut ContainerLogs> {
        self.logs.as_mut().filter(|logs| logs.target == *target)
    }
}

/// The JSON as read-only lines, shown from its first line.
fn json_view(json: &str) -> LogViewState {
    let mut view = LogViewState::new();
    view.push_lines(json.lines());

    view.perform(Action::Move(Motion::DocumentStart));
    view
}
