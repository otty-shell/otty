//! Following a container's logs for the inspector's Logs tab (spec
//! Decision 12).
//!
//! The engine's answer holds stdout and stderr frames (or plain text for a
//! container with a terminal); they are kept in the order they came and cut
//! into lines here (`log_answer.rs`). bollard's 30 s request limit covers only
//! the wait for the answer to start, not the stream after it, so one client
//! serves both the lists and a long follow.
//!
//! Both engines keep the lines of earlier runs of a container. On a restart
//! Docker ends the stream; Podman ends it or, with its journald log driver,
//! keeps it going (seen on Docker 29.9.1 and Podman 5.8.8). The app therefore
//! goes by the engine's start event, not by the end of a stream.
//!
//! Every line is read with its time. That tells where a stream stands, so a
//! later stream can go on from there after a broken connection.
//!
//! A run longer than [`MAX_LOG_LINE_BYTES`] is shown as several lines, and
//! each of them has a position of its own (see [`LogPosition`]).

use std::str;

use bollard::models::ContainerState;
use bollard::query_parameters::InspectContainerOptions;
use futures_util::stream::{self, BoxStream, StreamExt, TryStreamExt};

use super::{ContainerRun, LogLine, LogPosition, MAX_LOG_LINE_BYTES};
use crate::connection::{Engine, EngineError};
use crate::log_answer::{
    AnswerLine, LogAnswer, MAX_WAITING_CHAR_BYTES, OutputKind,
};
use crate::timestamp::{NANOS_PER_SECOND, unix_nanos};

impl LogPosition {
    /// The `nth_at_time`-th line (from 1) with the time `time` (nanoseconds
    /// since the Unix epoch).
    pub(crate) fn new(time: i64, nth_at_time: u32) -> Self {
        Self {
            time,
            nth_at_time,
            piece: 0,
        }
    }

    /// The position right after this one, for a line without a time of its
    /// own.
    fn next_piece(self) -> Self {
        Self {
            piece: self.piece.saturating_add(1),
            ..self
        }
    }
}

impl LogLine {
    /// A line with `text` that ends at `position`.
    pub(crate) fn new(text: String, position: Option<LogPosition>) -> Self {
        Self { text, position }
    }
}

impl ContainerRun {
    /// A run with these facts; `started_at` in nanoseconds since the Unix
    /// epoch.
    pub(crate) fn new(
        started_at: Option<i64>,
        cutoff: Option<i64>,
        is_running: bool,
    ) -> Self {
        Self {
            started_at,
            cutoff,
            is_running,
        }
    }

    /// The run the engine's `state` tells about.
    fn from_state(state: &ContainerState) -> Self {
        let started_at = state.started_at.as_deref();
        let finished_at = state.finished_at.as_deref();

        Self::new(
            started_at.and_then(unix_nanos),
            run_cutoff(started_at, finished_at),
            state.running == Some(true),
        )
    }
}

/// Reads the `<time> <text>` lines of one log stream in order.
#[derive(Debug, Default)]
struct LineReader {
    /// Lines older than this are of an earlier run.
    cutoff: Option<i64>,
    /// Lines up to here are on screen already.
    after: Option<LogPosition>,
    /// The position of the last line read.
    last: Option<LogPosition>,
    /// The line that was cut is of an earlier run, so its pieces are too.
    drops_pieces: bool,
    /// The start of a character at the end of the last open stdout line.
    waiting_stdout: Vec<u8>,
    /// The same for stderr: the two outputs come mixed.
    waiting_stderr: Vec<u8>,
}

impl LineReader {
    /// A reader that drops lines older than `cutoff`.
    fn of_run(cutoff: Option<i64>) -> Self {
        Self {
            cutoff,
            ..Self::default()
        }
    }

    /// A reader that drops lines up to `after`.
    fn after(after: LogPosition) -> Self {
        Self {
            after: Some(after),
            ..Self::default()
        }
    }

    /// The lines to show for `line`, which still has its time: none when it
    /// must be dropped, several when its text is too long for one.
    fn read_line(&mut self, line: AnswerLine) -> Vec<LogLine> {
        let (kind, is_continuation, is_open) =
            (line.kind(), line.is_continuation(), line.is_open());
        let mut bytes = line.into_bytes();

        if is_continuation && self.drops_pieces {
            return Vec::new();
        }

        let time = if is_continuation {
            None
        } else {
            take_time(&mut bytes)
        };
        self.drops_pieces =
            time.is_some_and(|time| self.cutoff.is_some_and(|cut| time < cut));
        if self.drops_pieces {
            return Vec::new();
        }

        let text = self.text_of(bytes, kind, is_open);
        let mut position = match time {
            Some(time) => Some(next_position(self.last, time)),
            None => self.last.map(LogPosition::next_piece),
        };

        let mut lines = Vec::new();
        for piece in pieces_of(text) {
            if position.is_some() {
                self.last = position;
            }
            if !self.is_seen(position) {
                lines.push(LogLine::new(piece, position));
            }
            position = position.map(LogPosition::next_piece);
        }

        lines
    }

    /// The text of `bytes`, with the start of a character that waited for them
    /// in front. An open line may end in the start of a character; that waits
    /// for the next line of `kind`.
    fn text_of(
        &mut self,
        mut bytes: Vec<u8>,
        kind: OutputKind,
        is_open: bool,
    ) -> String {
        let waiting = match kind {
            OutputKind::Stdout => &mut self.waiting_stdout,
            OutputKind::Stderr => &mut self.waiting_stderr,
        };

        if !waiting.is_empty() {
            waiting.append(&mut bytes);
            bytes = std::mem::take(waiting);
        }
        if is_open {
            let text_end = bytes.len() - unfinished_char_len(&bytes);
            *waiting = bytes.split_off(text_end);
        }

        String::from_utf8(bytes).unwrap_or_else(|err| {
            String::from_utf8_lossy(err.as_bytes()).into_owned()
        })
    }

    /// Whether a line at `position` is on screen already. A line without a
    /// position comes before the first time, so ahead of any position.
    fn is_seen(&self, position: Option<LogPosition>) -> bool {
        self.after.is_some_and(|after| {
            position.is_none_or(|position| position <= after)
        })
    }
}

