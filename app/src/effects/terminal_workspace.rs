use iced::Task;

use super::notifications;
use crate::app::App;
use crate::dispatch::AppEvent;
use crate::widgets::explorer::event::{ExplorerEvent, ExplorerIntent};
use crate::widgets::modes::event::{ModesEvent, ModesIntent};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::event::TerminalWorkspaceEffect;

/// Run a terminal workspace effect: tell the tabs, the modes widget and the
/// Files panel what the terminals did, and tell the user about a failure.
pub(crate) fn run(
    app: &App,
    effect: TerminalWorkspaceEffect,
) -> Task<AppEvent> {
    match effect {
        TerminalWorkspaceEffect::TabClosed { tab_id } => Task::done(
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab { tab_id })),
        ),
        TerminalWorkspaceEffect::Failed(text) => notifications::push(text),
        // Tabs only show in Terminal mode (spec Decision 5). A tab tells the
        // modes widget once its terminal runs, so a tab that cannot start
        // leaves the mode as it was.
        TerminalWorkspaceEffect::TabStarted => Task::done(AppEvent::Modes(
            ModesEvent::Intent(ModesIntent::TabShown),
        )),
        TerminalWorkspaceEffect::TitleChanged {
            tab_id,
            title,
            folder,
        } => Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::SetTitle {
                tab_id,
                title,
                folder,
            },
        ))),
        TerminalWorkspaceEffect::SyncExplorer => {
            sync_explorer_from_terminal(app)
        },
    }
}

/// Show the folder of the active terminal in the Files panel; nothing is
/// sent when the active tab has no local folder.
fn sync_explorer_from_terminal(app: &App) -> Task<AppEvent> {
    let active_tab_id = app.widgets.tabs.active_tab_id();
    let Some(cwd) = app
        .widgets
        .terminal_workspace
        .local_cwd_for_active_tab(active_tab_id)
    else {
        return Task::none();
    };

    Task::done(AppEvent::Explorer(ExplorerEvent::Intent(
        ExplorerIntent::SyncRoot { cwd },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };

    #[test]
    fn a_failure_becomes_a_notification() {
        let (app, _) = App::new();

        let events = collect_outputs(run(
            &app,
            TerminalWorkspaceEffect::Failed(String::from("boom")),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if text == "boom"
        ));
    }

    /// Tabs only show in Terminal mode, so a tab whose terminal started
    /// tells the modes widget, which switches from another mode.
    #[test]
    fn a_started_tab_tells_the_modes_widget_it_is_shown() {
        let (app, _) = App::new();

        let events =
            collect_outputs(run(&app, TerminalWorkspaceEffect::TabStarted));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::TabShown))]
        ));
    }
}
