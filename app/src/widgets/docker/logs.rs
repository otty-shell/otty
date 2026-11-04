//! The logs subscription of the inspector's Logs tab: it follows one
//! container and sends its new lines to the widget. It exists only while
//! the widget follows logs (see `DockerState::wanted_log_target`).

use iced::Subscription;
use iced::futures::stream::BoxStream;
use iced::futures::{FutureExt, SinkExt, StreamExt};
use otty_docker::{Engine, EngineError, LogLine, LogPosition};

use super::errors::DockerError;
use super::event::{DockerEvent, DockerIntent};
use super::inspector::{LOG_TAIL, LogRead, LogTarget, MAX_LOG_LINES};

/// Messages that may wait for the app before the stream waits too.
const CHANNEL_SIZE: usize = 100;
/// The most lines sent in one message; the first answer brings up to
/// [`LOG_TAIL`] lines at once.
const BATCH_LINES: usize = LOG_TAIL;
/// A message is full once its lines hold this much text: up to
/// [`CHANNEL_SIZE`] messages can wait here, and as many again in iced.
const BATCH_BYTES: usize = 128 * 1024;

/// The lines of one message and what ended them.
#[derive(Debug, Default)]
struct LineBatch {
    lines: Vec<String>,
    position: Option<LogPosition>,
    error: Option<EngineError>,
}

/// Follow the logs of `target`; nothing when no logs are followed. The
/// subscription is keyed by the target, so another container or engine, or
/// the container coming back, stops this stream and starts a new one.
pub(super) fn subscription(
    target: Option<&LogTarget>,
) -> Subscription<DockerEvent> {
    match target {
        Some(target) => Subscription::run_with(target.clone(), follow),
        None => Subscription::none(),
    }
}

/// Connect, read which run the lines belong to and send it, then ask for the
/// lines of `target` and every new one, and send them in batches. The stream
/// ends when the container stops or the engine fails; a failure is sent once.
/// It is not started again by itself: the engine's start event and a reconnect
/// do that (see `InspectorState`), so a stopped container costs nothing.
///
/// A stream that goes on after a position does not read the run: the lines on
/// screen belong to the run read before them, and that run is what the next
/// reconnect must compare with.
fn follow(target: &LogTarget) -> BoxStream<'static, DockerEvent> {
    let target = target.clone();

    Box::pin(iced::stream::channel(
        CHANNEL_SIZE,
        async move |mut output| {
            let engine = match Engine::connect(target.engine()) {
                Ok(engine) => engine,
                Err(err) => {
                    let _ = output.send(failed(&target, err)).await;
                    return;
                },
            };
            let id = target.container_id();

            let mut lines = if let LogRead::After(position) = target.read() {
                engine.follow_logs_after(id, position, MAX_LOG_LINES)
            } else {
                let run = match engine.container_run(id).await {
                    Ok(run) => run,
                    Err(err) => {
                        let _ = output.send(failed(&target, err)).await;
                        return;
                    },
                };
                let event = intent(DockerIntent::LogRunRead {
                    target: target.clone(),
                    run,
                });
                if output.send(event).await.is_err() {
                    return;
                }

                if target.read() == LogRead::LatestRun {
                    engine.follow_latest_run_logs(id, &run, LOG_TAIL)
                } else {
                    engine.follow_logs(id, LOG_TAIL)
                }
            };

            while let Some(first) = lines.next().await {
                let batch = ready_lines(first, &mut lines);

                if !batch.lines.is_empty() {
                    let event = intent(DockerIntent::LogLines {
                        target: target.clone(),
                        lines: batch.lines,
                        position: batch.position,
                    });
                    if output.send(event).await.is_err() {
                        return;
                    }
                }
                if let Some(err) = batch.error {
                    let _ = output.send(failed(&target, err)).await;
                    return;
                }
            }
        },
    ))
}

