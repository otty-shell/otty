use iced::{Task, window};

use super::AppEvent;
use crate::app::App;
use crate::widgets::chrome::{ChromeEffect, ChromeEvent};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
use crate::widgets::tabs::{TabsEvent, TabsIntent};

pub(crate) fn handle(app: &mut App, event: ChromeEvent) -> Task<AppEvent> {
    match event {
        ChromeEvent::Intent(event) => {
            app.widgets.chrome.reduce(event).map(AppEvent::Chrome)
        },
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
        ToggleSidebarVisibility => sidebar_intent(SidebarIntent::ToggleLeft),
        ToggleFilesPanel => sidebar_intent(SidebarIntent::ToggleRight),
        OpenSettings => Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::OpenSettingsTab,
        ))),
        StartWindowDrag => window::latest().and_then(window::drag),
    }
}

fn sidebar_intent(intent: SidebarIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Sidebar(SidebarEvent::Intent(intent)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_outputs::collect_outputs;

    #[test]
    fn logo_toggle_maps_to_the_left_panel_toggle() {
        let events = collect_outputs(handle_effect(
            ChromeEffect::ToggleSidebarVisibility,
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::ToggleLeft
            ))]
        ));
    }

    #[test]
    fn files_panel_button_maps_to_the_right_panel_toggle() {
        let events =
            collect_outputs(handle_effect(ChromeEffect::ToggleFilesPanel));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::ToggleRight
            ))]
        ));
    }

    #[test]
    fn settings_button_opens_the_settings_tab() {
        let events = collect_outputs(handle_effect(ChromeEffect::OpenSettings));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenSettingsTab
            ))]
        ));
    }
}
