//! The reads and writes of Agents mode run on a thread of their own, so a
//! slow disk, server or shell never holds up the window.

use iced::Task;
use iced::futures::channel::oneshot;
use otty_agents::{SourceError, SourceFailure, SourceId, Unavailable};
use otty_app_agents::AgentsIntent;

/// Run `work` on a thread of its own and answer with `answer` of what it
/// gave. When the thread can not start, or ends without an answer because
/// `work` panicked, the stop is logged with `task`, which says what the
/// work does ("reading the limits of Coder"), and `stopped` gives what to
/// answer with instead.
pub(crate) fn on_own_thread<T: Send + 'static>(
    task: String,
    work: impl FnOnce() -> T + Send + 'static,
    stopped: impl FnOnce() -> T + Send + 'static,
    answer: impl FnOnce(T) -> AgentsIntent + Send + 'static,
) -> Task<AgentsIntent> {
    Task::perform(wait_for_thread(work), move |result| {
        let given = result.unwrap_or_else(|| {
            log::warn!("{task} stopped before it answered");
            stopped()
        });
        answer(given)
    })
}

/// Why a read that stopped before it answered gave nothing.
pub(crate) fn read_stopped() -> SourceError {
    SourceError::NotAvailable(Unavailable::ReadStopped)
}

/// What a read from `sources` that stopped before it answered gives: each
/// of them failed because the read stopped, so what was read before stays
/// and the user sees why.
pub(crate) fn failed_because_stopped(
    sources: &[SourceId],
) -> Vec<SourceFailure> {
    sources
        .iter()
        .map(|source| SourceFailure::new(*source, read_stopped()))
        .collect()
}

/// Run `work` on a thread of its own and wait for its answer without
/// holding up the runtime. Not on the runtime's blocking pool: closing the
/// runtime waits for that pool, so a read stuck on a slow shell would keep
/// the app from exiting. `None` when the thread can not start or ends
/// without an answer.
pub(crate) async fn wait_for_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = oneshot::channel();

    let thread = std::thread::Builder::new()
        .name(String::from("otty-agents-io"))
        .spawn(move || {
            let _ = sender.send(work());
        });
    if let Err(err) = thread {
        log::warn!("no thread for a read of Agents mode: {err}");
        return None;
    }

    receiver.await.ok()
}

#[cfg(test)]
mod tests {
    use otty_app_agents::FolderQuery;

    use super::*;
    use crate::testing::run;

    fn folder_looked_up(is_folder: bool) -> AgentsIntent {
        AgentsIntent::FolderLookedUp(otty_app_agents::LoadedFolder {
            query: FolderQuery {
                asker: otty_app_agents::FolderAsker::NewSessionDialog,
                folder: std::path::PathBuf::from("/work/otty"),
                list_in: std::path::PathBuf::from("/work"),
                prefix: String::from("otty"),
            },
            is_folder,
            suggestions: Vec::new(),
        })
    }

    #[test]
    fn the_work_answers_with_what_it_gave() {
        let answers = run(on_own_thread(
            String::from("looking up a folder"),
            || true,
            || false,
            folder_looked_up,
        ));

        assert_eq!(answers, [folder_looked_up(true)]);
    }

    #[test]
    fn work_that_stops_without_an_answer_answers_with_the_stopped_value() {
        let answers = run(on_own_thread(
            String::from("looking up a folder"),
            || -> bool { panic!("the read breaks") },
            || false,
            folder_looked_up,
        ));

        assert_eq!(answers, [folder_looked_up(false)]);
    }

    #[test]
    fn each_source_fails_because_the_read_stopped() {
        let failures = failed_because_stopped(&[
            SourceId::LiteLlmPrices,
            SourceId::KeptPrices,
        ]);

        assert_eq!(
            failures,
            [
                SourceFailure::new(SourceId::LiteLlmPrices, read_stopped()),
                SourceFailure::new(SourceId::KeptPrices, read_stopped()),
            ]
        );
    }
}
