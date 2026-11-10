//! Test-only helpers that read what a [`Task`] produces.
//!
//! `update` and the effects return a [`Task`], so tests use these helpers to
//! see which events it carries and whether it asks for a widget operation,
//! such as moving keyboard focus.

use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::scrollable::{
    AbsoluteOffset, RelativeOffset,
};
use iced::advanced::widget::operation::{Focusable, Scrollable};
use iced::futures::executor::block_on;
use iced::futures::{FutureExt, StreamExt};
use iced::widget::Id;
use iced::{Rectangle, Task, Vector};
use iced_test::runtime::Action;
use iced_test::runtime::task::into_stream;
use otty_app_slots::focus;

/// A focusable widget for a focus operation to act on, in place of a widget
/// of the window.
struct Probe {
    is_focused: bool,
}

/// A scrollable widget for a scroll operation to act on: it keeps where it
/// was snapped to.
struct ScrollProbe {
    snapped_to: Option<RelativeOffset<Option<f32>>>,
}

impl Scrollable for ScrollProbe {
    fn snap_to(&mut self, offset: RelativeOffset<Option<f32>>) {
        self.snapped_to = Some(offset);
    }

    fn scroll_to(&mut self, _offset: AbsoluteOffset<Option<f32>>) {}

    fn scroll_by(
        &mut self,
        _offset: AbsoluteOffset,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
    ) {
    }
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

/// Return whether `task` takes the keyboard focus from every widget, as
/// focusing [`focus::NOTHING`] does.
///
/// Unlike [`collect_outputs`], it reads only what the task has ready, so a
/// widget operation that waits for an answer, such as a command to a
/// terminal, does not keep it waiting.
///
/// # Example
///
/// ```ignore
/// let focus = iced::widget::operation::focus::<()>(focus::NOTHING);
/// assert!(focuses_nothing(focus));
/// ```
pub(crate) fn focuses_nothing<T>(task: Task<T>) -> bool {
    focuses_widget(task, &focus::NOTHING)
}

/// Return whether `task` gives the keyboard focus to the widget with `id`.
///
/// Like [`focuses_nothing`], it reads only what the task has ready.
///
/// # Example
///
/// ```ignore
/// let id = iced::widget::Id::new("field");
/// let focus = iced::widget::operation::focus::<()>(id.clone());
/// assert!(focuses_widget(focus, &id));
/// ```
pub(crate) fn focuses_widget<T>(task: Task<T>, id: &Id) -> bool {
    ready_actions(task).iter_mut().any(|action| match action {
        Action::Widget(operation) => focuses(operation.as_mut(), id),
        _ => false,
    })
}

/// Return whether `task` runs any widget operation, such as giving a
/// terminal pane the keyboard.
///
/// Like [`focuses_nothing`], it reads only what the task has ready.
///
/// # Example
///
/// ```ignore
/// let focus = iced::widget::operation::focus::<()>("pane");
/// assert!(runs_widget_operation(focus));
/// assert!(!runs_widget_operation(iced::Task::<()>::none()));
/// ```
pub(crate) fn runs_widget_operation<T>(task: Task<T>) -> bool {
    ready_actions(task)
        .iter()
        .any(|action| matches!(action, Action::Widget(_)))
}

/// Return whether `task` scrolls the scrollable with `id` to its end.
///
/// Like [`focuses_nothing`], it reads only what the task has ready.
///
/// # Example
///
/// ```ignore
/// let snap = iced::widget::operation::snap_to_end::<()>("list");
/// assert!(snaps_to_end(snap, &iced::widget::Id::new("list")));
/// ```
pub(crate) fn snaps_to_end<T>(task: Task<T>, id: &Id) -> bool {
    ready_actions(task).iter_mut().any(|action| match action {
        Action::Widget(operation) => snapped_to_end(operation.as_mut(), id),
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

/// Whether `operation` snaps a scrollable with `id` to its end.
fn snapped_to_end(operation: &mut dyn Operation, id: &Id) -> bool {
    let mut probe = ScrollProbe { snapped_to: None };

    operation.scrollable(
        Some(id),
        Rectangle::default(),
        Rectangle::default(),
        Vector::ZERO,
        &mut probe,
    );

    probe.snapped_to == Some(RelativeOffset::END.into())
}

#[cfg(test)]
mod tests {
    use iced::Task;
    use otty_app_slots::focus;

    use super::{
        collect_outputs, focuses_nothing, focuses_widget,
        runs_widget_operation, snaps_to_end,
    };

    #[test]
    fn a_widget_operation_is_found_and_a_message_is_not() {
        assert!(runs_widget_operation(iced::widget::operation::focus::<()>(
            "pane"
        )));
        assert!(!runs_widget_operation(Task::done(())));
        assert!(!runs_widget_operation(Task::<()>::none()));
    }

    #[test]
    fn snapping_a_scrollable_to_its_end_is_found_by_its_id() {
        let list = iced::widget::Id::new("list");

        assert!(snaps_to_end(
            iced::widget::operation::snap_to_end::<()>(list.clone()),
            &list
        ));
        assert!(!snaps_to_end(
            iced::widget::operation::snap_to_end::<()>("other"),
            &list
        ));
        assert!(!snaps_to_end(Task::done(()), &list));
    }

    #[test]
    fn focusing_nothing_takes_the_focus_from_every_widget() {
        let other = iced::widget::operation::focus::<()>(
            iced::widget::Id::new("field"),
        );

        assert!(focuses_nothing(iced::widget::operation::focus::<()>(
            focus::NOTHING,
        )));
        assert!(!focuses_nothing(other));
        assert!(!focuses_nothing(Task::done(())));
    }

    #[test]
    fn focusing_a_widget_is_found_by_its_id() {
        let field = iced::widget::Id::new("field");
        let focus = iced::widget::operation::focus::<()>(field.clone());

        assert!(focuses_widget(focus, &field));
        assert!(!focuses_widget(
            iced::widget::operation::focus::<()>(focus::NOTHING),
            &field
        ));
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
