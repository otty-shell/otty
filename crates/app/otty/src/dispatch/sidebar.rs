use iced::Task;
use otty_app_quick_launch::QuickLaunchIntent;
use otty_app_slots::Component;
use otty_app_terminal::TerminalIntent;

use super::{quick_launch, terminal};
use crate::app::App;
use crate::frame::sidebar::{SidebarIntent, SidebarOutcome};
use crate::message::AppEvent;

/// Give a sidebar intent to the widget and read what it reports.
pub(crate) fn deliver(app: &mut App, intent: SidebarIntent) -> Task<AppEvent> {
    let outcomes = app.sidebar.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: SidebarOutcome) -> Task<AppEvent> {
    match outcome {
        SidebarOutcome::HeaderMenuOpened => super::take_keyboard(),
        // The pane grids take the new main area.
        SidebarOutcome::MainAreaResized => {
            terminal::deliver(app, TerminalIntent::SyncPaneGridSize)
        },
        // The Launches tree drops its hover, press and drag; it keeps no
        // hover until the drag settles.
        SidebarOutcome::PanelEdgeDragged => {
            quick_launch::deliver(app, QuickLaunchIntent::ResetInteractionState)
        },
        // The terminal decides: in Terminal mode the active pane gets its
        // keys back; in Containers mode nothing gets them.
        SidebarOutcome::HeaderMenuClosed => {
            terminal::deliver(app, TerminalIntent::FocusActive)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch;
    use crate::frame::sidebar::types::HeaderMenu;
    use crate::testing::task_outputs::{
        focuses_nothing, runs_widget_operation,
    };
    use crate::testing::window::{app, open_shell_tab};

    /// The header menu takes the keyboard from the focused terminal, so
    /// typed keys do not reach its shell and Esc reaches the frame.
    #[test]
    fn an_opened_header_menu_takes_the_keyboard_focus() {
        let mut app = app();

        let task = deliver(
            &mut app,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        );

        assert!(app.sidebar.has_header_menu_open());
        assert!(focuses_nothing(task));
    }

    /// In Terminal mode the active pane gets its keys back when the header
    /// menu closes.
    #[test]
    fn a_closed_header_menu_gives_the_pane_its_keys() {
        let mut app = app();
        let _tab = open_shell_tab(&mut app);
        let _open = deliver(
            &mut app,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        );

        let task = deliver(&mut app, SidebarIntent::HeaderMenuDismiss);

        assert!(!app.sidebar.has_header_menu_open());
        assert!(runs_widget_operation(task));
    }

    /// A side panel that closes gives the pane grid the room it had: the
    /// new main area reaches the terminal, and its panes split into it.
    #[test]
    fn a_closed_side_panel_gives_the_pane_grid_its_room() {
        let mut app = app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(iced::Size::new(
                290.0, 600.0,
            ))),
        );
        let _tab = open_shell_tab(&mut app);
        let split = || {
            AppEvent::Terminal(
                otty_app_terminal::TerminalIntent::SplitFocusedPane {
                    axis: iced::widget::pane_grid::Axis::Vertical,
                },
            )
        };
        let _ = dispatch::dispatch(&mut app, split());
        let with_panels = app.terminal.pane_count();

        let _ = deliver(&mut app, SidebarIntent::ToggleLeft);
        let _ = deliver(&mut app, SidebarIntent::ToggleRight);
        let _ = dispatch::dispatch(&mut app, split());

        assert_eq!(with_panels, Some(1));
        assert_eq!(app.terminal.pane_count(), Some(2));
    }
}
