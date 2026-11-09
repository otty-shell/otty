use iced::Task;
use otty_ui_term::settings::{
    BackendSettings, BlockSelectionMode, FontSettings, InteractionSettings,
    Settings, ThemeSettings,
};

use super::notifications;
use crate::app::App;
use crate::dispatch::AppEvent;
use crate::fonts::apply_text_locale;
use crate::shell_integration::{
    integrated_shell_session, shell_integration_dir,
};
use crate::widgets::settings::event::{
    SettingsEffect, SettingsEvent, SettingsIntent,
};
use crate::widgets::settings::storage::{load_settings, save_settings};
use crate::widgets::settings::types::{SettingsData, SettingsLoad};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Run a settings effect: read and write the settings file, apply saved
/// settings to the app, give the keyboard back when the popup closes and
/// tell the user about a failure.
pub(crate) fn run(app: &mut App, effect: SettingsEffect) -> Task<AppEvent> {
    match effect {
        SettingsEffect::LoadRequested => load(),
        SettingsEffect::SaveRequested(settings) => save(settings),
        SettingsEffect::ApplyTheme(data) => apply_theme(app, &data),
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

fn apply_theme(app: &mut App, data: &SettingsData) -> Task<AppEvent> {
    apply_text_locale();
    app.theme_manager.set_theme(data.theme());
    let current_theme = app.theme_manager.current();
    let terminal_palette = current_theme.terminal_palette();

    app.terminal_settings = Settings {
        font: FontSettings {
            size: app.fonts.terminal.size,
            font_type: app.fonts.terminal.font_type,
            ..FontSettings::default()
        },
        theme: ThemeSettings::new(Box::new(terminal_palette.clone())),
        backend: BackendSettings::default(),
        interaction: InteractionSettings::default()
            .with_block_selection_mode(BlockSelectionMode::CommandOnly),
    };

    let shell_path = data.terminal_shell().to_string();
    app.shell_session =
        integrated_shell_session(&shell_path, &shell_integration_dir());

    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
        TerminalWorkspaceIntent::ApplyTheme {
            palette: Box::new(terminal_palette),
        },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };
    use crate::widgets::settings::types::SettingsLoadStatus;

    #[test]
    fn a_failure_becomes_a_notification() {
        let (mut app, _) = App::new();

        let events = collect_outputs(run(
            &mut app,
            SettingsEffect::Failed(String::from(
                "Could not save the settings: disk full",
            )),
        ));

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
        let (mut app, _) = App::new();

        let events = collect_outputs(run(&mut app, SettingsEffect::Closed));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))]
        ));
    }

    /// Settings is no tab any more, so applying them only reaches the
    /// terminals.
    #[test]
    fn applied_settings_reach_only_the_terminals() {
        let (mut app, _) = App::new();

        let events = collect_outputs(run(
            &mut app,
            SettingsEffect::ApplyTheme(SettingsData::default()),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::ApplyTheme { .. }
            ))]
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
