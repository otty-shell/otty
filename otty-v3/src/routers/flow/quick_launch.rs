use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::quick_launch::model::{NodePath, QuickLaunch};
use crate::widgets::tabs::TabsCommand;

/// Open a wizard tab in create mode for a quick launch command.
pub(crate) fn open_wizard_create_tab(
    app: &mut App,
    parent_path: NodePath,
) -> Task<AppEvent> {
    app.pending_workflows
        .push_quick_launch_wizard_create(parent_path);

    Task::done(AppEvent::TabsCommand(TabsCommand::OpenWizardTab {
        title: String::from("Create Quick Launch"),
    }))
}

/// Open a wizard tab in edit mode for an existing command.
pub(crate) fn open_wizard_edit_tab(
    app: &mut App,
    path: NodePath,
    command: Box<QuickLaunch>,
) -> Task<AppEvent> {
    let title = format!("Edit: {}", command.title());
    app.pending_workflows
        .push_quick_launch_wizard_edit(path, command);

    Task::done(AppEvent::TabsCommand(TabsCommand::OpenWizardTab { title }))
}

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

/// Close a tab by id.
pub(crate) fn close_tab(_app: &mut App, tab_id: u64) -> Task<AppEvent> {
    Task::done(AppEvent::TabsCommand(TabsCommand::Close { tab_id }))
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
