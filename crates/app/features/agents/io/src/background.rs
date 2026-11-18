//! The reads and writes of Agents mode run on a thread of their own, so a
//! slow disk, server or shell never holds up the window.

use iced::Task;
use iced::futures::channel::oneshot;
use otty_app_agents::AgentsIntent;

/// Run `work` on a thread of its own and answer with `answer` of what it
/// gave. When the thread can not start, or ends without an answer because
/// `work` panicked, `stopped` gives what to answer with instead.
pub(crate) fn in_background<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    stopped: impl FnOnce() -> T + Send + 'static,
    answer: impl FnOnce(T) -> AgentsIntent + Send + 'static,
) -> Task<AgentsIntent> {
    Task::perform(on_thread(work), move |result| {
        answer(result.unwrap_or_else(stopped))
    })
}

/// Run `work` on a thread of its own and wait for its answer without
/// holding up the runtime. Not on the runtime's blocking pool: closing the
/// runtime waits for that pool, so a read stuck on a slow shell would keep
/// the app from exiting. `None` when the thread can not start or ends
/// without an answer.
async fn on_thread<T: Send + 'static>(
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
        let answers = run(in_background(|| true, || false, folder_looked_up));

        assert_eq!(answers, [folder_looked_up(true)]);
    }

    #[test]
    fn work_that_stops_without_an_answer_answers_with_the_stopped_value() {
        let answers = run(in_background(
            || -> bool { panic!("the read breaks") },
            || false,
            folder_looked_up,
        ));

        assert_eq!(answers, [folder_looked_up(false)]);
    }
}
