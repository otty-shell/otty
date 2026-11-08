use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::sidebar::event::{SidebarEffect, SidebarEvent};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Route a sidebar event: reduce its intents and run its effects.
pub(crate) fn route(app: &mut App, event: SidebarEvent) -> Task<AppEvent> {
    match event {
        SidebarEvent::Intent(intent) => {
            app.widgets.sidebar.reduce(intent).map(AppEvent::Sidebar)
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
        // The active pane gets its keys back; in Containers mode its pane
        // is not on screen, so nothing gets them.
        HeaderMenuClosed => Task::done(AppEvent::TerminalWorkspace(
            TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive,
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    #[test]
    fn a_closed_header_menu_gives_the_active_terminal_its_keys_back() {
        let events =
            collect_outputs(handle_effect(SidebarEffect::HeaderMenuClosed));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))]
        ));
    }
}
