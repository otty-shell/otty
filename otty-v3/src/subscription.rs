use iced::{Subscription, window};

use crate::app::{App, AppEvent};
use crate::widgets::quick_launch::event::QUICK_LAUNCHES_TICK_MS;
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchUiEvent};
use crate::widgets::terminal_workspace::TerminalWorkspaceEvent;

/// Build the active subscription set from current app state.
pub(super) fn subscription(app: &App) -> Subscription<AppEvent> {
    let win_subs = window::events().map(|(_id, event)| AppEvent::Window(event));
    let key_subs = iced::keyboard::listen().map(AppEvent::Keyboard);

    let mut subs = vec![win_subs, key_subs];

    // Subscribe to terminal widget events for every open terminal pane.
    for (&_tab_id, tab) in app.widgets.terminal_workspace.tabs() {
        for entry in tab.terminals().values() {
            let sub = entry.terminal().subscription().map(|event| {
                AppEvent::TerminalWorkspaceUi(TerminalWorkspaceEvent::Widget(
                    event,
                ))
            });
            subs.push(sub);
        }
    }

    // Quick launch tick for launch indicators and auto-persist
    if app.widgets.quick_launch.has_active_launches()
        || app.widgets.quick_launch.state_is_dirty()
        || app.widgets.quick_launch.persist_in_flight()
    {
        let tick = iced::time::every(std::time::Duration::from_millis(
            QUICK_LAUNCHES_TICK_MS,
        ))
        .map(|_| {
            AppEvent::QuickLaunch(QuickLaunchEvent::Ui(
                QuickLaunchUiEvent::Tick,
            ))
        });
        subs.push(tick);
    }

    Subscription::batch(subs)
}
