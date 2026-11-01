use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::sidebar::{SidebarEffect, SidebarEvent};

/// Route a sidebar event: reduce its intents and run its effects.
pub(crate) fn handle(app: &mut App, event: SidebarEvent) -> Task<AppEvent> {
    match event {
        SidebarEvent::Intent(event) => {
            app.widgets.sidebar.reduce(event).map(AppEvent::Sidebar)
        },
        SidebarEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(event: SidebarEffect) -> Task<AppEvent> {
    use SidebarEffect::*;

    match event {
        SyncTerminalGridSizes => Task::done(AppEvent::SyncTerminalGridSizes),
        QuickLaunchResetInteractionState => Task::done(AppEvent::QuickLaunch(
            QuickLaunchEvent::Intent(QuickLaunchIntent::ResetInteractionState),
        )),
    }
}