/// `first` and the lines that are already waiting, up to [`BATCH_LINES`] and
/// [`BATCH_BYTES`], where they end and the error that came instead of a line.
fn ready_lines(
    first: Result<LogLine, EngineError>,
    lines: &mut BoxStream<'static, Result<LogLine, EngineError>>,
) -> LineBatch {
    let mut batch = LineBatch::default();
    let mut bytes = 0;
    let mut next = Some(first);

    while let Some(item) = next {
        match item {
            Ok(line) => {
                batch.position = line.position();
                bytes += line.text().len();
                batch.lines.push(line.into_text());
            },
            Err(err) => {
                batch.error = Some(err);
                return batch;
            },
        }
        if batch.lines.len() == BATCH_LINES || bytes >= BATCH_BYTES {
            break;
        }
        next = lines.next().now_or_never().flatten();
    }

    batch
}

fn failed(target: &LogTarget, err: EngineError) -> DockerEvent {
    intent(DockerIntent::LogsFailed {
        target: target.clone(),
        error: DockerError::from(err),
    })
}

fn intent(intent: DockerIntent) -> DockerEvent {
    DockerEvent::Intent(intent)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread::JoinHandle;

    use iced::Executor;
    use iced::futures::executor::block_on;
    use iced::futures::stream;
    use otty_docker::{MAX_LOG_LINE_BYTES, fixtures};

    use super::*;

    const RUN: &str = r#"{"Id":"c178","State":{"Running":true,"StartedAt":"2026-11-03T17:46:21.956057283Z","FinishedAt":"2026-11-03T17:46:21.857758301Z"}}"#;
    /// The last line of the run before and the first of the latest one.
    const OLD_LINE: &str = "2026-11-03T17:46:21.856Z bye";
    const NEW_LINE: &str = "2026-11-03T17:46:22.049Z hello";

    /// A socket file of its own, removed at the end.
    struct TestSocket(PathBuf);

    impl TestSocket {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let number = NEXT.fetch_add(1, Ordering::Relaxed);
            let process = std::process::id();

            Self(
                std::env::temp_dir()
                    .join(format!("otty-logs-test-{process}-{number}.sock")),
            )
        }
    }

    impl Drop for TestSocket {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    /// The request line of what the client sent, once all of it is here.
    fn read_request(stream: &mut UnixStream) -> String {
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            let count = stream.read(&mut byte).expect("the request");
            assert_eq!(count, 1, "the client went away");
            request.push(byte[0]);
        }

        let text = String::from_utf8_lossy(&request);
        text.lines().next().unwrap_or_default().to_string()
    }

    fn answer(stream: &mut UnixStream, status: &str, kind: &str, body: &[u8]) {
        let length = body.len();
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(head.as_bytes()).expect("the head");
        stream.write_all(body).expect("the body");
    }

    /// Answer inspect with `inspect` until the logs were asked for, then
    /// answer with `lines` and stop; a 404 to inspect stops it too. Returns
    /// the request lines.
    fn serve(
        socket: &TestSocket,
        inspect: Option<&'static str>,
        lines: &'static [&'static str],
    ) -> JoinHandle<Vec<String>> {
        let listener = UnixListener::bind(&socket.0).expect("bind the socket");

        std::thread::spawn(move || {
            let mut requests = Vec::new();
            for mut stream in listener.incoming().flatten() {
                let request = read_request(&mut stream);
                let is_logs = request.contains("/logs");
                requests.push(request);

                if is_logs {
                    let mut body = Vec::new();
                    for line in lines {
                        let line = format!("{line}\n");
                        let length = u32::try_from(line.len()).expect("short");
                        body.extend_from_slice(&[1, 0, 0, 0]);
                        body.extend_from_slice(&length.to_be_bytes());
                        body.extend_from_slice(line.as_bytes());
                    }
                    let kind = "application/vnd.docker.multiplexed-stream";
                    answer(&mut stream, "200 OK", kind, &body);
                    break;
                }
                let Some(inspect) = inspect else {
                    let message = br#"{"message":"No such container"}"#;
                    answer(
                        &mut stream,
                        "404 Not Found",
                        "application/json",
                        message,
                    );
                    break;
                };
                answer(
                    &mut stream,
                    "200 OK",
                    "application/json",
                    inspect.as_bytes(),
                );
            }
            requests
        })
    }

    fn target_at(socket: &TestSocket, read: LogRead) -> LogTarget {
        LogTarget::new(fixtures::engine_at(&socket.0), "c178").reading(read)
    }

    /// What the stream of `target` sends until it ends.
    fn followed_intents(target: &LogTarget) -> Vec<DockerIntent> {
        let runtime =
            <iced::executor::Default as Executor>::new().expect("a runtime");
        let _context = runtime.enter();

        let events: Vec<DockerEvent> = block_on(follow(target).collect());

        events
            .into_iter()
            .map(|event| match event {
                DockerEvent::Intent(intent) => intent,
                DockerEvent::Effect(effect) => {
                    panic!("no effect expected: {effect:?}")
                },
            })
            .collect()
    }

    /// The lines of all `LogLines` in `intents`.
    fn lines_of(intents: &[DockerIntent]) -> Vec<&str> {
        intents
            .iter()
            .filter_map(|intent| match intent {
                DockerIntent::LogLines { lines, .. } => Some(lines),
                _ => None,
            })
            .flatten()
            .map(String::as_str)
            .collect()
    }

    /// The run comes ahead of the lines, because the next reconnect compares
    /// with it as soon as a line is on screen.
    #[test]
    fn a_new_stream_sends_its_run_and_then_the_last_lines() {
        let socket = TestSocket::new();
        let server = serve(&socket, Some(RUN), &[OLD_LINE, NEW_LINE]);
        let target = target_at(&socket, LogRead::LastLines);

        let intents = followed_intents(&target);

        let requests = server.join().expect("the server");
        assert_eq!(requests.len(), 2, "{requests:?}");
        assert!(
            requests[0].contains("/containers/c178/json"),
            "{requests:?}"
        );
        assert!(requests[1].contains("tail=500"), "{requests:?}");
        assert!(matches!(
            &intents[0],
            DockerIntent::LogRunRead { target: read, run }
                if read == &target && run.is_running()
        ));
        assert_eq!(lines_of(&intents), ["bye", "hello"]);
    }

    #[test]
    fn a_stream_of_the_latest_run_drops_the_older_lines() {
        let socket = TestSocket::new();
        let server = serve(&socket, Some(RUN), &[OLD_LINE, NEW_LINE]);

        let intents = followed_intents(&target_at(&socket, LogRead::LatestRun));

        let requests = server.join().expect("the server");
        assert_eq!(requests.len(), 2, "{requests:?}");
        assert!(matches!(&intents[0], DockerIntent::LogRunRead { .. }));
        assert_eq!(lines_of(&intents), ["hello"]);
    }

    /// The lines on screen belong to the run read before them. Reading it
    /// again here would hide a restart from the next reconnect.
    #[test]
    fn a_stream_going_on_does_not_read_the_run() {
        let socket = TestSocket::new();
        let server = serve(&socket, None, &[OLD_LINE, NEW_LINE]);
        let position = fixtures::log_position(1_793_727_981, 1);
        let target = target_at(&socket, LogRead::After(position));

        let intents = followed_intents(&target);

        let requests = server.join().expect("the server");
        assert_eq!(requests.len(), 1, "{requests:?}");
        assert!(requests[0].contains("/logs?"), "{requests:?}");
        assert!(requests[0].contains("since=1793727981"), "{requests:?}");
        assert!(requests[0].contains("tail=5000"), "{requests:?}");
        assert_eq!(intents.len(), 1, "{intents:?}");
        assert_eq!(lines_of(&intents), ["bye", "hello"]);
    }

    #[test]
    fn the_lines_of_a_message_say_where_they_end() {
        let socket = TestSocket::new();
        let server = serve(&socket, Some(RUN), &[OLD_LINE, NEW_LINE]);

        let intents = followed_intents(&target_at(&socket, LogRead::LastLines));

        server.join().expect("the server");
        let last_position =
            intents.iter().rev().find_map(|intent| match intent {
                DockerIntent::LogLines { position, .. } => *position,
                _ => None,
            });
        assert!(last_position.is_some(), "{intents:?}");
    }

    #[test]
    fn a_container_that_cannot_be_read_is_one_failure() {
        let socket = TestSocket::new();
        let server = serve(&socket, None, &[]);

        let intents = followed_intents(&target_at(&socket, LogRead::LastLines));

        server.join().expect("the server");
        assert_eq!(intents.len(), 1, "{intents:?}");
        assert!(matches!(
            &intents[0],
            DockerIntent::LogsFailed { error, .. }
                if error.to_string().contains("No such container")
        ));
    }

    /// `count` lines of `bytes` bytes each that are already waiting.
    fn waiting_lines(
        count: usize,
        bytes: usize,
    ) -> BoxStream<'static, Result<LogLine, EngineError>> {
        let lines: Vec<Result<LogLine, EngineError>> = (0..count)
            .map(|_| Ok(fixtures::log_line(&"x".repeat(bytes), None)))
            .collect();

        stream::iter(lines).boxed()
    }

    #[test]
    fn a_batch_stops_at_its_line_limit() {
        let mut rest = waiting_lines(BATCH_LINES + 10, 1);
        let first = Ok(fixtures::log_line("first", None));

        let batch = ready_lines(first, &mut rest);

        assert_eq!(batch.lines.len(), BATCH_LINES);
    }

    /// Up to 100 messages can wait for the app, so one message must not hold
    /// [`BATCH_LINES`] lines of the longest kind.
    #[test]
    fn a_batch_of_long_lines_stops_at_its_byte_limit() {
        let mut rest = waiting_lines(BATCH_LINES, MAX_LOG_LINE_BYTES);
        let first = Ok(fixtures::log_line("first", None));

        let batch = ready_lines(first, &mut rest);

        let bytes: usize = batch.lines.iter().map(String::len).sum();
        assert!(bytes >= BATCH_BYTES, "{bytes}");
        assert!(bytes < BATCH_BYTES + MAX_LOG_LINE_BYTES, "{bytes}");
    }

    #[test]
    fn what_a_batch_left_comes_in_the_next_one_in_order() {
        let lines: Vec<Result<LogLine, EngineError>> = (0..6)
            .map(|n| {
                let text = format!("{n}{}", "x".repeat(MAX_LOG_LINE_BYTES - 1));
                Ok(fixtures::log_line(&text, None))
            })
            .collect();
        let mut rest: BoxStream<'static, Result<LogLine, EngineError>> =
            stream::iter(lines).boxed();

        let mut firsts = Vec::new();
        while let Some(first) = block_on(rest.next()) {
            let batch = ready_lines(first, &mut rest);
            firsts.extend(batch.lines.iter().map(|line| line[..1].to_string()));
        }

        assert_eq!(firsts, ["0", "1", "2", "3", "4", "5"]);
    }

    /// The whole way to the app: one long run comes as lines that fit the limit
    /// and lose nothing, and each message says where it ends.
    #[test]
    fn a_long_run_reaches_the_app_in_lines_that_fit_the_limit() {
        let socket = TestSocket::new();
        let text = "語".repeat(150_000);
        let line: &'static str =
            format!("2026-11-03T17:46:22.049Z {text}").leak();
        let lines: &'static [&'static str] = vec![line].leak();
        let server = serve(&socket, Some(RUN), lines);

        let intents = followed_intents(&target_at(&socket, LogRead::LastLines));

        server.join().expect("the server");
        let shown = lines_of(&intents);
        assert!(shown.len() > 1);
        assert!(shown.iter().all(|line| line.len() <= MAX_LOG_LINE_BYTES));
        assert_eq!(shown.concat(), text);
        let is_position_told = intents.iter().all(|intent| match intent {
            DockerIntent::LogLines { position, .. } => position.is_some(),
            _ => true,
        });
        assert!(is_position_told, "{intents:?}");
    }

    #[test]
    fn an_error_ends_a_batch() {
        let mut rest: BoxStream<'static, Result<LogLine, EngineError>> =
            stream::iter(Vec::new()).boxed();

        let error = EngineError::PathNotUtf8(PathBuf::from("gone"));
        let batch = ready_lines(Err(error), &mut rest);

        assert!(batch.lines.is_empty());
        assert_eq!(batch.position, None);
        assert!(batch.error.is_some());
    }
}
