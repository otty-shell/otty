//! State of the inspector (spec Decision 12): the followed logs of the
//! selected container and the inspect JSON of the selected row. The open tab
//! belongs to the parent widget.

use std::collections::BTreeSet;
use std::fmt;

use iced::widget::text_editor::{Action, Motion};
use otty_containers::{ContainerRun, EngineAddress, LogPosition};
use otty_ui_kit::components::log_view::LogViewState;
use serde_json::Value;

use super::super::super::types::ContainersError;
use super::inspect_tree::{InspectTree, first_section_title};
use super::types::{InspectTarget, LogRead, LogTarget};
use crate::i18n::{self, Key};

/// Lines asked for when the logs open; then new lines follow.
pub(crate) const LOG_TAIL: usize = 500;
/// The most log lines kept; older lines are dropped.
pub(crate) const MAX_LOG_LINES: usize = 5000;
/// The most log text kept, in bytes; older lines are dropped. It holds for the
/// shown lines and the lines waiting while Follow is off together, plus at most
/// the newest waiting line, which is always kept.
pub(crate) const MAX_LOG_TEXT_BYTES: usize = 16 * 1024 * 1024;

/// The followed logs of one container.
pub(crate) struct ContainerLogs {
    target: LogTarget,
    lines: LogViewState,
    error: Option<String>,
    /// The run the lines belong to; `None` until the stream has read it.
    shown_run: Option<ContainerRun>,
    /// Where the lines received so far end, waiting ones too.
    position: Option<LogPosition>,
    /// A reconnect found that the run of these lines ended, so what the old
    /// stream still reports says nothing: the container simply ended.
    has_ended: bool,
}

impl ContainerLogs {
    /// The container the lines come from.
    pub(crate) fn target(&self) -> &LogTarget {
        &self.target
    }

    /// The lines, at most [`MAX_LOG_LINES`] and [`MAX_LOG_TEXT_BYTES`].
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
            lines: empty_log_lines(),
            error: None,
            shown_run: None,
            position: None,
            has_ended: false,
        }
    }

    /// The container came back: forget the lines, waiting ones too, and the
    /// old stream's error, and ask for a new stream. Follow stays as it is.
    fn start_over(&mut self) {
        let is_following = self.lines.is_following();

        self.target.next_stream(LogRead::LatestRun);
        self.lines = empty_log_lines();
        self.lines.set_following(is_following);
        self.error = None;
        self.shown_run = None;
        self.position = None;
        self.has_ended = false;
    }

    /// The engine reconnected and the same run goes on: keep the lines, waiting
    /// ones too, and ask for a new stream that goes on after them. Without a
    /// line to go on from, the new stream reads what the old one was to read.
    /// The old stream may still be alive: iced drops it, and its late lines
    /// carry the old key.
    fn go_on(&mut self) {
        let read = match self.position {
            Some(position) => LogRead::After(position),
            None => self.target.read(),
        };

        self.target.next_stream(read);
        self.error = None;
        self.has_ended = false;
    }

    /// The same run ended while the engine was away: keep the lines, start no
    /// stream and forget the old stream's error. That stream keeps its key, so
    /// a failure it still sends has to be told apart from a real one.
    fn end(&mut self) {
        self.error = None;
        self.has_ended = true;
    }

    /// Whether lines are here, on screen or waiting.
    fn has_lines(&self) -> bool {
        self.lines.line_count() > 0 || self.lines.pending_count() > 0
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

/// What the open logs do once their engine has reconnected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogsAfterReconnect {
    /// Another run started in the gap: clear the view and read that run.
    StartOver,
    /// The same run goes on: keep the lines and add those missed.
    GoOn,
    /// The same run ended in the gap: keep the lines, ask nothing more, and the
    /// message of the old stream goes, because the container simply ended.
    Ended,
    /// Nothing to read: keep the lines and ask the engine nothing more.
    Keep,
}