#[cfg(test)]
impl LineReader {
    /// What `read_line` shows first for one whole line.
    fn read(&mut self, line: &str) -> Option<LogLine> {
        self.read_line(AnswerLine::whole(line)).into_iter().next()
    }
}

impl Engine {
    /// What inspect tells about the container's latest run.
    pub async fn container_run(
        &self,
        id: &str,
    ) -> Result<ContainerRun, EngineError> {
        let answer = self
            .client()
            .inspect_container(id, None::<InspectContainerOptions>)
            .await?;

        Ok(ContainerRun::from_state(&answer.state.unwrap_or_default()))
    }

    /// The last `tail` lines of the container with `id` (stdout and stderr),
    /// then every new line until the container stops or the stream is
    /// dropped. An engine error ends the stream with that error.
    pub fn follow_logs(
        &self,
        id: &str,
        tail: usize,
    ) -> BoxStream<'static, Result<LogLine, EngineError>> {
        self.read_lines(id, tail, None, LineReader::default())
    }

    /// Like [`Self::follow_logs`], for a container that just came back: only
    /// the lines of its latest `run`. The engine keeps the lines of earlier
    /// runs and it is asked for lines since a whole second only, which still
    /// holds the old run's last lines after a fast restart. So the
    /// lines are read with their times and the older ones are dropped here.
    ///
    /// ```no_run
    /// # async fn run(engine: otty_containers::Engine) -> Result<(), otty_containers::EngineError> {
    /// use futures_util::StreamExt;
    ///
    /// let run = engine.container_run("c178aa0f").await?;
    /// let mut lines = engine.follow_latest_run_logs("c178aa0f", &run, 500);
    /// while let Some(Ok(line)) = lines.next().await {
    ///     println!("{}", line.text());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn follow_latest_run_logs(
        &self,
        id: &str,
        run: &ContainerRun,
        tail: usize,
    ) -> BoxStream<'static, Result<LogLine, EngineError>> {
        self.read_lines(id, tail, run.cutoff, LineReader::of_run(run.cutoff))
    }

    /// Like [`Self::follow_logs`], going on after a broken connection: only
    /// the lines after `after`, at most the last `tail` of them. The engine is
    /// asked for lines since a whole second only, so the lines of that second
    /// come again and are dropped here by their exact position.
    ///
    /// ```no_run
    /// # async fn run(engine: otty_containers::Engine) {
    /// use futures_util::StreamExt;
    ///
    /// let mut first = engine.follow_logs("c178aa0f", 500);
    /// let Some(Ok(line)) = first.next().await else {
    ///     return;
    /// };
    /// drop(first);
    ///
    /// if let Some(position) = line.position() {
    ///     let mut rest = engine.follow_logs_after("c178aa0f", position, 5000);
    ///     while let Some(Ok(line)) = rest.next().await {
    ///         println!("{}", line.text());
    ///     }
    /// }
    /// # }
    /// ```
    pub fn follow_logs_after(
        &self,
        id: &str,
        after: LogPosition,
        tail: usize,
    ) -> BoxStream<'static, Result<LogLine, EngineError>> {
        self.read_lines(id, tail, Some(after.time), LineReader::after(after))
    }

    /// Follow the lines from `since` (nanoseconds since the Unix epoch) with
    /// their times, through `reader`.
    ///
    /// bollard's `logs` call cannot be used: for a container with a terminal
    /// it holds the text until a line break comes, without any limit, and it
    /// holds a whole frame however long. No public call of bollard takes a
    /// logs request and gives the bytes as they come. `export_container` gives
    /// the bytes of `/containers/{name}/export`, so the name here makes that
    /// the logs request and leaves `/export` to the fragment after `#`, which
    /// is never sent. `the_engine_sees_a_plain_logs_request` checks this.
    fn read_lines(
        &self,
        id: &str,
        tail: usize,
        since: Option<i64>,
        mut reader: LineReader,
    ) -> BoxStream<'static, Result<LogLine, EngineError>> {
        let since = since
            .and_then(whole_second_before)
            .map_or_else(String::new, |since| format!("&since={since}"));
        let request = format!(
            "{id}/logs?follow=true&stdout=true&stderr=true&timestamps=true&tail={tail}{since}#"
        );
        let mut answer = LogAnswer::default();

        self.client()
            .export_container(&request)
            .map_err(EngineError::from)
            .map_ok(Some)
            .chain(stream::once(async { Ok(None) }))
            .map_ok(move |chunk| {
                let answer_lines = match chunk {
                    Some(chunk) => answer.read(&chunk),
                    None => answer.finish(),
                };
                let lines = answer_lines
                    .into_iter()
                    .flat_map(|line| reader.read_line(line))
                    .collect::<Vec<_>>();
                stream::iter(lines).map(Ok)
            })
            .try_flatten()
            .boxed()
    }
}

/// The moment (nanoseconds since the Unix epoch) that divides the latest run's
/// lines from older ones, from the container's `StartedAt` and `FinishedAt`:
/// halfway between the end of the last run and the new start, because the
/// engines stamp lines a little off those two times (spec Decision 12) and
/// the gap between them is much wider. `None` when nothing has to be cut: the
/// container never stopped before, or the times are unknown. A `FinishedAt`
/// after `StartedAt` is the end of the latest run itself, which is over
/// already, so it is cut at its start.
fn run_cutoff(
    started_at: Option<&str>,
    finished_at: Option<&str>,
) -> Option<i64> {
    let started = unix_nanos(started_at?)?;
    let finished = unix_nanos(finished_at?)?;

    if finished > started {
        return Some(started);
    }

    Some(finished + (started - finished) / 2)
}

/// The position of a line with the time `time` after a line at `last`. It
/// never goes back, so a line stamped before the one ahead of it goes right
/// after that one.
fn next_position(last: Option<LogPosition>, time: i64) -> LogPosition {
    match last {
        Some(last) if time < last.time => last.next_piece(),
        Some(last) if time == last.time => {
            LogPosition::new(time, last.nth_at_time.saturating_add(1))
        },
        _ => LogPosition::new(time, 1),
    }
}

/// `nanos` as the whole second at or before it, for the engine's `since`;
/// `None` when it does not fit.
fn whole_second_before(nanos: i64) -> Option<i32> {
    i32::try_from(nanos.div_euclid(NANOS_PER_SECOND)).ok()
}

