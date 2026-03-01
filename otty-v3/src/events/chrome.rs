use iced::{Task, window};

use crate::app::App;
use crate::widgets::chrome::{ChromeEffect, ChromeEvent};
use crate::widgets::sidebar::{SidebarEvent, SidebarUiEvent};
use super::AppEvent;

pub(crate) fn handle(app: &mut App, event: ChromeEvent) -> Task<AppEvent> {
    match event {
        ChromeEvent::Ui(event) => app
            .widgets
            .chrome
            .reduce(event)
            .map(AppEvent::Chrome),
        ChromeEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(effect: ChromeEffect) -> Task<AppEvent> {
    use ChromeEffect::*;

    match effect {
        FullScreenToggled { mode } => {
            window::latest().and_then(move |id| window::set_mode(id, mode))
        },
        MinimizeWindow => {
            window::latest().and_then(|id| window::minimize(id, true))
        },
        CloseWindow => window::latest().and_then(window::close),
        ToggleSidebarVisibility => Task::done(AppEvent::Sidebar(
            SidebarEvent::Ui(SidebarUiEvent::ToggleVisibility),
        )),
        StartWindowDrag => window::latest().and_then(window::drag),
    }
}
