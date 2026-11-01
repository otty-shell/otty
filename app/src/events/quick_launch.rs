use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::domain::quick_launch::WizardTabInit;
use crate::i18n::{self, Key};
use crate::widgets::quick_launch::{
    QuickLaunchCtx, QuickLaunchEffect, QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::tabs::types::TabMachine;
use crate::widgets::tabs::{TabsEvent, TabsIntent};

pub(crate) fn handle(app: &mut App, event: QuickLaunchEvent) -> Task<AppEvent> {
    match event {
        QuickLaunchEvent::Intent(event) => handle_intent(app, event),
        QuickLaunchEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_intent(app: &mut App, event: QuickLaunchIntent) -> Task<AppEvent> {
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
                title: i18n::t(Key::QuickLaunchCreateTabTitle).to_string(),
                init: WizardTabInit::Create { parent_path },
            })),
        ),
        QuickLaunchEffect::OpenWizardEditTab { path, command } => Task::done(
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenWizardTab {
                title: i18n::edit_tab_title(command.title()),
                init: WizardTabInit::Edit { path, command },
            })),
        ),
        QuickLaunchEffect::OpenCommandTerminalTab {
            title,
            settings,
            command,
        } => {
            let machine = match command.ssh_host() {
                Some(host) => TabMachine::Ssh(host.to_string()),
                None => TabMachine::ThisComputer,
            };

            Task::done(AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenCommandTab {
                    title,
                    settings: Box::new(settings),
                    exit_behavior: command.exit_behavior(),
                    machine,
                },
            )))
        },
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
    use otty_ui_term::settings::Settings;

    use super::handle_effect;
    use crate::events::AppEvent;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::quick_launch::QuickLaunchEffect;
    use crate::widgets::quick_launch::types::QuickLaunch;
    use crate::widgets::tabs::types::TabMachine;
    use crate::widgets::tabs::{TabsEvent, TabsIntent};

    fn launch(spec: serde_json::Value) -> QuickLaunch {
        serde_json::from_value(serde_json::json!({
            "title": "htop",
            "spec": spec,
        }))
        .expect("launch should deserialize")
    }

    fn opened_machine(command: QuickLaunch) -> Option<TabMachine> {
        let events = collect_outputs(handle_effect(
            QuickLaunchEffect::OpenCommandTerminalTab {
                title: String::from("htop"),
                settings: Settings::default(),
                command: Box::new(command),
            },
        ));

        events.into_iter().find_map(|event| match event {
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenCommandTab {
                machine,
                ..
            })) => Some(machine),
            _ => None,
        })
    }

    #[test]
    fn given_ssh_launch_when_its_tab_opens_then_the_tab_runs_on_the_ssh_host() {
        let command = launch(serde_json::json!({
            "type": "Ssh",
            "ssh": {
                "host": "laptop",
                "port": 22,
                "user": null,
                "identity_file": null,
                "extra_args": []
            }
        }));

        assert_eq!(
            opened_machine(command),
            Some(TabMachine::Ssh(String::from("laptop")))
        );
    }

    #[test]
    fn given_local_launch_when_its_tab_opens_then_the_tab_runs_on_this_computer()
     {
        let command = launch(serde_json::json!({
            "type": "Custom",
            "custom": {
                "program": "htop",
                "args": [],
                "env": [],
                "working_directory": null
            }
        }));

        assert_eq!(opened_machine(command), Some(TabMachine::ThisComputer));
    }

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
