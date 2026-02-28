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
    use ChromeEffect::*;

    match effect {
        FullScreenToggled { mode } => {
            window::latest().and_then(move |id| window::set_mode(id, mode))
        },
        MinimizeWindow => {
            window::latest().and_then(|id| window::minimize(id, true))
        },
        CloseWindow => window::latest().and_then(window::close),
        ToggleSidebarVisibility => {
            Task::done(AppEvent::SidebarUi(SidebarEvent::ToggleVisibility))
        },
        StartWindowDrag => {
            window::latest().and_then(window::drag)
        },
    }
}

fn map_chrome_event_to_command(event: ChromeEvent) -> ChromeCommand {
    use {ChromeCommand as C, ChromeEvent as E};

    match event {
        E::ToggleFullScreen => C::ToggleFullScreen,
        E::MinimizeWindow => C::MinimizeWindow,
        E::CloseWindow => C::CloseWindow,
        E::ToggleSidebarVisibility => C::ToggleSidebarVisibility,
        E::StartWindowDrag => C::StartWindowDrag,
    }
}
