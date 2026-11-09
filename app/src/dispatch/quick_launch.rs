use iced::Task;

use crate::app::App;
use crate::event::AppEvent;
use crate::widgets::quick_launch::QuickLaunchCtx;
use crate::widgets::quick_launch::event::QuickLaunchIntent;

/// Deliver a launches intent to the widget's reducer, with the terminal
/// settings and the sidebar pointer state it needs.
pub(crate) fn deliver(
    app: &mut App,
    intent: QuickLaunchIntent,
) -> Task<AppEvent> {
    let ctx = QuickLaunchCtx {
        terminal_settings: &app.terminal_settings,
        sidebar_cursor: app.widgets.sidebar.cursor(),
        sidebar_is_resizing: app.widgets.sidebar.is_resizing(),
    };

    app.widgets
        .quick_launch
        .reduce(intent, &ctx)
        .map(AppEvent::QuickLaunch)
}
