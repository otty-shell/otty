//! The logs subscription of the inspector's Logs tab: it follows one
//! container and sends its new lines to the widget. It exists only while
//! the widget follows logs (see `DockerState::wanted_log_target`).

use iced::Subscription;
use iced::futures::stream::BoxStream;
use iced::futures::{FutureExt, SinkExt, StreamExt};
use otty_docker::{Engine, EngineError};

use super::errors::DockerError;
use super::event::{DockerEvent, DockerIntent};
use super::inspector::{LOG_TAIL, LogTarget};

/// Messages that may wait for the app before the stream waits too.
const CHANNEL_SIZE: usize = 100;
/// The most lines sent in one message; the first answer brings up to
/// [`LOG_TAIL`] lines at once.
const BATCH_LINES: usize = LOG_TAIL;

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

/// Connect, ask for the last [`LOG_TAIL`] lines and every new one, and send
/// them in batches; after a comeback, of the latest run only. The stream ends
/// when the container stops or the engine fails; a failure is sent once. It
/// is not started again by itself: the engine's start event does that (see
/// `InspectorState::restart_logs`), so a stopped container costs nothing.
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
            let mut lines = if target.reads_latest_run_only() {
                engine.follow_latest_run_logs(id, LOG_TAIL)
            } else {
                engine.follow_logs(id, LOG_TAIL)
            };

            while let Some(first) = lines.next().await {
                let (batch, error) = ready_lines(first, &mut lines);

                if !batch.is_empty() {
                    let event = intent(DockerIntent::LogLines {
                        target: target.clone(),
                        lines: batch,
                    });
                    if output.send(event).await.is_err() {
                        return;
                    }
                }
                if let Some(err) = error {
                    let _ = output.send(failed(&target, err)).await;
                    return;
                }
            }
        },
    ))
}

/// `first` and the lines that are already waiting, up to [`BATCH_LINES`],
/// and the error that came instead of a line.
fn ready_lines(
    first: Result<String, EngineError>,
    lines: &mut BoxStream<'static, Result<String, EngineError>>,
) -> (Vec<String>, Option<EngineError>) {
    let mut batch = Vec::new();
    let mut next = Some(first);

    while let Some(item) = next {
        match item {
            Ok(line) => batch.push(line),
            Err(err) => return (batch, Some(err)),
        }
        if batch.len() == BATCH_LINES {
            break;
        }
        next = lines.next().now_or_never().flatten();
    }

    (batch, None)
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
