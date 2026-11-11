//! Watches one engine's `/events` and says which list to reload, at most
//! once per 500 ms per list; reconnects with a growing delay when the stream
//! breaks and says when the engine stops answering (spec Decision 14).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use bollard::query_parameters::EventsOptions;
use futures_util::future;
use futures_util::stream::{self, BoxStream, Stream, StreamExt, TryStreamExt};

use crate::connection::{Engine, EngineError};
use crate::debounce::ReloadDebounce;
use crate::events::{EngineChange, event_filters};
use crate::watch_event::WatchEvent;

/// More doublings than this pass every sensible longest delay; the cap
/// keeps the shift in range.
const MAX_DOUBLINGS: u32 = 16;

/// The times the watch works with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WatchTiming {
    /// The shortest time between two reloads of one list.
    gap: Duration,
    /// The delay before the first try to reconnect.
    first_retry: Duration,
    /// The longest delay between two tries.
    longest_retry: Duration,
}

impl WatchTiming {
    /// Spec Decision 14: one reload per list per 500 ms; reconnect after
    /// 1 s, doubling up to 30 s.
    const DEFAULT: Self = Self {
        gap: Duration::from_millis(500),
        first_retry: Duration::from_secs(1),
        longest_retry: Duration::from_secs(30),
    };
}

/// The watch of one engine between two events it sends.
struct Watch {
    engine: Engine,
    timing: WatchTiming,
    /// The open events stream; `None` before the first open and after it
    /// broke.
    events: Option<BoxStream<'static, Result<EngineChange, EngineError>>>,
    /// How many times in a row the stream broke or the engine did not
    /// answer; an event that comes through sets it back to zero.
    failures: u32,
    debounce: ReloadDebounce,
    /// Events that are ready and still to be sent.
    due: VecDeque<WatchEvent>,
}

impl Watch {
    fn new(engine: Engine, timing: WatchTiming) -> Self {
        Self {
            engine,
            timing,
            events: None,
            failures: 0,
            debounce: ReloadDebounce::new(timing.gap),
            due: VecDeque::new(),
        }
    }

    /// Wait for the next thing to tell the app.
    async fn next_event(&mut self) -> WatchEvent {
        loop {
            if let Some(event) = self.due.pop_front() {
                return event;
            }

            let Some(events) = self.events.as_mut() else {
                if let Some(event) = self.open().await {
                    return event;
                }
                continue;
            };

            let item = match self.debounce.next_due() {
                Some(due) => {
                    let deadline = tokio::time::Instant::from_std(due);
                    match tokio::time::timeout_at(deadline, events.next()).await
                    {
                        Ok(item) => item,
                        Err(_) => {
                            let now = Instant::now();
                            let due = self.debounce.take_due(now);
                            self.due.extend(
                                due.into_iter().map(WatchEvent::Changed),
                            );
                            continue;
                        },
                    }
                },
                None => events.next().await,
            };

            match item {
                Some(Ok(change)) => {
                    self.failures = 0;
                    if let Some(id) = change.started_container() {
                        self.due.push_back(WatchEvent::ContainerStarted(
                            id.to_string(),
                        ));
                    }
                    if self.debounce.change(change.list(), Instant::now()) {
                        self.due.push_back(WatchEvent::Changed(change.list()));
                    }
                },
                Some(Err(err)) => {
                    log::debug!("docker events stream broke: {err}");
                    self.mark_broken();
                },
                None => {
                    log::debug!("docker events stream ended");
                    self.mark_broken();
                },
            }
        }
    }

    /// Open the events stream: at once the first time; after a break, wait
    /// a growing delay and check first that the engine still answers.
    async fn open(&mut self) -> Option<WatchEvent> {
        if self.failures == 0 {
            self.events = Some(self.engine.changes());
            return None;
        }

        tokio::time::sleep(reconnect_delay(self.failures, &self.timing)).await;

        if let Err(err) = self.engine.ping().await {
            log::debug!("docker engine does not answer: {err}");
            self.failures = self.failures.saturating_add(1);
            return Some(WatchEvent::Unreachable);
        }

        self.events = Some(self.engine.changes());
        Some(WatchEvent::Reconnected)
    }

