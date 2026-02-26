use iced::Task;

use super::{runtime, sidebar};
use crate::app::{App, Event as AppEvent};
use crate::widgets::explorer::ExplorerUiEvent;
use crate::widgets::sidebar::SidebarUiEvent;
use crate::widgets::terminal::TerminalEvent;

/// Route terminal UI event into widget reduction and follow-up tasks.
pub(crate) fn route_event(
    app: &mut App,
    event: TerminalEvent,
) -> Task<AppEvent> {
    let sidebar_task =
        if let TerminalEvent::PaneGridCursorMoved { position, .. } = &event {
            sidebar::route_event(
                app,
                SidebarUiEvent::PaneGridCursorMoved {
                    position: *position,
                },
            )
        } else {
            Task::none()
        };

    let ctx = runtime::make_terminal_ctx(app);
    let sync_task = terminal_sync_followup(&event);
    let terminal_task = app.widgets.terminal_mut().reduce(event, &ctx);
    Task::batch(vec![sidebar_task, terminal_task, sync_task])
}

fn terminal_sync_followup(event: &TerminalEvent) -> Task<AppEvent> {
    let should_sync = matches!(
        event,
        TerminalEvent::PaneClicked { .. }
            | TerminalEvent::SplitPane { .. }
            | TerminalEvent::ClosePane { .. }
            | TerminalEvent::Widget(otty_ui_term::Event::ContentSync { .. })
    );

    if should_sync {
        Task::done(AppEvent::ExplorerUi(
            ExplorerUiEvent::SyncFromActiveTerminal,
        ))
    } else {
        Task::none()
    }
}
