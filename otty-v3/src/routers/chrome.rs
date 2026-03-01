use iced::{Task, window};

use crate::app::{App, AppEvent};
use crate::widgets::chrome::{ChromeEffect, ChromeEvent, ChromeUiEvent};
use crate::widgets::sidebar::SidebarEvent;

/// Route a chrome event through widget reduction or app orchestration.
pub(crate) fn route(app: &mut App, event: ChromeEvent) -> Task<AppEvent> {
    match event {
        ChromeEvent::Ui(event) => route_ui_event(app, event),
        ChromeEvent::Effect(effect) => route_effect_event(effect),
    }
}

fn route_ui_event(app: &mut App, event: ChromeUiEvent) -> Task<AppEvent> {
    app.widgets.chrome.reduce(event).map(AppEvent::Chrome)
}

fn route_effect_event(effect: ChromeEffect) -> Task<AppEvent> {
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
        StartWindowDrag => window::latest().and_then(window::drag),
    }
}
