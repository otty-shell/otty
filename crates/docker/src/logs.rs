//! Following a container's logs for the inspector's Logs tab (spec
//! Decision 12).
//!
//! bollard splits the engine's answer into stdout and stderr frames (or
//! plain lines for a container with a terminal); they are kept in the order
//! they came and cut into lines here. bollard's 30 s request limit covers
//! only the wait for the answer to start, not the stream after it, so one
//! client serves both the lists and a long follow.
//!
//! Both engines keep the lines of earlier runs of a container. On a restart
//! Docker ends the stream; Podman ends it or, with its journald log driver,
//! keeps it going (seen on Docker 29.9.1 and Podman 5.8.8). The app therefore
//! goes by the engine's start event, not by the end of a stream.

use bollard::container::LogOutput;
use bollard::query_parameters::{InspectContainerOptions, LogsOptionsBuilder};
use futures_util::stream::{self, BoxStream, StreamExt, TryStreamExt};

use crate::connection::{Engine, EngineError};
use crate::timestamp::{NANOS_PER_SECOND, unix_nanos};

impl Engine {
    /// The last `tail` lines of the container with `id` (stdout and stderr),
    /// then every new line until the container stops or the stream is
    /// dropped. An engine error ends the stream with that error.
    pub fn follow_logs(
        &self,
        id: &str,
        tail: usize,
    ) -> BoxStream<'static, Result<String, EngineError>> {
        let options = LogsOptionsBuilder::new()
            .follow(true)
            .stdout(true)
            .stderr(true)
            .tail(&tail.to_string())
            .build();

        self.client()
            .logs(id, Some(options))
            .map_err(EngineError::from)
            .map_ok(|output| stream::iter(output_lines(&output)).map(Ok))
            .try_flatten()
            .boxed()
    }

    /// Like [`Self::follow_logs`], for a container that just came back: only
    /// the lines of its latest run. The engine keeps the lines of earlier
    /// runs and bollard can ask it for lines since a whole second only, which
    /// still holds the old run's last lines after a fast restart. So the
    /// lines are read with their times and the older ones are dropped here.
    pub fn follow_latest_run_logs(
        &self,
        id: &str,
        tail: usize,
    ) -> BoxStream<'static, Result<String, EngineError>> {
        let engine = self.clone();
        let id = id.to_string();
        let tail = tail.to_string();

        stream::once(async move {
            let answer = engine
                .client()
                .inspect_container(&id, None::<InspectContainerOptions>)
                .await?;
            let state = answer.state.unwrap_or_default();
            let cutoff = run_cutoff(
                state.started_at.as_deref(),
                state.finished_at.as_deref(),
            );

            let mut options = LogsOptionsBuilder::new()
                .follow(true)
                .stdout(true)
                .stderr(true)
                .timestamps(true)
                .tail(&tail);
            if let Some(since) = cutoff.and_then(whole_second_before) {
                options = options.since(since);
            }

            let lines = engine
                .client()
                .logs(&id, Some(options.build()))
                .map_err(EngineError::from)
                .map_ok(move |output| {
                    let lines = output_lines(&output)
                        .iter()
                        .filter_map(|line| line_of_run(line, cutoff))
                        .collect::<Vec<_>>();
                    stream::iter(lines).map(Ok)
                })
                .try_flatten();

            Ok::<_, EngineError>(lines)
        })
        .try_flatten()
        .boxed()
    }
}

/// The lines of one piece of output, without their line breaks. Input the
/// container read (stdin) is not output and gives no lines.
fn output_lines(output: &LogOutput) -> Vec<String> {
    let message = match output {
        LogOutput::StdOut { message }
        | LogOutput::StdErr { message }
        | LogOutput::Console { message } => message,
        _ => return Vec::new(),
    };

    let text = String::from_utf8_lossy(message);
    if text.is_empty() {
        return Vec::new();
    }

    let text = text.strip_suffix('\n').unwrap_or(&text);
    text.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect()
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

/// The text of a `<time> <text>` log line, or `None` when the line is older
/// than `cutoff`. A line that does not start with a time is kept as it is:
/// better shown than lost.
fn line_of_run(line: &str, cutoff: Option<i64>) -> Option<String> {
    let parts = line
        .split_once(' ')
        .and_then(|(time, text)| Some((unix_nanos(time)?, text)));
    let Some((time, text)) = parts else {
        return Some(line.to_string());
    };

    let is_older = cutoff.is_some_and(|cutoff| time < cutoff);
    (!is_older).then(|| text.to_string())
}

/// `nanos` as the whole second at or before it, which is all the engine's
/// `since` takes through bollard; `None` when it does not fit.
fn whole_second_before(nanos: i64) -> Option<i32> {
    i32::try_from(nanos.div_euclid(NANOS_PER_SECOND)).ok()
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::thread::JoinHandle;

    use bollard::models::SystemVersion;
    use futures_util::TryStreamExt;

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
        let length = u32::try_from(text.len()).expect("short text");
        let mut frame = vec![kind, 0, 0, 0];
        frame.extend_from_slice(&length.to_be_bytes());
        frame.extend_from_slice(text.as_bytes());
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

        let lines: Vec<String> = engine_at(&path)
            .follow_latest_run_logs("c178", 500)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(lines, ["hello", "tick 0"]);
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

        let lines: Vec<String> = engine_at(&path)
            .follow_latest_run_logs("c178", 500)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(lines, ["hello"]);
        let requests = server.join().expect("server thread");
        assert!(!requests[1].contains("since=1"), "{requests:?}");
    }

    #[tokio::test]
    async fn a_container_gone_before_the_latest_run_is_read_is_an_error() {
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

        let result: Result<Vec<String>, EngineError> = engine_at(&path)
            .follow_latest_run_logs("gone", 500)
            .try_collect()
            .await;

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

        let lines: Vec<String> = engine_at(&path)
            .follow_logs("1f3a9c2e", 500)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(
            lines,
            [
                "starting",
                "listening on :80",
                "warning: no config",
                "ready"
            ]
        );
        let line = server.join().expect("server thread");
        assert!(line.contains("/containers/1f3a9c2e/logs?"), "{line}");
        for query in ["follow=true", "stdout=true", "stderr=true", "tail=500"] {
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

        let lines: Vec<String> = engine_at(&path)
            .follow_logs("b93d6f0a", 10)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(lines, ["$ ls", "bin  etc", "last line without a break"]);
        let line = server.join().expect("server thread");
        assert!(line.contains("tail=10"), "{line}");
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

        let result: Result<Vec<String>, EngineError> = engine_at(&path)
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
