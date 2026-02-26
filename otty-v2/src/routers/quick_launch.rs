use iced::Task;

use super::runtime;
use crate::app::{App, Event as AppEvent};
use crate::guards::inline_edit_guard;
use crate::widgets::quick_launch;

/// Route quick-launch UI event into widget reduction.
pub(crate) fn route_event(
    app: &mut App,
    event: quick_launch::QuickLaunchEvent,
) -> Task<AppEvent> {
    let ctx = runtime::quick_launch_ctx_from_values(
        &app.terminal_settings,
        app.widgets.sidebar().cursor(),
        app.widgets.sidebar().is_resizing(),
    );
    app.widgets.quick_launch_mut().reduce(event, &ctx)
}

/// Build optional pre-dispatch task for inline-edit cancellation.
pub(crate) fn pre_dispatch_inline_edit_cancel(
    app: &mut App,
    event: &AppEvent,
) -> Option<Task<AppEvent>> {
    if app.widgets.quick_launch().inline_edit().is_some()
        && inline_edit_guard(event)
    {
        let ctx = runtime::quick_launch_ctx_from_values(
            &app.terminal_settings,
            app.widgets.sidebar().cursor(),
            app.widgets.sidebar().is_resizing(),
        );
        Some(
            app.widgets
                .quick_launch_mut()
                .reduce(quick_launch::QuickLaunchEvent::CancelInlineEdit, &ctx),
        )
    } else {
        None
    }
}

/// Handle keyboard events mapped to quick-launch interactions.
pub(crate) fn route_keyboard(
    app: &mut App,
    event: iced::keyboard::Event,
) -> Task<AppEvent> {
    if let iced::keyboard::Event::KeyPressed { key, .. } = event {
        let ctx = runtime::quick_launch_ctx_from_values(
            &app.terminal_settings,
            app.widgets.sidebar().cursor(),
            app.widgets.sidebar().is_resizing(),
        );
        if matches!(
            key,
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
        ) && app.widgets.quick_launch().inline_edit().is_some()
        {
            return app.widgets.quick_launch_mut().reduce(
                quick_launch::QuickLaunchEvent::CancelInlineEdit,
                &ctx,
            );
        }

        if matches!(
            key,
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
        ) && app.widgets.quick_launch().inline_edit().is_none()
        {
            return app
                .widgets
                .quick_launch_mut()
                .reduce(quick_launch::QuickLaunchEvent::DeleteSelected, &ctx);
        }
    }

    Task::none()
}
