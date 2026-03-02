use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::domain::quick_launch::WizardTabInit;
use crate::widgets::quick_launch::{
    QuickLaunchCtx, QuickLaunchEffect, QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
use crate::widgets::tabs::{TabsEvent, TabsIntent};

pub(crate) fn handle(app: &mut App, event: QuickLaunchEvent) -> Task<AppEvent> {
    match event {
        QuickLaunchEvent::Intent(event) => handle_intent(app, event),
        QuickLaunchEvent::Effect(effect) => handle_effect(effect),
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

    let ctx = QuickLaunchCtx {
        terminal_settings: &app.terminal_settings,
        sidebar_cursor: app.widgets.sidebar.cursor(),
        sidebar_is_resizing: app.widgets.sidebar.is_resizing(),
    };

    app.widgets
        .quick_launch
        .reduce(event, &ctx)
        .map(AppEvent::QuickLaunch)
}

fn handle_effect(effect: QuickLaunchEffect) -> Task<AppEvent> {
    match effect {
        QuickLaunchEffect::OpenWizardCreateTab { parent_path } => Task::done(
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenWizardTab {
                title: String::from("Create Quick Launch"),
                init: WizardTabInit::Create { parent_path },
            })),
        ),
        QuickLaunchEffect::OpenWizardEditTab { path, command } => Task::done(
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenWizardTab {
                title: format!("Edit: {}", command.title()),
                init: WizardTabInit::Edit { path, command },
            })),
        ),
        QuickLaunchEffect::OpenCommandTerminalTab {
            title, settings, ..
        } => Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::OpenCommandTab {
                title,
                settings: Box::new(settings),
            },
        ))),
        QuickLaunchEffect::OpenErrorTab { title, message } => {
            Task::done(AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenErrorTab { title, message },
            )))
        },
        QuickLaunchEffect::CloseTabRequested { tab_id } => Task::done(
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab { tab_id })),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::handle_effect;
    use crate::widgets::quick_launch::QuickLaunchEffect;

    #[test]
    fn given_wizard_create_request_when_opened_then_task_is_emitted() {
        let task = handle_effect(QuickLaunchEffect::OpenWizardCreateTab {
            parent_path: vec![String::from("Demo")],
        });
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn given_error_tab_request_when_opened_then_task_is_emitted() {
        let task = handle_effect(QuickLaunchEffect::OpenErrorTab {
            title: String::from("Failed"),
            message: String::from("boom"),
        });
        assert_eq!(task.units(), 1);
    }
}
