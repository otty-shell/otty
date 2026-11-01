//! Test-only helper that reads the messages a [`Task`] produces.
//!
//! Reducers return a [`Task`] instead of a list of effects, so tests use this
//! helper to see which events a reducer emitted.

use iced::Task;
use iced::futures::StreamExt;
use iced::futures::executor::block_on;
use iced_test::runtime::Action;
use iced_test::runtime::task::into_stream;

/// Run `task` to the end and return every message it produced, in the order
/// they arrived.
///
/// Only use it on tasks made of `Task::done` and `Task::batch`: tasks that do
/// real I/O (`Task::perform` on a process or a file) would run it. Widget
/// operations such as focus or scroll are skipped.
///
/// # Example
///
/// ```ignore
/// let outputs = collect_outputs(Task::done(1).chain(Task::done(2)));
/// assert_eq!(outputs, vec![1, 2]);
/// ```
pub(crate) fn collect_outputs<T>(task: Task<T>) -> Vec<T> {
    let Some(stream) = into_stream(task) else {
        return Vec::new();
    };

    let actions: Vec<Action<T>> = block_on(stream.collect());

    actions
        .into_iter()
        .filter_map(|action| match action {
            Action::Output(output) => Some(output),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use iced::Task;

    use super::collect_outputs;

    #[test]
    fn given_chained_tasks_when_collected_then_outputs_keep_their_order() {
        let outputs = collect_outputs(Task::done(1).chain(Task::done(2)));

        assert_eq!(outputs, vec![1, 2]);
    }

    #[test]
    fn given_empty_task_when_collected_then_nothing_is_returned() {
        let outputs: Vec<u8> = collect_outputs(Task::none());

        assert!(outputs.is_empty());
    }
}