/// The rows of an inspect tree and what they were built from.
struct KeptTree {
    rows: Option<InspectTree>,
    lists_version: u64,
    shows_empty: bool,
}

/// The inspect answer of one row and how its tree is open.
pub(crate) struct LoadedInspect {
    json: LogViewState,
    value: Value,
    open_sections: BTreeSet<String>,
    /// Built when the answer, the lists or the Show empty switch change, not on
    /// every rebuild of the view: it walks the answer and checks its values
    /// against the lists.
    tree: Option<KeptTree>,
}

impl LoadedInspect {
    /// The pretty-printed JSON, shown from its first line.
    pub(crate) fn json(&self) -> &LogViewState {
        &self.json
    }

    /// The titles of the open sections of the tree.
    pub(crate) fn open_sections(&self) -> &BTreeSet<String> {
        &self.open_sections
    }

    /// The rows of the tree; `None` when the answer is not a JSON object, or
    /// while the tree has not been built because nothing shows it.
    pub(crate) fn tree(&self) -> Option<&InspectTree> {
        self.tree.as_ref()?.rows.as_ref()
    }

    /// The answer `json`, with the first shown section of its tree open.
    fn new(json: &str, shows_empty: bool) -> Self {
        let value = serde_json::from_str(json).unwrap_or(Value::Null);

        let first_section =
            first_section_title(&value, shows_empty).map(str::to_string);

        Self {
            json: json_view(json),
            value,
            open_sections: first_section.into_iter().collect(),
            tree: None,
        }
    }

    /// Open the section `title`, or close it when it is open.
    fn toggle_section(&mut self, title: &str) {
        if !self.open_sections.remove(title) {
            self.open_sections.insert(title.to_string());
        }
    }
}

/// Load state of the inspect JSON.
pub(crate) enum InspectLoad {
    /// Asked for, not here yet.
    Loading,
    /// The answer: the JSON and its tree.
    Loaded(Box<LoadedInspect>),
    /// The engine's message.
    Failed(String),
}

impl fmt::Debug for InspectLoad {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Loading => formatter.write_str("Loading"),
            Self::Loaded(loaded) => formatter
                .debug_tuple("Loaded")
                .field(&loaded.json.line_count())
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

/// The inspector's followed logs and the last inspect JSON.
#[derive(Debug, Default)]
pub(crate) struct InspectorState {
    logs: Option<ContainerLogs>,
    inspect: Option<InspectData>,
    is_raw_json: bool,
    shows_empty: bool,
}

impl InspectorState {
    /// Whether the Inspect tab shows the raw JSON instead of the tree.
    pub(crate) fn is_raw_json(&self) -> bool {
        self.is_raw_json
    }

