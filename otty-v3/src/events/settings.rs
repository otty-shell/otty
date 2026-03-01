use iced::Task;
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use super::AppEvent;
use crate::app::App;
use crate::widgets::settings::model::SettingsData;
use crate::widgets::settings::{
    SettingsEffect, SettingsEvent, SettingsUiEvent,
};
use crate::widgets::terminal_workspace::services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceUiEvent,
};

pub(crate) fn handle(app: &mut App, event: SettingsEvent) -> Task<AppEvent> {
    match event {
        SettingsEvent::Ui(event) => {
            app.widgets.settings.reduce(event).map(AppEvent::Settings)
        },
        SettingsEvent::Effect(effect) => handle_effect(app, effect),
    }
}

fn handle_effect(app: &mut App, effect: SettingsEffect) -> Task<AppEvent> {
    use SettingsEffect::*;

    match effect {
        ReloadLoaded(load) => Task::done(AppEvent::Settings(
            SettingsEvent::Ui(SettingsUiEvent::ReloadLoaded(load)),
        )),
        ReloadFailed(message) => Task::done(AppEvent::Settings(
            SettingsEvent::Ui(SettingsUiEvent::ReloadFailed(message)),
        )),
        SaveFailed(message) => Task::done(AppEvent::Settings(
            SettingsEvent::Ui(SettingsUiEvent::SaveFailed(message)),
        )),
        SaveCompleted(data) => Task::done(AppEvent::Settings(
            SettingsEvent::Ui(SettingsUiEvent::SaveCompleted(data)),
        )),
        ApplyTheme(data) => apply_theme(app, &data),
    }
}

fn apply_theme(app: &mut App, data: &SettingsData) -> Task<AppEvent> {
    app.theme_manager
        .set_custom_palette(data.to_color_palette());
    let current_theme = app.theme_manager.current();
    app.terminal_settings = Settings {
        font: FontSettings {
            size: app.fonts.terminal.size,
            font_type: app.fonts.terminal.font_type,
            ..FontSettings::default()
        },
        theme: ThemeSettings::new(Box::new(
            current_theme.terminal_palette().clone(),
        )),
        backend: BackendSettings::default(),
    };

    let shell_path = data.terminal_shell().to_string();
    app.shell_session = match setup_shell_session_with_shell(&shell_path) {
        Ok(session) => session,
        Err(err) => {
            log::warn!("shell integration setup failed: {err}");
            fallback_shell_session_with_shell(&shell_path)
        },
    };

    let palette = data.to_color_palette();
    let terminal_palette: otty_ui_term::ColorPalette = palette.into();

    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Ui(
        TerminalWorkspaceUiEvent::ApplyTheme {
            palette: Box::new(terminal_palette),
        },
    )))
}