    /// The stream broke or ended: open it again on the next turn.
    fn mark_broken(&mut self) {
        self.events = None;
        self.failures = self.failures.saturating_add(1);
    }
}

impl Engine {
    /// Watch this engine's events until the stream is dropped. The stream
    /// never ends on its own: when the engine's events stream breaks, it
    /// reconnects with a growing delay (1 s, 2 s, … up to 30 s) and says
    /// [`WatchEvent::Reconnected`], or [`WatchEvent::Unreachable`] while the
    /// engine does not answer.
    ///
    /// ```no_run
    /// # async fn run(engine: otty_containers::Engine) {
    /// use futures_util::StreamExt;
    /// use otty_containers::WatchEvent;
    ///
    /// let mut watch = Box::pin(engine.watch());
    /// while let Some(event) = watch.next().await {
    ///     if let WatchEvent::Changed(list) = event {
    ///         println!("reload {list:?}");
    ///     }
    /// }
    /// # }
    /// ```
    pub fn watch(&self) -> impl Stream<Item = WatchEvent> + Send + 'static {
        watch_stream(self.clone(), WatchTiming::DEFAULT)
    }

    /// What the engine's events change, as they come. Ends when the engine
    /// closes the stream; an error item means it broke.
    pub(crate) fn changes(
        &self,
    ) -> BoxStream<'static, Result<EngineChange, EngineError>> {
        let options = EventsOptions {
            filters: Some(event_filters()),
            ..EventsOptions::default()
        };

        self.client()
            .events(Some(options))
            .map_err(EngineError::from)
            .try_filter_map(|message| {
                future::ready(Ok(EngineChange::from_event(&message)))
            })
            .boxed()
    }

    /// Ask whether the engine answers (`/_ping`).
    pub(crate) async fn ping(&self) -> Result<(), EngineError> {
        self.client().ping().await?;

        Ok(())
    }
}

/// The delay before the `failures`-th try to reconnect in a row: the first
/// delay, doubled for each earlier failure, at most the longest delay.
fn reconnect_delay(failures: u32, timing: &WatchTiming) -> Duration {
    let doublings = failures.saturating_sub(1).min(MAX_DOUBLINGS);
    let delay = timing.first_retry.saturating_mul(1 << doublings);

    delay.min(timing.longest_retry)
}

