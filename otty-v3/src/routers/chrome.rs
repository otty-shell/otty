use iced::{Task, window};

use crate::app::{App, AppEvent};
use crate::widgets::chrome::{ChromeCommand, ChromeEffect, ChromeEvent};
use crate::widgets::sidebar::SidebarEvent;

/// Route a chrome UI event through the widget reducer and map effects.
pub(crate) fn route_event(app: &mut App, event: ChromeEvent) -> Task<AppEvent> {
    let command = map_chrome_event_to_command(event);
    app.widgets
        .chrome
        .reduce(command)
        .map(AppEvent::ChromeEffect)
}

/// Route a chrome effect event to an app-level task.
pub(crate) fn route_effect(effect: ChromeEffect) -> Task<AppEvent> {
    match effect {
        ChromeEffect::FullScreenToggled { mode } => {
            window::latest().and_then(move |id| window::set_mode(id, mode))
        },
        ChromeEffect::MinimizeWindow => {
            window::latest().and_then(|id| window::minimize(id, true))
        },
        ChromeEffect::CloseWindow => window::latest().and_then(window::close),
        ChromeEffect::ToggleSidebarVisibility => {
            Task::done(AppEvent::SidebarUi(SidebarEvent::ToggleVisibility))
        },
        ChromeEffect::StartWindowDrag => {
            window::latest().and_then(window::drag)
        },
    }
}

fn map_chrome_event_to_command(event: ChromeEvent) -> ChromeCommand {
    match event {
        ChromeEvent::ToggleFullScreen => ChromeCommand::ToggleFullScreen,
        ChromeEvent::MinimizeWindow => ChromeCommand::MinimizeWindow,
        ChromeEvent::CloseWindow => ChromeCommand::CloseWindow,
        ChromeEvent::ToggleSidebarVisibility => {
            ChromeCommand::ToggleSidebarVisibility
        },
        ChromeEvent::StartWindowDrag => ChromeCommand::StartWindowDrag,
    }
}
