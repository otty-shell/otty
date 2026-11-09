use iced::Task;

use super::notifications;
use crate::domain::quick_launch::WizardTabInit;
use crate::event::AppEvent;
use crate::i18n::{self, Key};
use crate::widgets::quick_launch::event::{
    QuickLaunchEffect, QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::quick_launch::services::prepare_quick_launch_setup;
use crate::widgets::quick_launch::storage::save_quick_launches;
use crate::widgets::quick_launch::types::QuickLaunchFile;
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::tabs::types::TabMachine;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Run a launches effect: open the tabs a launch asks for, write the
/// launches, prepare a launch in the background and tell the user about a
/// failure.
pub(crate) fn run(effect: QuickLaunchEffect) -> Task<AppEvent> {
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
        QuickLaunchEffect::Failed(text) => notifications::push(text),
        QuickLaunchEffect::CloseTabRequested { tab_id } => Task::done(
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab { tab_id })),
        ),
        QuickLaunchEffect::PersistRequested { data } => persist(data),
        QuickLaunchEffect::LaunchSetupRequested {
            command,
            path,
            launch_id,
            settings,
            secret,
            cancel,
        } => Task::perform(
            prepare_quick_launch_setup(
                *command, path, launch_id, settings, secret, cancel,
            ),
            |outcome| {
                AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                    QuickLaunchIntent::SetupCompleted(outcome),
                ))
            },
        ),
        // The active pane gets its keys back; in Containers mode its pane
        // is not on screen, so nothing gets them.
        QuickLaunchEffect::OverlayClosed => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive,
            )),
        ),
    }
}

/// Write the launches in the background.
fn persist(data: QuickLaunchFile) -> Task<AppEvent> {
    Task::perform(
        async move { save_quick_launches(&data).map_err(|err| format!("{err}")) },
        persist_finished,
    )
}

fn persist_finished(result: Result<(), String>) -> AppEvent {
    let intent = match result {
        Ok(()) => QuickLaunchIntent::PersistCompleted,
        Err(message) => QuickLaunchIntent::PersistFailed(message),
    };

    AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use otty_ui_term::settings::Settings;

    use super::{persist_finished, run};
    use crate::domain::quick_launch::QuickLaunch;
    use crate::event::AppEvent;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };
    use crate::widgets::quick_launch::event::{
        QuickLaunchEffect, QuickLaunchEvent, QuickLaunchIntent,
    };
    use crate::widgets::quick_launch::types::QuickLaunchSetupOutcome;
    use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
    use crate::widgets::tabs::types::TabMachine;
    use crate::widgets::terminal_workspace::event::{
        TerminalWorkspaceEvent, TerminalWorkspaceIntent,
    };

    fn launch(spec: serde_json::Value) -> QuickLaunch {
        serde_json::from_value(serde_json::json!({
            "title": "htop",
            "spec": spec,
        }))
        .expect("launch should deserialize")
    }

    fn opened_machine(command: QuickLaunch) -> Option<TabMachine> {
        let events =
            collect_outputs(run(QuickLaunchEffect::OpenCommandTerminalTab {
                title: String::from("htop"),
                settings: Settings::default(),
                command: Box::new(command),
            }));

        events.into_iter().find_map(|event| match event {
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenCommandTab {
                machine,
                ..
            })) => Some(machine),
            _ => None,
        })
    }

    #[test]
    fn a_closed_overlay_gives_the_active_terminal_its_keys_back() {
        let events = collect_outputs(run(QuickLaunchEffect::OverlayClosed));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))]
        ));
    }

    #[test]
    fn a_finished_save_reaches_the_widget() {
        assert!(matches!(
            persist_finished(Ok(())),
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::PersistCompleted
            ))
        ));
    }

    #[test]
    fn a_failed_save_reaches_the_widget_with_its_reason() {
        assert!(matches!(
            persist_finished(Err(String::from("disk full"))),
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::PersistFailed(message)
            )) if message == "disk full"
        ));
    }

    #[test]
    fn a_canceled_launch_setup_comes_back_to_the_widget() {
        let command = launch(serde_json::json!({
            "type": "Custom",
            "custom": {
                "program": "htop",
                "args": [],
                "env": [],
                "working_directory": null
            }
        }));

        let events =
            collect_outputs(run(QuickLaunchEffect::LaunchSetupRequested {
                command: Box::new(command),
                path: vec![String::from("htop")],
                launch_id: 7,
                settings: Settings::default(),
                secret: None,
                cancel: Arc::new(AtomicBool::new(true)),
            }));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::SetupCompleted(
                    QuickLaunchSetupOutcome::Canceled { launch_id: 7, .. }
                )
            ))]
        ));
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
        let task = run(QuickLaunchEffect::OpenWizardCreateTab {
            parent_path: vec![String::from("Demo")],
        });
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn a_failed_launch_becomes_a_notification() {
        let events = collect_outputs(run(QuickLaunchEffect::Failed(
            String::from("Could not launch \"htop\": boom"),
        )));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if text == "Could not launch \"htop\": boom"
        ));
    }
}