/// The watch as a stream that never ends on its own.
fn watch_stream(
    engine: Engine,
    timing: WatchTiming,
) -> BoxStream<'static, WatchEvent> {
    stream::unfold(Watch::new(engine, timing), |mut watch| async move {
        let event = watch.next_event().await;
        Some((event, watch))
    })
    .boxed()
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::sync::mpsc;

    use bollard::models::SystemVersion;

    use super::*;
    use crate::address::EngineAddress;
    use crate::engine::EngineInfo;
    use crate::test_support::{
        TempDir, read_request_line, write_response, write_stream_head,
        write_stream_line,
    };
    use crate::watch_event::ListKind;

    const CONTAINER_START: &str = r#"{"Type":"container","Action":"start","Actor":{"ID":"c1"},"scope":"local"}"#;
    const CONTAINER_DIE: &str = r#"{"Type":"container","Action":"die","Actor":{"ID":"c1"},"scope":"local"}"#;
    const CONTAINER_EXEC: &str = r#"{"Type":"container","Action":"exec_create: sh","Actor":{"ID":"c1"},"scope":"local"}"#;
    const IMAGE_PULL: &str = r#"{"Type":"image","Action":"pull","Actor":{"ID":"busybox:1.37"},"scope":"local"}"#;

    /// Short times, so the tests do not wait seconds.
    const FAST: WatchTiming = WatchTiming {
        gap: Duration::from_millis(150),
        first_retry: Duration::from_millis(20),
        longest_retry: Duration::from_millis(80),
    };
    /// How long a test waits for the watch at most.
    const TEST_LIMIT: Duration = Duration::from_secs(5);

    fn engine(path: &Path) -> Engine {
        let answer: SystemVersion =
            serde_json::from_str(r#"{"ApiVersion":"1.44"}"#).expect("answer");
        let info = EngineInfo::from_version(
            &answer,
            EngineAddress::Unix(path.to_path_buf()),
        );

        Engine::connect(&info).expect("engine")
    }

    /// An engine whose socket file is still there but nothing listens on it
    /// any more (bollard refuses to make a client for a missing file).
    fn gone_engine(dir: &TempDir) -> Engine {
        let path = dir.path().join("gone.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let engine = engine(&path);

        drop(listener);
        engine
    }

    /// One answer the fake engine gives, in the order the requests come.
    enum Answer {
        /// Stream these event lines, then close the connection.
        EventsThenClose(&'static [&'static str]),
        /// Stream these event lines and keep the connection open until the
        /// test ends.
        EventsThenWait(&'static [&'static str]),
        /// Answer `/_ping`.
        Pong,
    }

    /// Serve `answers` one connection each and send back each request line.
    /// The thread keeps waiting connections open until `hold` is dropped.
    fn serve(
        listener: UnixListener,
        answers: Vec<Answer>,
        hold: mpsc::Receiver<()>,
    ) -> mpsc::Receiver<String> {
        let (lines, received) = mpsc::channel();

        std::thread::spawn(move || {
            let mut open = Vec::new();
            let mut answers = answers.into_iter();
            for mut stream in listener.incoming().flatten() {
                let Some(line) = read_request_line(&mut stream) else {
                    continue;
                };
                let _ = lines.send(line);
                let Some(answer) = answers.next() else {
                    break;
                };
                match answer {
                    Answer::EventsThenClose(events) => {
                        write_stream_head(&mut stream);
                        for event in events {
                            write_stream_line(&mut stream, event);
                        }
                    },
                    Answer::EventsThenWait(events) => {
                        write_stream_head(&mut stream);
                        for event in events {
                            write_stream_line(&mut stream, event);
                        }
                        open.push(stream);
                    },
                    Answer::Pong => {
                        write_response(&mut stream, "200 OK", "OK");
                    },
                }
            }
            let _ = hold.recv();
        });

        received
    }

    /// Take `count` events of the watch, with the time each one came.
    async fn take_timed(
        watch: BoxStream<'static, WatchEvent>,
        count: usize,
    ) -> Vec<(WatchEvent, Instant)> {
        let taken = watch
            .take(count)
            .map(|event| (event, Instant::now()))
            .collect::<Vec<_>>();

        tokio::time::timeout(TEST_LIMIT, taken)
            .await
            .expect("the watch sends in time")
    }

    #[test]
    fn the_retry_delay_doubles_up_to_the_longest() {
        let timing = WatchTiming::DEFAULT;

        let delays: Vec<u64> = (1..=7)
            .map(|failures| reconnect_delay(failures, &timing).as_secs())
            .collect();

        assert_eq!(delays, vec![1, 2, 4, 8, 16, 30, 30]);
        assert_eq!(reconnect_delay(u32::MAX, &timing), Duration::from_secs(30));
        assert_eq!(reconnect_delay(0, &timing), Duration::from_secs(1));
    }

    #[tokio::test]
    async fn changes_come_from_the_events_stream_and_end_with_it() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let (_hold, wait) = mpsc::channel();
        let lines = serve(
            listener,
            vec![Answer::EventsThenClose(&[
                CONTAINER_START,
                CONTAINER_EXEC,
                IMAGE_PULL,
            ])],
            wait,
        );

        let changes: Vec<ListKind> = engine(&path)
            .changes()
            .map(|change| change.expect("a change").list())
            .collect()
            .await;

        assert_eq!(changes, vec![ListKind::Containers, ListKind::Images]);
        let line = lines.recv_timeout(TEST_LIMIT).expect("the request");
        assert!(line.starts_with("GET /events?"), "{line}");
        assert!(line.contains("filters="), "{line}");
        assert!(line.contains("network"), "{line}");
    }

    #[tokio::test]
    async fn a_missing_engine_does_not_answer_the_ping() {
        let dir = TempDir::new();

        let result = gone_engine(&dir).ping().await;

        assert!(matches!(result, Err(EngineError::Request(_))), "{result:?}");
    }

    #[tokio::test]
    async fn a_running_engine_answers_the_ping() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let (_hold, wait) = mpsc::channel();
        let lines = serve(listener, vec![Answer::Pong], wait);

        let result = engine(&path).ping().await;

        assert!(result.is_ok(), "{result:?}");
        let line = lines.recv_timeout(TEST_LIMIT).expect("the request");
        assert!(line.starts_with("GET /_ping"), "{line}");
    }

    #[tokio::test]
    async fn changes_inside_the_gap_give_one_more_reload_after_it() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let (_hold, wait) = mpsc::channel();
        let _lines = serve(
            listener,
            vec![Answer::EventsThenWait(&[
                CONTAINER_START,
                CONTAINER_DIE,
                IMAGE_PULL,
            ])],
            wait,
        );

        let events = take_timed(watch_stream(engine(&path), FAST), 4).await;

        let kinds: Vec<WatchEvent> =
            events.iter().map(|(event, _)| event.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                WatchEvent::ContainerStarted(String::from("c1")),
                WatchEvent::Changed(ListKind::Containers),
                WatchEvent::Changed(ListKind::Images),
                WatchEvent::Changed(ListKind::Containers),
            ]
        );
        let waited = events[3].1.duration_since(events[1].1);
        assert!(waited >= FAST.gap - Duration::from_millis(5), "{waited:?}");
    }

    /// A fast restart sends start events closer together than the reload gap:
    /// the list reload waits, each start still comes at once.
    #[tokio::test]
    async fn every_container_start_comes_through_inside_the_gap() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let (_hold, wait) = mpsc::channel();
        let _lines = serve(
            listener,
            vec![Answer::EventsThenWait(&[
                CONTAINER_START,
                CONTAINER_DIE,
                CONTAINER_START,
            ])],
            wait,
        );

        let events = take_timed(watch_stream(engine(&path), FAST), 4).await;

        let kinds: Vec<WatchEvent> =
            events.iter().map(|(event, _)| event.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                WatchEvent::ContainerStarted(String::from("c1")),
                WatchEvent::Changed(ListKind::Containers),
                WatchEvent::ContainerStarted(String::from("c1")),
                WatchEvent::Changed(ListKind::Containers),
            ]
        );
        let second_start = events[2].1.duration_since(events[0].1);
        assert!(second_start < FAST.gap, "{second_start:?}");
    }

    #[tokio::test]
    async fn a_broken_stream_is_opened_again_and_says_so() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let (_hold, wait) = mpsc::channel();
        let lines = serve(
            listener,
            vec![
                Answer::EventsThenClose(&[CONTAINER_START]),
                Answer::Pong,
                Answer::EventsThenWait(&[IMAGE_PULL]),
            ],
            wait,
        );

        let events = take_timed(watch_stream(engine(&path), FAST), 4).await;

        let kinds: Vec<WatchEvent> =
            events.iter().map(|(event, _)| event.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                WatchEvent::ContainerStarted(String::from("c1")),
                WatchEvent::Changed(ListKind::Containers),
                WatchEvent::Reconnected,
                WatchEvent::Changed(ListKind::Images),
            ]
        );
        let paths: Vec<String> = lines.try_iter().collect();
        assert_eq!(paths.len(), 3, "{paths:?}");
        assert!(paths[1].starts_with("GET /_ping"), "{paths:?}");
        assert!(paths[2].starts_with("GET /events?"), "{paths:?}");
    }

    #[tokio::test]
    async fn a_gone_engine_is_unreachable_and_tried_again_later() {
        let dir = TempDir::new();

        let events = take_timed(watch_stream(gone_engine(&dir), FAST), 2).await;

        assert_eq!(events[0].0, WatchEvent::Unreachable);
        assert_eq!(events[1].0, WatchEvent::Unreachable);
        let waited = events[1].1.duration_since(events[0].1);
        assert!(waited >= Duration::from_millis(35), "{waited:?}");
    }
}
