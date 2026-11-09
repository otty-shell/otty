use iced::Task;

use crate::dispatch::AppEvent;
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::sidebar::event::SidebarEffect;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Run a sidebar effect: ask for new terminal grid sizes, or tell another
/// widget what the sidebar did.
pub(crate) fn run(effect: SidebarEffect) -> Task<AppEvent> {
    use SidebarEffect::*;

    match effect {
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
        let events = collect_outputs(run(SidebarEffect::HeaderMenuClosed));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))]
        ));
    }
}
