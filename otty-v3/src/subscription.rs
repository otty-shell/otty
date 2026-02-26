use iced::{Subscription, window};

use crate::app::{App, AppEvent};
use crate::widgets::quick_launch::event::QUICK_LAUNCHES_TICK_MS;

/// Build the active subscription set from current app state.
pub(super) fn subscription(app: &App) -> Subscription<AppEvent> {
    let win_subs = window::events().map(|(_id, event)| AppEvent::Window(event));
    let key_subs = iced::keyboard::listen().map(AppEvent::Keyboard);

    let mut subs = vec![win_subs, key_subs];

    // TODO: add terminal subscriptions when terminal workspace widget exists

    // Quick launch tick for launch indicators and auto-persist
    if app.widgets.quick_launch.has_active_launches()
        || app.widgets.quick_launch.state_is_dirty()
    {
        let tick = iced::time::every(std::time::Duration::from_millis(
            QUICK_LAUNCHES_TICK_MS,
        ))
        .map(|_| {
            AppEvent::QuickLaunchUi(
                crate::widgets::quick_launch::QuickLaunchEvent::Tick,
            )
        });
        subs.push(tick);
    }

    Subscription::batch(subs)
}
