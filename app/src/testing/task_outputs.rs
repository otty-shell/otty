//! Test-only helpers that read what a [`Task`] produces.
//!
//! Reducers return a [`Task`] instead of a list of effects, so tests use
//! these helpers to see which events a reducer emitted and whether it asked
//! for a widget operation, such as moving keyboard focus.

use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::Focusable;
use iced::futures::executor::block_on;
use iced::futures::{FutureExt, StreamExt};
use iced::widget::Id;
use iced::{Rectangle, Task};
use iced_test::runtime::Action;
use iced_test::runtime::task::into_stream;

use crate::focus;

/// A focusable widget for a focus operation to act on, in place of a widget
/// of the window.
struct Probe {
    is_focused: bool,
}

impl Focusable for Probe {
    fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn focus(&mut self) {
        self.is_focused = true;
    }

    fn unfocus(&mut self) {
        self.is_focused = false;
    }
}

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

/// Return whether `task` asks for a widget operation, such as moving
/// keyboard focus. [`collect_outputs`] skips these.
///
/// # Example
///
/// ```ignore
/// let focus = iced::widget::operation::focus::<()>(iced::widget::Id::new("x"));
/// assert!(runs_widget_operation(focus));
/// ```
pub(crate) fn runs_widget_operation<T>(task: Task<T>) -> bool {
    let Some(stream) = into_stream(task) else {
        return false;
    };
    let actions: Vec<Action<T>> = block_on(stream.collect());

    actions
        .iter()
        .any(|action| matches!(action, Action::Widget(_)))
}

/// Return whether `task` takes the keyboard focus from every widget, as
/// [`focus::nothing`] does.
///
/// Unlike [`collect_outputs`], it reads only what the task has ready, so a
/// widget operation that waits for an answer, such as a command to a
/// terminal, does not keep it waiting.
///
/// # Example
///
/// ```ignore
/// assert!(focuses_nothing(crate::focus::nothing::<()>()));
/// ```
pub(crate) fn focuses_nothing<T>(task: Task<T>) -> bool {
    ready_actions(task).iter_mut().any(|action| match action {
        Action::Widget(operation) => {
            focuses(operation.as_mut(), &focus::NOTHING)
        },
        _ => false,
    })
}

/// The actions `task` has ready now, in order.
fn ready_actions<T>(task: Task<T>) -> Vec<Action<T>> {
    let Some(mut stream) = into_stream(task) else {
        return Vec::new();
    };
    let mut actions = Vec::new();

    while let Some(Some(action)) = stream.next().now_or_never() {
        actions.push(action);
    }

    actions
}

/// Whether `operation` focuses a widget with `id`.
fn focuses(operation: &mut dyn Operation, id: &Id) -> bool {
    let mut probe = Probe { is_focused: false };

    operation.focusable(Some(id), Rectangle::default(), &mut probe);

    probe.is_focused
}

#[cfg(test)]
mod tests {
    use iced::Task;

    use super::{collect_outputs, focuses_nothing};
    use crate::focus;

    #[test]
    fn focusing_nothing_takes_the_focus_from_every_widget() {
        let other = iced::widget::operation::focus::<()>(
            iced::widget::Id::new("field"),
        );

        assert!(focuses_nothing(focus::nothing::<()>()));
        assert!(!focuses_nothing(other));
        assert!(!focuses_nothing(Task::done(())));
    }

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
