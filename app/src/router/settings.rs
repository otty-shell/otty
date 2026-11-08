use iced::Task;
use otty_ui_term::settings::{
    BackendSettings, BlockSelectionMode, FontSettings, InteractionSettings,
    Settings, ThemeSettings,
};

use super::AppEvent;
use crate::app::App;
use crate::fonts::apply_text_locale;
use crate::i18n::{self, Key};
use crate::widgets::settings::event::{
    SettingsEffect, SettingsEvent, SettingsIntent,
};
use crate::widgets::settings::storage::{load_settings, save_settings};
use crate::widgets::settings::types::{SettingsData, SettingsLoad};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};
use crate::widgets::terminal_workspace::services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
};

/// Route a settings event: reduce its intents and apply saved settings.
pub(crate) fn route(app: &mut App, event: SettingsEvent) -> Task<AppEvent> {
    match event {
        SettingsEvent::Intent(intent) => {
            app.widgets.settings.reduce(intent).map(AppEvent::Settings)
        },
        SettingsEvent::Effect(effect) => handle_effect(app, effect),
    }
}

fn handle_effect(app: &mut App, effect: SettingsEffect) -> Task<AppEvent> {
    match effect {
        SettingsEffect::LoadRequested => load(),
        SettingsEffect::SaveRequested(settings) => save(settings),
        SettingsEffect::ApplyTheme(data) => apply_theme(app, &data),
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
    app.shell_session = match setup_shell_session_with_shell(&shell_path) {
        Ok(session) => session,
        Err(err) => {
            log::warn!("shell integration setup failed: {err}");
            fallback_shell_session_with_shell(&shell_path)
        },
    };

    Task::batch(vec![
        Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::RefreshSettingsTitle {
                title: i18n::t(Key::SettingsTabTitle).to_string(),
            },
        ))),
        Task::done(AppEvent::TerminalWorkspace(
            TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::ApplyTheme {
                    palette: Box::new(terminal_palette),
                },
            ),
        )),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::settings::types::SettingsLoadStatus;

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
