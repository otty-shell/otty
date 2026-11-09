use iced::{Subscription, window};

use crate::app::App;
use crate::event::AppEvent;
use crate::widgets::quick_launch::QUICK_LAUNCHES_TICK_MS;
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Build the active subscription set from current app state.
pub(super) fn subscription(app: &App) -> Subscription<AppEvent> {
    let win_subs = window::events().map(|(_id, event)| AppEvent::Window(event));

    let mut subs = vec![win_subs];

    // Subscribe to terminal widget events for every open terminal pane.
    for (&_tab_id, tab) in app.widgets.terminal_workspace.tabs() {
        for entry in tab.terminals().values() {
            let sub = entry.terminal().subscription().map(|event| {
                AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::Widget(event),
                ))
            });
            subs.push(sub);
        }
    }

    subs.push(app.widgets.explorer.subscription().map(AppEvent::Explorer));

    // Containers mode only: engine events keep the tables fresh, and the logs
    // of the selected container follow while its Logs tab is open.
    subs.push(
        app.widgets
            .containers
            .subscription()
            .map(AppEvent::Containers),
    );

    // The toast timer, while a toast shows.
    subs.push(
        app.widgets
            .notifications
            .subscription()
            .map(AppEvent::Notifications),
    );

    // Quick launch tick for launch indicators and auto-persist
    if app.widgets.quick_launch.has_active_launches()
        || app.widgets.quick_launch.state_is_dirty()
        || app.widgets.quick_launch.persist_in_flight()
    {
        let tick = iced::time::every(std::time::Duration::from_millis(
            QUICK_LAUNCHES_TICK_MS,
        ))
        .map(|_| {
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::Tick,
            ))
        });
        subs.push(tick);
    }

    // Frames for the focus mode note while it stays and fades out
    if app.widgets.sidebar.is_focus_note_shown() {
        let frames = window::frames().map(|now| {
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::FocusNoteFrame(now),
            ))
        });
        subs.push(frames);
    }

    Subscription::batch(subs)
}
