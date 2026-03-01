use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::quick_launch::model::{NodePath, QuickLaunch};
use crate::widgets::tabs::TabsCommand;


/// Open a terminal tab from a prepared quick launch command.
pub(crate) fn open_command_terminal_tab(
    app: &mut App,
    title: String,
    settings: otty_ui_term::settings::Settings,
    _command: QuickLaunch,
) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();

    Task::done(AppEvent::TabsCommand(TabsCommand::OpenCommandTab {
        terminal_id,
        title,
        settings: Box::new(settings),
    }))
}

/// Open an error tab for a failed quick launch.
pub(crate) fn open_error_tab(
    app: &mut App,
    title: String,
    message: String,
) -> Task<AppEvent> {
    app.pending_workflows
        .push_quick_launch_error_tab(title.clone(), message);

    Task::done(AppEvent::TabsCommand(TabsCommand::OpenErrorTab { title }))
}

#[cfg(test)]
mod tests {
    use super::{open_error_tab, open_wizard_create_tab};
    use crate::app::{App, PendingQuickLaunchWizard};

    #[test]
    fn given_wizard_create_request_when_opened_then_pending_wizard_is_queued() {
        let (mut app, _) = App::new();

        let _ = open_wizard_create_tab(&mut app, vec![String::from("Demo")]);

        match app.pending_workflows.pop_quick_launch_wizard() {
            Some(PendingQuickLaunchWizard::Create { parent_path }) => {
                assert_eq!(parent_path, vec![String::from("Demo")]);
            },
            Some(PendingQuickLaunchWizard::Edit { .. }) => {
                panic!("unexpected edit continuation")
            },
            None => panic!("expected pending wizard continuation"),
        }
    }

    #[test]
    fn given_error_tab_request_when_opened_then_payload_is_queued() {
        let (mut app, _) = App::new();

        let _ = open_error_tab(
            &mut app,
            String::from("Failed"),
            String::from("boom"),
        );

        let payload = app
            .pending_workflows
            .pop_quick_launch_error_tab()
            .expect("expected pending error payload");
        let (title, message) = payload.into_parts();
        assert_eq!(title, "Failed");
        assert_eq!(message, "boom");
    }
}
