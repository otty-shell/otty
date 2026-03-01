use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::quick_launch::model::{NodePath, QuickLaunch};
use crate::widgets::quick_launch::{
    QuickLaunchCtx, QuickLaunchEffect, QuickLaunchEvent, QuickLaunchUiEvent,
};
use crate::widgets::sidebar::{SidebarEvent, SidebarUiEvent};
use crate::widgets::tabs::TabsCommand;

/// Route a quick launch event through widget reduction or app orchestration.
pub(crate) fn route(app: &mut App, event: QuickLaunchEvent) -> Task<AppEvent> {
    match event {
        QuickLaunchEvent::Ui(event) => route_ui_event(app, event),
        QuickLaunchEvent::Effect(effect) => route_effect_event(app, effect),
    }
}

fn route_ui_event(app: &mut App, event: QuickLaunchUiEvent) -> Task<AppEvent> {
    // The add button lives in the quick launch panel but triggers the
    // sidebar add-menu overlay, so redirect instead of reducing.
    if matches!(event, QuickLaunchUiEvent::HeaderAddButtonPressed) {
        return Task::done(AppEvent::Sidebar(SidebarEvent::Ui(
            SidebarUiEvent::AddMenuOpen,
        )));
    }

    let ctx = build_ctx_from_parts(
        &app.terminal_settings,
        app.widgets.sidebar.cursor(),
        app.widgets.sidebar.is_resizing(),
    );
    app.widgets
        .quick_launch
        .reduce(event, &ctx)
        .map(AppEvent::QuickLaunch)
}

fn route_effect_event(
    app: &mut App,
    effect: QuickLaunchEffect,
) -> Task<AppEvent> {
    match effect {
        QuickLaunchEffect::OpenWizardCreateTab { parent_path } => {
            open_wizard_create_tab(app, parent_path)
        },
        QuickLaunchEffect::OpenWizardEditTab { path, command } => {
            open_wizard_edit_tab(app, path, command)
        },
        QuickLaunchEffect::OpenCommandTerminalTab {
            title, settings, ..
        } => open_command_terminal_tab(app, title, settings),
        QuickLaunchEffect::OpenErrorTab { title, message } => {
            open_error_tab(app, title, message)
        },
        QuickLaunchEffect::CloseTabRequested { tab_id } => {
            Task::done(AppEvent::CloseTab { tab_id })
        },
    }
}

fn build_ctx_from_parts<'a>(
    terminal_settings: &'a otty_ui_term::settings::Settings,
    sidebar_cursor: iced::Point,
    sidebar_is_resizing: bool,
) -> QuickLaunchCtx<'a> {
    QuickLaunchCtx {
        terminal_settings,
        sidebar_cursor,
        sidebar_is_resizing,
    }
}

fn open_wizard_create_tab(
    app: &mut App,
    parent_path: NodePath,
) -> Task<AppEvent> {
    app.pending_workflows
        .push_quick_launch_wizard_create(parent_path);

    Task::done(AppEvent::TabsCommand(TabsCommand::OpenWizardTab {
        title: String::from("Create Quick Launch"),
    }))
}

fn open_wizard_edit_tab(
    app: &mut App,
    path: NodePath,
    command: Box<QuickLaunch>,
) -> Task<AppEvent> {
    let title = format!("Edit: {}", command.title());
    app.pending_workflows
        .push_quick_launch_wizard_edit(path, command);

    Task::done(AppEvent::TabsCommand(TabsCommand::OpenWizardTab { title }))
}

fn open_command_terminal_tab(
    app: &mut App,
    title: String,
    settings: otty_ui_term::settings::Settings,
) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();

    Task::done(AppEvent::TabsCommand(TabsCommand::OpenCommandTab {
        terminal_id,
        title,
        settings: Box::new(settings),
    }))
}

fn open_error_tab(
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
