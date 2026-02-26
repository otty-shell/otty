use std::time::Duration;

use iced::{Subscription, window};

use crate::app::{App, Event};
use crate::features::quick_launch;
use crate::features::terminal::TerminalEvent;

pub(super) fn subscription(app: &App) -> Subscription<Event> {
    let mut subscriptions = Vec::new();
    for (&tab_id, terminal) in app.features.terminal().tabs() {
        for entry in terminal.terminals().values() {
            let sub = entry.terminal.subscription().with(tab_id).map(
                |(_tab_id, event)| {
                    Event::Terminal(TerminalEvent::Widget(event))
                },
            );
            subscriptions.push(sub);
        }
    }

    let terminal_subs = Subscription::batch(subscriptions);
    let win_subs = window::events().map(|(_id, event)| Event::Window(event));
    let key_subs = iced::keyboard::listen().map(Event::Keyboard);

    let mut subs = vec![terminal_subs, win_subs, key_subs];
    if app.features.quick_launch().has_active_launches()
        || app.features.quick_launch().is_dirty()
        || app.features.quick_launch().is_persist_in_flight()
    {
        subs.push(
            iced::time::every(Duration::from_millis(
                quick_launch::QUICK_LAUNCHES_TICK_MS,
            ))
            .map(|_| Event::QuickLaunch(quick_launch::QuickLaunchEvent::Tick)),
        );
    }

    Subscription::batch(subs)
}