/// Take the time at the start of `bytes` and the space after it; `None`, with
/// `bytes` as they were, when they do not start with a time.
fn take_time(bytes: &mut Vec<u8>) -> Option<i64> {
    let end = bytes.iter().position(|byte| *byte == b' ')?;
    let time = unix_nanos(str::from_utf8(&bytes[..end]).ok()?)?;

    bytes.drain(..=end);
    Some(time)
}

/// How many bytes at the end of `bytes` are the start of a character whose
/// rest has not come yet.
fn unfinished_char_len(bytes: &[u8]) -> usize {
    let is_later_byte_of_char = |byte: &u8| *byte & 0b1100_0000 == 0b1000_0000;
    let Some(later_bytes) = bytes
        .iter()
        .rev()
        .take(MAX_WAITING_CHAR_BYTES)
        .position(|byte| !is_later_byte_of_char(byte))
    else {
        return 0;
    };
    let char_start = bytes.len() - later_bytes - 1;

    match str::from_utf8(&bytes[char_start..]) {
        Err(err) if err.error_len().is_none() => bytes.len() - char_start,
        _ => 0,
    }
}

/// `text` as pieces of at most [`MAX_LOG_LINE_BYTES`], cut between characters.
/// Text can be longer than what was cut before it was read: each bad byte
/// becomes a replacement character of three bytes.
fn pieces_of(mut text: String) -> Vec<String> {
    let mut pieces = Vec::with_capacity(1);

    while text.len() > MAX_LOG_LINE_BYTES {
        let rest = text.split_off(text.floor_char_boundary(MAX_LOG_LINE_BYTES));
        pieces.push(text);
        text = rest;
    }
    pieces.push(text);

    pieces
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::thread::JoinHandle;

    use bollard::models::SystemVersion;

    use super::*;
    use crate::address::EngineAddress;
    use crate::engine::EngineInfo;
    use crate::test_support::{TempDir, read_request_line, write_response};

    /// Stream kinds of the engine's multiplexed log format.
    const STDOUT: u8 = 1;
    const STDERR: u8 = 2;

    fn engine_at(path: &Path) -> Engine {
        let answer: SystemVersion =
            serde_json::from_str(r#"{"Version":"1.0","ApiVersion":"1.44"}"#)
                .expect("version answer");
        let info = EngineInfo::from_version(
            &answer,
            EngineAddress::Unix(path.to_path_buf()),
        );

        Engine::connect(&info).expect("engine")
    }

    /// One frame of the multiplexed log format: kind, three zero bytes, the
    /// big-endian length, then the text.
    fn frame(kind: u8, text: &str) -> Vec<u8> {
        frame_bytes(kind, text.as_bytes())
    }

    /// Like [`frame`], for text that may not be valid UTF-8.
    fn frame_bytes(kind: u8, text: &[u8]) -> Vec<u8> {
        let length = u32::try_from(text.len()).expect("short text");
        let mut frame = vec![kind, 0, 0, 0];
        frame.extend_from_slice(&length.to_be_bytes());
        frame.extend_from_slice(text);
        frame
    }

    /// Answer one request with a log `body` and close the connection, as an
    /// engine does when the container stops; return the request line.
    fn serve_logs(
        listener: UnixListener,
        content_type: &'static str,
        body: Vec<u8>,
    ) -> JoinHandle<String> {
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let Some(line) = read_request_line(&mut stream) else {
                    continue;
                };
                let length = body.len();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
                return line;
            }
            String::new()
        })
    }

    /// Answer the inspect request with `inspect`, then the logs request with
    /// a log `body`, on one connection each; return both request lines.
    fn serve_run(
        listener: UnixListener,
        inspect: &'static str,
        body: Vec<u8>,
    ) -> JoinHandle<Vec<String>> {
        std::thread::spawn(move || {
            let mut requests = Vec::new();
            for mut stream in listener.incoming().flatten() {
                let Some(line) = read_request_line(&mut stream) else {
                    continue;
                };
                let is_logs = line.contains("/logs");
                requests.push(line);
                if !is_logs {
                    write_response(&mut stream, "200 OK", inspect);
                    continue;
                }
                let length = body.len();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.docker.multiplexed-stream\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
                break;
            }
            requests
        })
    }

    /// The text the reader of a run shows for `line`.
    fn line_of_run(line: &str, cutoff: Option<i64>) -> Option<String> {
        LineReader::of_run(cutoff)
            .read(line)
            .map(LogLine::into_text)
    }

    /// What a reader shows of `lines`, as text.
    fn shown(reader: &mut LineReader, lines: &[&str]) -> Vec<String> {
        lines
            .iter()
            .filter_map(|line| reader.read(line))
            .map(LogLine::into_text)
            .collect()
    }

    /// What `reader` shows of a whole answer in `chunks`.
    fn read_answer(reader: &mut LineReader, chunks: &[&[u8]]) -> Vec<LogLine> {
        let mut answer = LogAnswer::default();
        let mut lines = Vec::new();
        for chunk in chunks {
            for line in answer.read(chunk) {
                lines.extend(reader.read_line(line));
            }
        }
        for line in answer.finish() {
            lines.extend(reader.read_line(line));
        }
        lines
    }

    /// All the text of `lines`, as one run.
    fn joined(lines: &[LogLine]) -> String {
        lines.iter().map(LogLine::text).collect()
    }

    /// The texts of log lines.
    fn texts(lines: &[LogLine]) -> Vec<&str> {
        lines.iter().map(LogLine::text).collect()
    }

    /// The times seen on Docker 29.9.1 across a `docker restart`.
    const STARTED_AT: &str = "2026-11-03T17:46:21.956057283Z";
    const FINISHED_AT: &str = "2026-11-03T17:46:21.857758301Z";

    #[test]
    fn the_cutoff_lies_between_the_end_of_the_old_run_and_the_new_start() {
        let cutoff = run_cutoff(Some(STARTED_AT), Some(FINISHED_AT));

        assert_eq!(cutoff, Some(1_793_727_981_906_907_792));
    }

    /// Podman stamps the old run's last line after `FinishedAt` and can write
    /// the first new line right at `StartedAt`, so neither one is the cutoff.
    #[test]
    fn lines_close_to_either_end_of_the_gap_stay_on_their_side() {
        let cutoff = run_cutoff(
            Some("2026-11-03T18:47:22.378999347+01:00"),
            Some("2026-11-03T18:47:22.315940239+01:00"),
        );

        let old = "2026-11-03T18:47:22.316419000+01:00 bye";
        let new = "2026-11-03T18:47:22.378000000+01:00 hello";
        assert_eq!(line_of_run(old, cutoff), None);
        assert_eq!(line_of_run(new, cutoff).as_deref(), Some("hello"));
    }

    #[test]
    fn a_first_run_has_no_cutoff_because_nothing_is_older() {
        let cutoff = run_cutoff(Some(STARTED_AT), Some("0001-01-01T00:00:00Z"));

        assert_eq!(cutoff, None);
    }

    /// The run was already over when OTTY asked: `FinishedAt` is its own end.
    #[test]
    fn a_run_that_is_over_is_cut_at_its_start() {
        let cutoff = run_cutoff(
            Some("2026-11-03T17:46:21Z"),
            Some("2026-11-03T17:46:22Z"),
        );

        assert_eq!(cutoff, Some(1_793_727_981_000_000_000));
    }

    #[test]
    fn unknown_run_times_give_no_cutoff() {
        assert_eq!(run_cutoff(None, Some(FINISHED_AT)), None);
        assert_eq!(run_cutoff(Some(STARTED_AT), None), None);
        assert_eq!(run_cutoff(Some("soon"), Some(FINISHED_AT)), None);
        assert_eq!(run_cutoff(Some("0001-01-01T00:00:00Z"), None), None);
    }

    #[test]
    fn a_line_loses_its_time_and_keeps_its_own_spaces() {
        let line = "2026-11-03T17:46:23.493268440Z  two  spaces ";

        assert_eq!(line_of_run(line, None).as_deref(), Some(" two  spaces "));
    }

    #[test]
    fn an_empty_line_of_the_run_stays_empty() {
        let line = "2026-11-03T17:46:23.493268440Z ";

        assert_eq!(line_of_run(line, None).as_deref(), Some(""));
    }

    #[test]
    fn a_line_at_the_cutoff_or_later_is_kept_and_an_older_one_dropped() {
        let cutoff = Some(1_793_727_983_000_000_000);

        let older = "2026-11-03T17:46:22.999999999Z old";
        let exact = "2026-11-03T17:46:23Z exact";
        let later = "2026-11-03T17:46:23.000000001Z later";
        assert_eq!(line_of_run(older, cutoff), None);
        assert_eq!(line_of_run(exact, cutoff).as_deref(), Some("exact"));
        assert_eq!(line_of_run(later, cutoff).as_deref(), Some("later"));
    }

    #[test]
    fn a_line_without_a_time_is_kept_whole() {
        let cutoff = Some(1_793_727_983_000_000_000);

        for line in ["no time here", "tail of a long line", ""] {
            assert_eq!(line_of_run(line, cutoff).as_deref(), Some(line));
        }
    }

    /// Podman's `k8s-file` gives one time to all that was written at once;
    /// these are its real times.
    const SHARED_TIME: &str = "2026-11-04T01:05:08.203276723+01:00";
    const SHARED_NANOS: i64 = 1_793_750_708_203_276_723;

    #[test]
    fn a_position_goes_up_with_the_time_and_within_one_time() {
        let early = LogPosition::new(10, 2);
        let same_time_later = LogPosition::new(10, 3);
        let later = LogPosition::new(11, 1);

        assert!(early < same_time_later);
        assert!(same_time_later < later);
    }

    #[test]
    fn each_line_says_where_the_stream_stands() {
        let mut reader = LineReader::default();

        let lines = [
            format!("{SHARED_TIME} a"),
            format!("{SHARED_TIME} b"),
            String::from("2026-11-04T00:05:09Z c"),
        ];
        let positions: Vec<Option<LogPosition>> = lines
            .iter()
            .filter_map(|line| reader.read(line))
            .map(|line| line.position())
            .collect();

        assert_eq!(
            positions,
            [
                Some(LogPosition::new(SHARED_NANOS, 1)),
                Some(LogPosition::new(SHARED_NANOS, 2)),
                Some(LogPosition::new(1_793_750_709_000_000_000, 1)),
            ]
        );
    }

    #[test]
    fn a_line_without_a_time_goes_right_after_the_last_position() {
        let mut reader = LineReader::default();

        let before = reader.read("no time yet").expect("a line");
        let timed = reader.read("2026-11-04T01:05:09Z timed").expect("a line");
        let after = reader.read("no time again").expect("a line");
        let next = reader.read("2026-11-04T01:05:09Z next").expect("a line");

        assert_eq!(before.position(), None);
        assert!(timed.position().is_some());
        assert!(after.position() > timed.position());
        assert!(next.position() > after.position());
    }

    /// stdout and stderr are copied apart, so one can be stamped a little
    /// before a line written ahead of it. The position never goes back: going
    /// back would bring the newer line again after a reconnect.
    #[test]
    fn an_older_time_does_not_move_the_position_back() {
        let mut reader = LineReader::default();

        let newer = reader.read("2026-11-04T01:05:09Z newer").expect("line");
        let older = reader.read("2026-11-04T01:05:08Z older").expect("line");

        assert_eq!(older.text(), "older");
        assert!(newer.position().is_some());
        assert!(older.position() > newer.position());
    }

    /// The stream broke after the newer line. The older line behind it was not
    /// on screen yet, so it comes on the next stream.
    #[test]
    fn a_line_with_an_older_time_is_not_lost_when_the_stream_goes_on() {
        let mut reader = LineReader::default();
        let newer = reader.read("2026-11-04T01:05:09Z newer").expect("line");
        let position = newer.position().expect("a position");

        let mut next = LineReader::after(position);
        let lines = shown(
            &mut next,
            &["2026-11-04T01:05:09Z newer", "2026-11-04T01:05:08Z older"],
        );

        assert_eq!(lines, ["older"]);
    }

    /// Text that is sure to be cut, and a line after it with the same time.
    fn long_answer(text: &str) -> Vec<u8> {
        let mut body = format!("{SHARED_TIME} {text}\n").into_bytes();
        body.extend(format!("{SHARED_TIME} next\n").into_bytes());
        frame_bytes(STDOUT, &body)
    }

    #[test]
    fn a_long_line_becomes_several_that_keep_all_of_its_text() {
        let text = "x".repeat(3 * MAX_LOG_LINE_BYTES + 10);

        let lines =
            read_answer(&mut LineReader::default(), &[&long_answer(&text)]);

        assert_eq!(lines.len(), 5);
        assert!(
            lines
                .iter()
                .all(|line| line.text().len() <= MAX_LOG_LINE_BYTES)
        );
        assert_eq!(lines[4].text(), "next");
        assert_eq!(joined(&lines[..4]), text);
    }

    #[test]
    fn a_long_line_is_never_cut_inside_a_character() {
        for prefix in ["", "a", "ab", "abc"] {
            let text = format!("{prefix}{}", "語😀é".repeat(30_000));

            let lines =
                read_answer(&mut LineReader::default(), &[&long_answer(&text)]);

            assert!(lines.len() > 2);
            assert!(
                lines
                    .iter()
                    .all(|line| line.text().len() <= MAX_LOG_LINE_BYTES)
            );
            assert!(!lines.iter().any(|line| line.text().contains('\u{FFFD}')));
            assert_eq!(joined(&lines[..lines.len() - 1]), text);
        }
    }

    /// Docker cuts a long line by bytes into frames, each with its own time.
    #[test]
    fn a_character_across_two_frames_stays_whole() {
        let text = "語";
        let (start, end) = text.as_bytes().split_at(2);
        let mut first = format!("{SHARED_TIME} before ").into_bytes();
        first.extend_from_slice(start);
        let mut second = format!("{SHARED_TIME} ").into_bytes();
        second.extend_from_slice(end);
        second.extend_from_slice(b" after\n");
        let mut body = frame_bytes(STDOUT, &first);
        body.extend(frame_bytes(STDOUT, &second));

        let lines = read_answer(&mut LineReader::default(), &[&body]);

        assert_eq!(texts(&lines), ["before ", "語 after"]);
    }

    #[test]
    fn a_character_across_two_frames_waits_past_the_other_output() {
        let (start, end) = "é".as_bytes().split_at(1);
        let mut first = format!("{SHARED_TIME} out ").into_bytes();
        first.extend_from_slice(start);
        let mut last = format!("{SHARED_TIME} ").into_bytes();
        last.extend_from_slice(end);
        last.push(b'\n');
        let mut body = frame_bytes(STDOUT, &first);
        body.extend(frame_bytes(
            STDERR,
            format!("{SHARED_TIME} err\n").as_bytes(),
        ));
        body.extend(frame_bytes(STDOUT, &last));

        let lines = read_answer(&mut LineReader::default(), &[&body]);

        assert_eq!(texts(&lines), ["out ", "err", "é"]);
    }

    #[test]
    fn a_character_cut_by_the_chunks_of_the_answer_stays_whole() {
        let body =
            frame_bytes(STDOUT, format!("{SHARED_TIME} 語語\n").as_bytes());
        let chunks: Vec<&[u8]> = body.chunks(1).collect();

        let lines = read_answer(&mut LineReader::default(), &chunks);

        assert_eq!(texts(&lines), ["語語"]);
    }

    #[test]
    fn broken_text_is_shown_as_replacement_and_a_broken_end_does_not_wait() {
        let mut bytes = format!("{SHARED_TIME} bad ").into_bytes();
        bytes.extend_from_slice(&[0xFF, b'x', 0xE8, 0xAA]);
        bytes.push(b'\n');
        bytes.extend_from_slice(format!("{SHARED_TIME} next\n").as_bytes());

        let lines = read_answer(
            &mut LineReader::default(),
            &[&frame_bytes(STDOUT, &bytes)],
        );

        assert_eq!(texts(&lines), ["bad \u{FFFD}x\u{FFFD}", "next"]);
    }

    /// One bad byte becomes a three-byte replacement, so what the engine sent
    /// can grow when it is read as text.
    #[test]
    fn a_long_run_of_broken_bytes_still_fits_the_line_limit() {
        let bytes = vec![0xFF_u8; MAX_LOG_LINE_BYTES];

        let lines = read_answer(
            &mut LineReader::default(),
            &[&frame_bytes(STDOUT, &bytes)],
        );

        assert!(lines.len() >= 3);
        assert!(
            lines
                .iter()
                .all(|line| line.text().len() <= MAX_LOG_LINE_BYTES)
        );
        assert_eq!(
            lines
                .iter()
                .map(|line| line.text().chars().count())
                .sum::<usize>(),
            bytes.len()
        );
    }

    #[test]
    fn the_pieces_of_a_long_line_each_have_a_later_position() {
        let text = "x".repeat(3 * MAX_LOG_LINE_BYTES);

        let lines =
            read_answer(&mut LineReader::default(), &[&long_answer(&text)]);

        let positions: Vec<LogPosition> = lines
            .iter()
            .map(|line| line.position().expect("a position"))
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(positions[0], LogPosition::new(SHARED_NANOS, 1));
        assert_eq!(
            positions[positions.len() - 1],
            LogPosition::new(SHARED_NANOS, 2)
        );
    }

    /// A stream can break after any piece. The next stream reads the whole line
    /// again, so what it shows must be exactly what was still missing.
    #[test]
    fn going_on_after_any_piece_neither_loses_nor_repeats_text() {
        let text = "語".repeat(70_000);
        let body = long_answer(&text);
        let lines = read_answer(&mut LineReader::default(), &[&body]);
        assert!(lines.len() > 3);

        for (index, line) in lines.iter().enumerate() {
            let position = line.position().expect("a position");

            let rest = read_answer(&mut LineReader::after(position), &[&body]);

            assert_eq!(rest, lines[index + 1..], "after piece {index}");
        }
    }

    /// A stream of a terminal that never sends a line break has no later time
    /// to go on from, only its pieces.
    #[test]
    fn going_on_inside_a_run_without_a_line_break_gives_only_the_rest() {
        let mut body = format!("{SHARED_TIME} ").into_bytes();
        body.extend(vec![b'x'; 3 * MAX_LOG_LINE_BYTES]);
        let lines = read_answer(&mut LineReader::default(), &[&body]);
        assert!(lines.len() > 2);
        let position = lines[1].position().expect("a position");

        let rest = read_answer(&mut LineReader::after(position), &[&body]);

        assert_eq!(rest, lines[2..]);
    }

    /// The pieces after the first have no time of their own, but they are of
    /// the older run too.
    #[test]
    fn the_pieces_of_a_line_of_an_older_run_are_dropped_with_it() {
        let cutoff = Some(SHARED_NANOS + 1);
        let old = format!("{SHARED_TIME} {}\n", "x".repeat(200_000));
        let mut body = frame_bytes(STDOUT, old.as_bytes());
        body.extend(frame_bytes(STDOUT, b"2026-11-04T00:05:09Z new\n"));

        let lines = read_answer(&mut LineReader::of_run(cutoff), &[&body]);

        assert_eq!(texts(&lines), ["new"]);
    }

    /// A piece can start with what looks like a time, and it is still text.
    #[test]
    fn a_piece_that_starts_like_a_time_keeps_its_text() {
        let piece_room = "x".repeat(
            MAX_LOG_LINE_BYTES - MAX_WAITING_CHAR_BYTES - SHARED_TIME.len() - 1,
        );
        let text = format!("{piece_room}2026-11-04T00:05:09Z rest");
        let line = format!("{SHARED_TIME} {text}\n");

        let lines = read_answer(
            &mut LineReader::default(),
            &[&frame_bytes(STDOUT, line.as_bytes())],
        );

        assert_eq!(texts(&lines)[1], "2026-11-04T00:05:09Z rest");
    }

    #[test]
    fn a_stream_without_a_line_break_gives_bounded_lines_and_keeps_the_newest()
    {
        let mut reader = LineReader::default();
        let mut answer = LogAnswer::default();
        let chunk = vec![b'x'; 64 * 1024];
        let mut last = String::new();

        for _ in 0..500 {
            for line in answer.read(&chunk) {
                for shown in reader.read_line(line) {
                    assert!(shown.text().len() <= MAX_LOG_LINE_BYTES);
                    last = shown.into_text();
                }
            }
        }

        assert!(!last.is_empty());
    }

    #[test]
    fn going_on_after_a_position_drops_what_came_before_it() {
        let position = LogPosition::new(1_793_750_709_000_000_000, 1);
        let mut reader = LineReader::after(position);

        let lines = shown(
            &mut reader,
            &[
                "2026-11-04T00:05:08Z seen long ago",
                "2026-11-04T00:05:09Z seen last",
                "2026-11-04T00:05:09.000000001Z missed",
                "2026-11-04T00:05:10Z new",
            ],
        );

        assert_eq!(lines, ["missed", "new"]);
    }

    /// The connection broke in the middle of what shares one time: the first
    /// two came, the third did not. Going by the time alone repeats or loses.
    #[test]
    fn going_on_inside_one_time_gives_only_the_lines_not_seen() {
        let mut reader = LineReader::after(LogPosition::new(SHARED_NANOS, 2));

        let lines = shown(
            &mut reader,
            &[
                &format!("{SHARED_TIME} a"),
                &format!("{SHARED_TIME} b"),
                &format!("{SHARED_TIME} c"),
            ],
        );

        assert_eq!(lines, ["c"]);
    }

    #[test]
    fn going_on_at_the_very_end_shows_nothing() {
        let mut reader = LineReader::after(LogPosition::new(SHARED_NANOS, 2));

        let lines = shown(
            &mut reader,
            &[&format!("{SHARED_TIME} a"), &format!("{SHARED_TIME} b")],
        );

        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn a_line_without_a_time_is_dropped_only_among_lines_already_seen() {
        let position = LogPosition::new(1_793_750_709_000_000_000, 1);
        let mut reader = LineReader::after(position);

        let lines = shown(
            &mut reader,
            &[
                "piece of a seen line",
                "2026-11-04T00:05:09Z seen last",
                "piece of a later line",
            ],
        );

        assert_eq!(lines, ["piece of a later line"]);
    }

    #[test]
    fn a_new_start_is_another_run_whatever_the_zone_it_is_written_in() {
        let state = |started: &str, running: bool| ContainerState {
            started_at: Some(started.to_string()),
            running: Some(running),
            ..ContainerState::default()
        };

        let run = ContainerRun::from_state(&state(
            "2026-11-04T00:05:06.599771794Z",
            true,
        ));
        let same_in_a_zone = ContainerRun::from_state(&state(
            "2026-11-04T01:05:06.599771794+01:00",
            true,
        ));
        let next = ContainerRun::from_state(&state(
            "2026-11-04T00:05:06.599771795Z",
            true,
        ));

        assert!(run.is_same_run(&same_in_a_zone));
        assert!(!run.is_same_run(&next));
    }

    #[test]
    fn a_run_says_whether_the_container_is_running() {
        let running = ContainerRun::from_state(&ContainerState {
            running: Some(true),
            ..ContainerState::default()
        });
        let stopped = ContainerRun::from_state(&ContainerState {
            running: Some(false),
            ..ContainerState::default()
        });
        let unknown = ContainerRun::from_state(&ContainerState::default());

        assert!(running.is_running());
        assert!(!stopped.is_running());
        assert!(!unknown.is_running());
    }

    #[test]
    fn a_container_never_started_differs_from_any_started_run() {
        let never = ContainerRun::from_state(&ContainerState {
            started_at: Some(String::from("0001-01-01T00:00:00Z")),
            ..ContainerState::default()
        });
        let started = ContainerRun::from_state(&ContainerState {
            started_at: Some(String::from(STARTED_AT)),
            ..ContainerState::default()
        });

        assert!(!never.is_same_run(&started));
    }

    #[test]
    fn a_run_cuts_its_lines_at_the_middle_of_the_gap_before_it() {
        let run = ContainerRun::from_state(&ContainerState {
            started_at: Some(String::from(STARTED_AT)),
            finished_at: Some(String::from(FINISHED_AT)),
            ..ContainerState::default()
        });

        assert_eq!(run.cutoff, run_cutoff(Some(STARTED_AT), Some(FINISHED_AT)));
        assert!(run.cutoff.is_some());
    }

    #[tokio::test]
    async fn a_followed_container_tells_the_position_of_each_line() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let mut body = frame(STDOUT, &format!("{SHARED_TIME} a\n"));
        body.extend(frame(STDOUT, &format!("{SHARED_TIME} b\n")));
        let server = serve_logs(
            listener,
            "application/vnd.docker.multiplexed-stream",
            body,
        );

        let lines: Vec<LogLine> = engine_at(&path)
            .follow_logs("c178", 500)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(texts(&lines), ["a", "b"]);
        assert_eq!(
            lines[1].position(),
            Some(LogPosition::new(SHARED_NANOS, 2))
        );
        let line = server.join().expect("server thread");
        assert!(line.contains("timestamps=true"), "{line}");
    }

    /// A chunk of the answer holds several lines of one time and is cut by
    /// `since` only to the whole second: the lines before the position come
    /// back and are dropped, in one request and without an inspect.
    #[tokio::test]
    async fn going_on_asks_since_that_second_and_gives_only_the_rest() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let mut body = frame(STDOUT, &format!("{SHARED_TIME} a\n"));
        body.extend(frame(STDOUT, &format!("{SHARED_TIME} b\n")));
        body.extend(frame(STDERR, &format!("{SHARED_TIME} c\n")));
        body.extend(frame(STDOUT, "2026-11-04T00:05:09Z d\n"));
        let server = serve_logs(
            listener,
            "application/vnd.docker.multiplexed-stream",
            body,
        );

        let lines: Vec<LogLine> = engine_at(&path)
            .follow_logs_after("c178", LogPosition::new(SHARED_NANOS, 2), 5000)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(texts(&lines), ["c", "d"]);
        assert_eq!(
            lines[0].position(),
            Some(LogPosition::new(SHARED_NANOS, 3))
        );
        let line = server.join().expect("server thread");
        assert!(line.contains("/containers/c178/logs?"), "{line}");
        for query in [
            "follow=true",
            "stdout=true",
            "stderr=true",
            "timestamps=true",
            "tail=5000",
            "since=1793750708",
        ] {
            assert!(line.contains(query), "{query} missing in {line}");
        }
    }

    #[tokio::test]
    async fn the_run_of_a_container_comes_from_inspect() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = std::thread::spawn(move || {
            let mut stream =
                listener.incoming().flatten().next().expect("a client");
            let line = read_request_line(&mut stream).expect("a request");
            write_response(
                &mut stream,
                "200 OK",
                r#"{"Id":"c178","State":{"Running":true,"StartedAt":"2026-11-04T00:05:06.5Z","FinishedAt":"0001-01-01T00:00:00Z"}}"#,
            );
            line
        });

        let run = engine_at(&path)
            .container_run("c178")
            .await
            .expect("the run");

        assert!(run.is_running());
        assert_eq!(run.started_at, unix_nanos("2026-11-04T00:05:06.5Z"));
        let line = server.join().expect("server thread");
        assert!(line.contains("/containers/c178/json"), "{line}");
    }

    #[tokio::test]
    async fn the_latest_run_gets_only_its_own_lines_without_their_times() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let mut body = frame(STDOUT, "2026-11-03T17:46:21.102Z tick 5\n");
        body.extend(frame(STDOUT, "2026-11-03T17:46:21.856Z bye\n"));
        body.extend(frame(STDERR, "2026-11-03T17:46:22.049Z hello\n"));
        body.extend(frame(STDOUT, "2026-11-03T17:46:22.050Z tick 0\n"));
        let server = serve_run(
            listener,
            r#"{"Id":"c178","State":{"Status":"running","StartedAt":"2026-11-03T17:46:21.956057283Z","FinishedAt":"2026-11-03T17:46:21.857758301Z"}}"#,
            body,
        );

        let engine = engine_at(&path);
        let run = engine.container_run("c178").await.expect("the run");
        let lines: Vec<LogLine> = engine
            .follow_latest_run_logs("c178", &run, 500)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(texts(&lines), ["hello", "tick 0"]);
        let requests = server.join().expect("server thread");
        assert_eq!(requests.len(), 2, "{requests:?}");
        assert!(
            requests[0].contains("/containers/c178/json"),
            "{requests:?}"
        );
        let logs = &requests[1];
        assert!(logs.contains("/containers/c178/logs?"), "{logs}");
        for query in [
            "follow=true",
            "stdout=true",
            "stderr=true",
            "tail=500",
            "timestamps=true",
            "since=1793727981",
        ] {
            assert!(logs.contains(query), "{query} missing in {logs}");
        }
    }

    #[tokio::test]
    async fn the_first_run_of_a_container_gets_every_line() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let body = frame(STDOUT, "2026-11-03T17:46:19.718Z hello\n");
        let server = serve_run(
            listener,
            r#"{"Id":"c178","State":{"StartedAt":"2026-11-03T17:46:21Z","FinishedAt":"0001-01-01T00:00:00Z"}}"#,
            body,
        );

        let engine = engine_at(&path);
        let run = engine.container_run("c178").await.expect("the run");
        let lines: Vec<LogLine> = engine
            .follow_latest_run_logs("c178", &run, 500)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(texts(&lines), ["hello"]);
        let requests = server.join().expect("server thread");
        assert!(!requests[1].contains("since=1"), "{requests:?}");
    }

    #[tokio::test]
    async fn a_gone_container_has_no_run_to_read() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = std::thread::spawn(move || {
            if let Some(mut stream) = listener.incoming().flatten().next()
                && read_request_line(&mut stream).is_some()
            {
                write_response(
                    &mut stream,
                    "404 Not Found",
                    r#"{"message":"No such container: gone"}"#,
                );
            }
        });

        let result = engine_at(&path).container_run("gone").await;

        server.join().expect("server thread");
        let Err(error) = result else {
            panic!("expected an error, got {result:?}");
        };
        assert!(error.to_string().contains("No such container"), "{error}");
    }

    #[tokio::test]
    async fn stdout_and_stderr_lines_come_in_the_engine_order() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let mut body = frame(STDOUT, "starting\nlistening on :80\n");
        body.extend(frame(STDERR, "warning: no config\n"));
        body.extend(frame(STDOUT, "ready\r\n"));
        let server = serve_logs(
            listener,
            "application/vnd.docker.multiplexed-stream",
            body,
        );

        let lines: Vec<LogLine> = engine_at(&path)
            .follow_logs("1f3a9c2e", 500)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(
            texts(&lines),
            [
                "starting",
                "listening on :80",
                "warning: no config",
                "ready"
            ]
        );
        let line = server.join().expect("server thread");
        assert!(line.contains("/containers/1f3a9c2e/logs?"), "{line}");
        for query in [
            "follow=true",
            "stdout=true",
            "stderr=true",
            "tail=500",
            "timestamps=true",
        ] {
            assert!(line.contains(query), "{query} missing in {line}");
        }
    }

    #[tokio::test]
    async fn a_terminal_container_sends_plain_lines() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let body = b"$ ls\r\nbin  etc\r\nlast line without a break".to_vec();
        let server =
            serve_logs(listener, "application/vnd.docker.raw-stream", body);

        let lines: Vec<LogLine> = engine_at(&path)
            .follow_logs("b93d6f0a", 10)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(
            texts(&lines),
            ["$ ls", "bin  etc", "last line without a break"]
        );
        let line = server.join().expect("server thread");
        assert!(line.contains("tail=10"), "{line}");
    }

    /// bollard's own logs call waits for a line break in plain text without
    /// any limit, so the request is made another way. To the engine it must
    /// still be the plain logs request.
    #[tokio::test]
    async fn the_engine_sees_a_plain_logs_request() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_logs(
            listener,
            "application/vnd.docker.raw-stream",
            Vec::new(),
        );

        let lines: Vec<LogLine> = engine_at(&path)
            .follow_logs_after("c178", LogPosition::new(SHARED_NANOS, 2), 5000)
            .try_collect()
            .await
            .expect("log lines");

        assert!(lines.is_empty());
        let line = server.join().expect("server thread");
        assert_eq!(
            line,
            "GET /containers/c178/logs?follow=true&stdout=true&stderr=true&timestamps=true&tail=5000&since=1793750708 HTTP/1.1"
        );
    }

    #[tokio::test]
    async fn a_terminal_stream_without_a_line_break_comes_in_bounded_lines() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let text = "語".repeat(100_000);
        let body = format!("{SHARED_TIME} {text}").into_bytes();
        let server =
            serve_logs(listener, "application/vnd.docker.raw-stream", body);

        let lines: Vec<LogLine> = engine_at(&path)
            .follow_logs("b93d6f0a", 10)
            .try_collect()
            .await
            .expect("log lines");

        server.join().expect("server thread");
        assert!(lines.len() > 4);
        assert!(
            lines
                .iter()
                .all(|line| line.text().len() <= MAX_LOG_LINE_BYTES)
        );
        assert_eq!(joined(&lines), text);
    }

    /// The first lines come while the engine still holds the stream open and
    /// no line break has come: nothing here waits for one.
    #[tokio::test]
    async fn a_terminal_stream_gives_lines_before_any_line_break_comes() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let (release, released) = std::sync::mpsc::channel::<()>();
        let server = std::thread::spawn(move || {
            let mut stream =
                listener.incoming().flatten().next().expect("a client");
            read_request_line(&mut stream).expect("a request");
            let body = format!("{SHARED_TIME} {}", "x".repeat(200_000));
            let head = "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.docker.raw-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
            let size = body.len();
            stream.write_all(head.as_bytes()).expect("the head");
            stream
                .write_all(format!("{size:x}\r\n{body}\r\n").as_bytes())
                .expect("a chunk");
            let _ = released.recv();
            let _ = stream.write_all(b"0\r\n\r\n");
        });

        let mut lines = engine_at(&path).follow_logs("b93d6f0a", 10);
        let wait = std::time::Duration::from_secs(10);
        let first = tokio::time::timeout(wait, lines.next()).await;

        release.send(()).expect("the server waits");
        // The rest is read before the join: where the socket buffer is smaller
        // than the body (8 KB on macOS), the server's write waits for it.
        let _rest: Vec<_> = lines.collect().await;
        server.join().expect("server thread");

        let line = first
            .expect("a line before the stream ended")
            .expect("a line")
            .expect("no error");
        assert!(line.text().starts_with("xxx"));
        assert!(line.text().len() <= MAX_LOG_LINE_BYTES);
    }

    /// The whole way of a broken connection: the first stream stops after a
    /// piece of a long line, the next one goes on from that piece's position.
    #[tokio::test]
    async fn a_new_stream_goes_on_after_a_piece_of_a_long_line() {
        let dir = TempDir::new();
        let text = "x".repeat(3 * MAX_LOG_LINE_BYTES);
        let body = long_answer(&text);
        let kind = "application/vnd.docker.multiplexed-stream";

        let first_path = dir.path().join("first.sock");
        let listener = UnixListener::bind(&first_path).expect("bind");
        let server = serve_logs(listener, kind, body.clone());
        let lines: Vec<LogLine> = engine_at(&first_path)
            .follow_logs("c178", 500)
            .try_collect()
            .await
            .expect("log lines");
        server.join().expect("server thread");
        let position = lines[1].position().expect("a position");

        let next_path = dir.path().join("next.sock");
        let listener = UnixListener::bind(&next_path).expect("bind");
        let server = serve_logs(listener, kind, body);
        let rest: Vec<LogLine> = engine_at(&next_path)
            .follow_logs_after("c178", position, 5000)
            .try_collect()
            .await
            .expect("log lines");
        server.join().expect("server thread");

        assert_eq!(rest, lines[2..]);
    }

    #[tokio::test]
    async fn an_engine_error_ends_the_stream_with_its_message() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = std::thread::spawn(move || {
            if let Some(mut stream) = listener.incoming().flatten().next()
                && read_request_line(&mut stream).is_some()
            {
                write_response(
                    &mut stream,
                    "404 Not Found",
                    r#"{"message":"No such container: gone"}"#,
                );
            }
        });

        let result: Result<Vec<LogLine>, EngineError> = engine_at(&path)
            .follow_logs("gone", 500)
            .try_collect()
            .await;

        server.join().expect("server thread");
        let Err(error) = result else {
            panic!("expected an error, got {result:?}");
        };
        assert!(error.to_string().contains("No such container"), "{error}");
    }
}
