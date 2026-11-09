//! Runs the effects of the command palette: turns a chosen entry into the
//! event the owning widget already handles for the same button.

use iced::Task;

use crate::commands::AppCommand;
use crate::domain::mode::Mode;
use crate::event::AppEvent;
use crate::widgets::palette::event::PaletteEffect;
use crate::widgets::palette::types::PaletteTarget;
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Run a palette effect: run the chosen entry, or give the keyboard back
/// when the palette closes without one.
pub(crate) fn run(effect: PaletteEffect) -> Task<AppEvent> {
    match effect {
        PaletteEffect::Run(target) => run_target(target),
        // The palette input had keyboard focus; give it back to the
        // terminal the user was in.
        PaletteEffect::Dismissed => focus_active_terminal(),
    }
}

/// Send the event the owning widget's own button sends for `target`.
fn run_target(target: PaletteTarget) -> Task<AppEvent> {
    match target {
        PaletteTarget::Session { tab_id } => Task::done(AppEvent::Tabs(
            TabsEvent::Intent(TabsIntent::ActivateTab { tab_id }),
        )),
        // The same single message the New session menu sends for a launch.
        // The launch is still starting, so the active terminal gets back
        // the keyboard focus the palette input took.
        PaletteTarget::Launch { path } => Task::batch(vec![
            Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::LaunchNode { path },
            ))),
            focus_active_terminal(),
        ]),
        PaletteTarget::Command(command) => run_command(command),
    }
}

/// Ask the app to run `command`. The commands that do not move keyboard
/// focus themselves (the panel toggles, and Terminal mode, whose terminal
/// comes back on screen) also give it back to the active terminal, which
/// lost it to the palette input.
fn run_command(command: AppCommand) -> Task<AppEvent> {
    let run = Task::done(AppEvent::Command(command));

    match command {
        AppCommand::ToggleLeftPanel
        | AppCommand::ToggleFilesPanel
        | AppCommand::ToggleFocusMode
        | AppCommand::SwitchMode(Mode::Terminal) => {
            Task::batch(vec![run, focus_active_terminal()])
        },
        _ => run,
    }
}

fn focus_active_terminal() -> Task<AppEvent> {
    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
        TerminalWorkspaceIntent::FocusActive,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    fn chosen(target: PaletteTarget) -> Vec<AppEvent> {
        collect_outputs(run(PaletteEffect::Run(target)))
    }

    fn is_focus_active(event: &AppEvent) -> bool {
        matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))
        )
    }

    #[test]
    fn a_session_switches_to_its_tab() {
        let events = chosen(PaletteTarget::Session { tab_id: 7 });

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(TabsIntent::ActivateTab {
                tab_id: 7
            }))]
        ));
    }

    #[test]
    fn a_launch_runs_with_one_intent_and_the_terminal_gets_focus() {
        let path = vec![String::from("servers"), String::from("laptop")];

        let events = chosen(PaletteTarget::Launch { path: path.clone() });

        let launches = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                        QuickLaunchIntent::LaunchNode { path: launched }
                    )) if *launched == path
                )
            })
            .count();
        assert_eq!(launches, 1);
        assert!(events.iter().any(is_focus_active));
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn closing_without_running_gives_the_terminal_its_focus_back() {
        let events = collect_outputs(run(PaletteEffect::Dismissed));

        assert!(matches!(events.as_slice(), [event] if is_focus_active(event)));
    }

    #[test]
    fn a_command_entry_asks_the_app_to_run_its_command() {
        for command in [
            AppCommand::NewTerminal,
            AppCommand::NewLaunch,
            AppCommand::NewFolder,
            AppCommand::OpenSettings,
            AppCommand::SplitRight,
            AppCommand::SplitDown,
            AppCommand::CloseSession,
            AppCommand::SwitchMode(Mode::Containers),
        ] {
            let events = chosen(PaletteTarget::Command(command));

            assert!(
                matches!(
                    events.as_slice(),
                    [AppEvent::Command(asked)] if *asked == command
                ),
                "{command:?}"
            );
        }
    }

    #[test]
    fn panel_toggle_entries_run_their_command_and_give_the_terminal_focus() {
        for command in [
            AppCommand::ToggleLeftPanel,
            AppCommand::ToggleFilesPanel,
            AppCommand::ToggleFocusMode,
        ] {
            let events = chosen(PaletteTarget::Command(command));

            assert!(
                events.iter().any(|event| matches!(
                    event,
                    AppEvent::Command(asked) if *asked == command
                )),
                "{command:?}"
            );
            assert!(events.iter().any(is_focus_active), "{command:?}");
            assert_eq!(events.len(), 2, "{command:?}");
        }
    }

    #[test]
    fn the_terminal_mode_entry_switches_and_gives_the_terminal_its_focus_back()
    {
        let events = chosen(PaletteTarget::Command(AppCommand::SwitchMode(
            Mode::Terminal,
        )));

        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Command(AppCommand::SwitchMode(Mode::Terminal))
        )));
        assert!(events.iter().any(is_focus_active));
        assert_eq!(events.len(), 2);
    }
}