    /// Whether the inspect tree shows empty values.
    pub(crate) fn shows_empty(&self) -> bool {
        self.shows_empty
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

    /// The text of the loaded inspect JSON, for the clipboard.
    pub(crate) fn inspect_json_text(&self) -> Option<String> {
        match self.inspect()?.load() {
            InspectLoad::Loaded(loaded) => Some(loaded.json.text()),
            _ => None,
        }
    }

    /// Follow the logs of `target`, or of nothing. Logs of another container
    /// are dropped; the same container keeps its lines and its stream.
    pub(super) fn follow_logs(&mut self, target: Option<LogTarget>) {
        if let (Some(logs), Some(target)) = (&self.logs, &target)
            && logs.target.is_same_container(target)
        {
            return;
        }

        self.logs = target.map(ContainerLogs::new);
    }

    /// The container `container_id` on the engine at `engine` was started or
    /// restarted: when its logs are followed, they start over with a new
    /// stream. Spec Decision 12: the view is cleared, no old lines are kept.
    pub(super) fn restart_logs(
        &mut self,
        engine: &EngineAddress,
        container_id: &str,
    ) {
        if let Some(logs) = self.logs.as_mut()
            && logs.target.is_container(engine, container_id)
        {
            logs.start_over();
        }
    }

    /// Add lines of `target`, which end at `position`. Returns `false` when
    /// its logs are no longer followed.
    pub(super) fn push_log_lines(
        &mut self,
        target: &LogTarget,
        lines: Vec<String>,
        position: Option<LogPosition>,
    ) -> bool {
        let Some(logs) = self.followed_logs(target) else {
            return false;
        };

        logs.lines.push_lines(lines);
        if position.is_some() {
            logs.position = position;
        }
        true
    }

    /// Remember which run the stream of `target` reads. Returns `false` when
    /// its logs are no longer followed.
    pub(super) fn set_log_run(
        &mut self,
        target: &LogTarget,
        run: ContainerRun,
    ) -> bool {
        let Some(logs) = self.followed_logs(target) else {
            return false;
        };

        logs.shown_run = Some(run);
        true
    }

    /// The engine of `target` reconnected and inspect told `run_now`, or
    /// nothing when it failed. Spec Decision 12: see
    /// [`logs_after_reconnect`]. Returns `false` when the answer is late: the
    /// logs moved to another container or stream meanwhile.
    pub(super) fn settle_logs_after_reconnect(
        &mut self,
        target: &LogTarget,
        run_now: Option<ContainerRun>,
    ) -> bool {
        let Some(logs) = self.followed_logs(target) else {
            return false;
        };

        let has_lines_without_position =
            logs.position.is_none() && logs.has_lines();
        match logs_after_reconnect(
            logs.shown_run,
            run_now,
            has_lines_without_position,
        ) {
            LogsAfterReconnect::StartOver => logs.start_over(),
            LogsAfterReconnect::GoOn => logs.go_on(),
            LogsAfterReconnect::Ended => logs.end(),
            LogsAfterReconnect::Keep => {},
        }
        true
    }

    /// Show that the logs of `target` could not be read. Returns `false`
    /// when its logs are no longer followed or their run has ended.
    pub(super) fn fail_logs(
        &mut self,
        target: &LogTarget,
        error: ContainersError,
    ) -> bool {
        let Some(logs) = self.followed_logs(target) else {
            return false;
        };
        if logs.has_ended {
            return false;
        }

        logs.error = Some(error.to_string());
        true
    }

    /// Apply a selection or scroll action of the log view. Scrolling up
    /// turns Follow off.
    pub(super) fn perform_log_action(&mut self, action: Action) {
        if let Some(logs) = self.logs.as_mut() {
            logs.lines.perform(action);
        }
    }

    /// Turn Follow of the followed logs on or off. Off, new lines wait; on,
    /// they are added and the view goes to the bottom. New logs start with
    /// Follow on, so a tab, selection or mode change resets it, and so does
    /// hiding the inspector.
    pub(super) fn set_log_following(&mut self, is_following: bool) {
        if let Some(logs) = self.logs.as_mut() {
            logs.lines.set_following(is_following);
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
        result: Result<String, ContainersError>,
    ) -> bool {
        let Some(inspect) = self
            .inspect
            .as_mut()
            .filter(|inspect| inspect.target == *target)
        else {
            return false;
        };

        inspect.load = match result {
            Ok(json) => {
                let loaded = LoadedInspect::new(&json, self.shows_empty);
                InspectLoad::Loaded(Box::new(loaded))
            },
            Err(err) => InspectLoad::Failed(err.to_string()),
        };
        true
    }

    /// Forget the kept JSON, with its open sections, when `selected` is not
    /// its row: the open state lasts until the selection changes.
    pub(super) fn forget_inspect_unless(
        &mut self,
        selected: Option<&InspectTarget>,
    ) {
        let is_other = self
            .inspect
            .as_ref()
            .is_some_and(|inspect| Some(&inspect.target) != selected);
        if is_other {
            self.inspect = None;
        }
    }

    /// The loaded answer, when it shows as a tree (the Raw JSON switch is off)
    /// that was not built from the lists at `lists_version` with the current
    /// Show empty switch. The caller checks that the Inspect tab is open.
    pub(super) fn answer_of_old_tree(
        &self,
        lists_version: u64,
    ) -> Option<&Value> {
        if self.is_raw_json {
            return None;
        }
        let InspectLoad::Loaded(loaded) = self.inspect.as_ref()?.load() else {
            return None;
        };

        let is_current = loaded.tree.as_ref().is_some_and(|tree| {
            tree.lists_version == lists_version
                && tree.shows_empty == self.shows_empty
        });
        (!is_current).then_some(&loaded.value)
    }

    /// Keep `rows` as the tree of the loaded answer, built from the lists at
    /// `lists_version` with the current Show empty switch.
    pub(super) fn keep_inspect_tree(
        &mut self,
        rows: Option<InspectTree>,
        lists_version: u64,
    ) {
        let shows_empty = self.shows_empty;

        if let Some(loaded) = self.loaded_inspect_mut() {
            loaded.tree = Some(KeptTree {
                rows,
                lists_version,
                shows_empty,
            });
        }
    }

    /// Apply a selection or scroll action of the inspect view.
    pub(super) fn perform_inspect_action(&mut self, action: Action) {
        if let Some(loaded) = self.loaded_inspect_mut() {
            loaded.json.perform(action);
        }
    }

    /// Open the inspect section `title`, or close it when it is open.
    pub(super) fn toggle_inspect_section(&mut self, title: &str) {
        if let Some(loaded) = self.loaded_inspect_mut() {
            loaded.toggle_section(title);
        }
    }

    /// Turn the Show empty switch of the inspect tree on or off.
    pub(super) fn set_shows_empty(&mut self, shows_empty: bool) {
        self.shows_empty = shows_empty;
    }

    /// Turn the Raw JSON switch of the Inspect tab on or off.
    pub(super) fn set_raw_json(&mut self, is_raw_json: bool) {
        self.is_raw_json = is_raw_json;
    }

    /// The loaded inspect answer, if any.
    fn loaded_inspect_mut(&mut self) -> Option<&mut LoadedInspect> {
        match self.inspect.as_mut()?.load {
            InspectLoad::Loaded(ref mut loaded) => Some(&mut **loaded),
            _ => None,
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

/// What the open logs do after a reconnect, from the run they show (`None`
/// until their stream has read it), the run inspect tells now (`None` when it
/// failed: the container is gone or the engine went away again, so nothing is
/// known about it) and whether lines are here that cannot be told apart from
/// new ones.
fn logs_after_reconnect(
    shown_run: Option<ContainerRun>,
    run_now: Option<ContainerRun>,
    has_lines_without_position: bool,
) -> LogsAfterReconnect {
    let Some(run_now) = run_now else {
        return LogsAfterReconnect::Keep;
    };
    let Some(shown_run) = shown_run else {
        return LogsAfterReconnect::GoOn;
    };

    if !shown_run.is_same_run(&run_now) {
        return LogsAfterReconnect::StartOver;
    }
    if !run_now.is_running() {
        return LogsAfterReconnect::Ended;
    }
    if has_lines_without_position {
        return LogsAfterReconnect::Keep;
    }

    LogsAfterReconnect::GoOn
}

/// A log view with no lines yet, held to [`MAX_LOG_LINES`] and
/// [`MAX_LOG_TEXT_BYTES`], that says where output was skipped.
fn empty_log_lines() -> LogViewState {
    LogViewState::new()
        .max_lines(MAX_LOG_LINES)
        .max_bytes(MAX_LOG_TEXT_BYTES)
        .skipped_output_line(i18n::t(Key::ContainersLogsSkippedOutput))
}

/// The JSON as read-only lines, shown from its first line.
fn json_view(json: &str) -> LogViewState {
    let mut view = LogViewState::new();
    view.push_lines(json.lines());

    view.perform(Action::Move(Motion::DocumentStart));
    view
}

#[cfg(test)]
mod tests {
    use otty_containers::fixtures;

    use super::*;

    const STARTED: i64 = 1_793_750_700;
    const STARTED_AGAIN: i64 = 1_793_750_800;

    #[test]
    fn another_start_time_starts_over() {
        let shown = fixtures::container_run(STARTED, true);
        let now = fixtures::container_run(STARTED_AGAIN, true);

        let decision = logs_after_reconnect(Some(shown), Some(now), false);

        assert_eq!(decision, LogsAfterReconnect::StartOver);
    }

    /// While events were down the container ran again and stopped: the live
    /// app would show that run, once, and the stream ends with the container.
    #[test]
    fn another_start_time_of_a_stopped_container_starts_over_too() {
        let shown = fixtures::container_run(STARTED, true);
        let now = fixtures::container_run(STARTED_AGAIN, false);

        let decision = logs_after_reconnect(Some(shown), Some(now), false);

        assert_eq!(decision, LogsAfterReconnect::StartOver);
    }

    #[test]
    fn the_same_run_still_running_goes_on() {
        let run = fixtures::container_run(STARTED, true);

        let decision = logs_after_reconnect(Some(run), Some(run), false);

        assert_eq!(decision, LogsAfterReconnect::GoOn);
    }

    #[test]
    fn the_same_run_stopped_has_ended() {
        let shown = fixtures::container_run(STARTED, true);
        let now = fixtures::container_run(STARTED, false);

        let decision = logs_after_reconnect(Some(shown), Some(now), false);

        assert_eq!(decision, LogsAfterReconnect::Ended);
    }

    /// Whether the lines can be told apart does not matter once the run ended.
    #[test]
    fn the_same_run_stopped_with_lines_without_a_position_has_ended_too() {
        let shown = fixtures::container_run(STARTED, true);
        let now = fixtures::container_run(STARTED, false);

        let decision = logs_after_reconnect(Some(shown), Some(now), true);

        assert_eq!(decision, LogsAfterReconnect::Ended);
    }

    #[test]
    fn no_answer_keeps_the_lines_whatever_is_shown() {
        let run = fixtures::container_run(STARTED, true);

        assert_eq!(
            logs_after_reconnect(Some(run), None, false),
            LogsAfterReconnect::Keep
        );
        assert_eq!(
            logs_after_reconnect(None, None, false),
            LogsAfterReconnect::Keep
        );
    }

    /// The stream could not even read its run (the engine was away), so
    /// nothing is on screen: read now, also for a stopped container, whose
    /// stream ends by itself.
    #[test]
    fn a_stream_that_never_read_its_run_reads_again() {
        for is_running in [true, false] {
            let now = fixtures::container_run(STARTED, is_running);

            let decision = logs_after_reconnect(None, Some(now), false);

            assert_eq!(decision, LogsAfterReconnect::GoOn);
        }
    }

    /// Lines without a time cannot be told from new ones, and repeating them
    /// is worse than waiting for the next start.
    #[test]
    fn lines_without_a_position_are_not_read_again() {
        let run = fixtures::container_run(STARTED, true);

        let decision = logs_after_reconnect(Some(run), Some(run), true);

        assert_eq!(decision, LogsAfterReconnect::Keep);
    }

    /// A restart is told apart before the lines matter: they go anyway.
    #[test]
    fn lines_without_a_position_do_not_hold_back_a_restart() {
        let shown = fixtures::container_run(STARTED, true);
        let now = fixtures::container_run(STARTED_AGAIN, true);

        let decision = logs_after_reconnect(Some(shown), Some(now), true);

        assert_eq!(decision, LogsAfterReconnect::StartOver);
    }
}
