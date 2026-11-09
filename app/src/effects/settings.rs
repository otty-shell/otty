use iced::Task;

use super::notifications;
use crate::event::AppEvent;
use crate::fonts::apply_text_locale;
use crate::shell_integration::{
    integrated_shell_session, shell_integration_dir,
};
use crate::theme::AppTheme;
use crate::widgets::settings::event::{
    SettingsEffect, SettingsEvent, SettingsIntent,
};
use crate::widgets::settings::storage::{load_settings, save_settings};
use crate::widgets::settings::types::{SettingsData, SettingsLoad};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Run a settings effect: read and write the settings file, send saved
/// settings to the app and the terminals, give the keyboard back when the
/// popup closes and tell the user about a failure.
pub(crate) fn run(effect: SettingsEffect) -> Task<AppEvent> {
    match effect {
        SettingsEffect::LoadRequested => load(),
        SettingsEffect::SaveRequested(settings) => save(settings),
        SettingsEffect::ApplyTheme(data) => apply_theme(&data),
        // The popup had the keyboard. In Terminal mode the active pane gets
        // it back; in Containers mode no pane is on screen to take it.
        SettingsEffect::Closed => Task::done(AppEvent::TerminalWorkspace(
            TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive,
            ),
        )),
        SettingsEffect::Failed(text) => notifications::push(text),
    }
}

/// Read the settings in the background.
fn load() -> Task<AppEvent> {
    Task::perform(
        async { load_settings().map_err(|err| format!("{err}")) },
        load_finished,
    )
}

fn load_finished(result: Result<SettingsLoad, String>) -> AppEvent {
    let intent = match result {
        Ok(load) => SettingsIntent::ReloadLoaded(load),
        Err(message) => SettingsIntent::ReloadFailed(message),
    };

    AppEvent::Settings(SettingsEvent::Intent(intent))
}

/// Write the settings in the background.
fn save(settings: SettingsData) -> Task<AppEvent> {
    Task::perform(
        async move {
            match save_settings(&settings) {
                Ok(()) => Ok(settings),
                Err(err) => Err(format!("{err}")),
            }
        },
        save_finished,
    )
}

fn save_finished(result: Result<SettingsData, String>) -> AppEvent {
    let intent = match result {
        Ok(settings) => SettingsIntent::SaveCompleted(settings),
        Err(message) => SettingsIntent::SaveFailed(message),
    };

    AppEvent::Settings(SettingsEvent::Intent(intent))
}

/// Write the shell integration files of the saved shell, then send the
/// theme and the shell session to the app before the open terminals get the
/// new colors, so terminals opened after that match them.
fn apply_theme(data: &SettingsData) -> Task<AppEvent> {
    apply_text_locale();
    let shell_session = integrated_shell_session(
        data.terminal_shell(),
        &shell_integration_dir(),
    );
    let terminal_palette = AppTheme::new(data.theme()).terminal_palette();

    Task::done(AppEvent::ThemeChosen(data.theme()))
        .chain(Task::done(AppEvent::ShellIntegrated(shell_session)))
        .chain(Task::done(AppEvent::TerminalWorkspace(
            TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::ApplyTheme {
                    palette: Box::new(terminal_palette),
                },
            ),
        )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::theme::BuiltinTheme;
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };
    use crate::widgets::settings::types::SettingsLoadStatus;

    #[test]
    fn a_failure_becomes_a_notification() {
        let events = collect_outputs(run(SettingsEffect::Failed(
            String::from("Could not save the settings: disk full"),
        )));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if text == "Could not save the settings: disk full"
        ));
    }

    /// Owner, Q1 variant A: when the popup closes, the active pane gets
    /// its keys back; in Containers mode no pane is on screen to take them.
    #[test]
    fn a_closed_popup_gives_the_keyboard_back_to_the_active_pane() {
        let events = collect_outputs(run(SettingsEffect::Closed));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))]
        ));
    }

    /// Settings is no tab any more, so applied settings reach only the app
    /// and the terminals. The app gets them first: new terminals start with
    /// its terminal settings and shell session.
    #[test]
    fn applied_settings_reach_the_app_then_the_terminals() {
        let mut saved = SettingsData::default();
        saved.set_theme(BuiltinTheme::OttyLight);
        // A shell without OTTY shell integration, so no files are written.
        saved
            .set_terminal_shell(String::from("/opt/otty-test/otty-test-shell"));

        let events = collect_outputs(run(SettingsEffect::ApplyTheme(saved)));

        assert!(matches!(
            events.as_slice(),
            [
                AppEvent::ThemeChosen(BuiltinTheme::OttyLight),
                AppEvent::ShellIntegrated(session),
                AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::ApplyTheme { .. }
                )),
            ] if session.name() == "otty-test-shell"
        ));
    }

    #[test]
    fn a_finished_read_reaches_the_widget() {
        let load = SettingsLoad::new(
            SettingsData::default(),
            SettingsLoadStatus::Loaded,
        );

        assert!(matches!(
            load_finished(Ok(load)),
            AppEvent::Settings(SettingsEvent::Intent(
                SettingsIntent::ReloadLoaded(_)
            ))
        ));
    }

    #[test]
    fn a_failed_read_reaches_the_widget_with_its_reason() {
        assert!(matches!(
            load_finished(Err(String::from("no access"))),
            AppEvent::Settings(SettingsEvent::Intent(
                SettingsIntent::ReloadFailed(message)
            )) if message == "no access"
        ));
    }

    #[test]
    fn a_finished_write_reaches_the_widget_with_what_was_written() {
        let written = SettingsData::default();

        assert!(matches!(
            save_finished(Ok(written.clone())),
            AppEvent::Settings(SettingsEvent::Intent(
                SettingsIntent::SaveCompleted(settings)
            )) if settings == written
        ));
    }

    #[test]
    fn a_failed_write_reaches_the_widget_with_its_reason() {
        assert!(matches!(
            save_finished(Err(String::from("disk full"))),
            AppEvent::Settings(SettingsEvent::Intent(
                SettingsIntent::SaveFailed(message)
            )) if message == "disk full"
        ));
    }
}
