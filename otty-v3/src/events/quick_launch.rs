use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::quick_launch::model::{NodePath, QuickLaunch};
use crate::widgets::quick_launch::{
    QuickLaunchCtx, QuickLaunchEffect, QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
use crate::widgets::tabs::{TabsEvent, TabsIntent};

pub(crate) fn handle(app: &mut App, event: QuickLaunchEvent) -> Task<AppEvent> {
    match event {
        QuickLaunchEvent::Intent(event) => handle_intent(app, event),
        QuickLaunchEvent::Effect(effect) => handle_effect(app, effect),
    }
}

fn handle_intent(app: &mut App, event: QuickLaunchIntent) -> Task<AppEvent> {
    // The add button lives in the quick launch panel but triggers the
    // sidebar add-menu overlay, so redirect instead of reducing.
    if matches!(event, QuickLaunchIntent::HeaderAddButtonPressed) {
        return Task::done(AppEvent::Sidebar(SidebarEvent::Intent(
            SidebarIntent::AddMenuOpen,
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

fn handle_effect(app: &mut App, effect: QuickLaunchEffect) -> Task<AppEvent> {
    match effect {
        QuickLaunchEffect::OpenWizardCreateTab { parent_path } => {
            open_wizard_create_tab(app, parent_path)
        },
        QuickLaunchEffect::OpenWizardEditTab { path, command } => {
            open_wizard_edit_tab(app, path, command)
        },
        QuickLaunchEffect::OpenCommandTerminalTab {
            title, settings, ..
        } => open_command_terminal_tab(title, settings),
        QuickLaunchEffect::OpenErrorTab { title, message } => {
            open_error_tab(app, title, message)
        },
        QuickLaunchEffect::CloseTabRequested { tab_id } => Task::done(
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab { tab_id })),
        ),
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

    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenWizardTab {
            title: String::from("Create Quick Launch"),
        },
    )))
}

fn open_wizard_edit_tab(
    app: &mut App,
    path: NodePath,
    command: Box<QuickLaunch>,
) -> Task<AppEvent> {
    let title = format!("Edit: {}", command.title());
    app.pending_workflows
        .push_quick_launch_wizard_edit(path, command);

    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenWizardTab { title },
    )))
}

fn open_command_terminal_tab(
    title: String,
    settings: otty_ui_term::settings::Settings,
) -> Task<AppEvent> {
    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenCommandTab {
            title,
            settings: Box::new(settings),
        },
    )))
}

fn open_error_tab(
    app: &mut App,
    title: String,
    message: String,
) -> Task<AppEvent> {
    app.pending_workflows
        .push_quick_launch_error_tab(title.clone(), message);

    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenErrorTab { title },
    )))
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
