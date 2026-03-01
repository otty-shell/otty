use iced::Task;
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use crate::app::{App, AppEvent};
use crate::widgets::settings::event::SettingsEvent;
use crate::widgets::settings::model::SettingsData;
use crate::widgets::settings::{SettingsCommand, SettingsEffect};
use crate::widgets::terminal_workspace::TerminalWorkspaceCommand;
use crate::widgets::terminal_workspace::services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
};

/// Route a settings UI event through the widget reducer.
pub(crate) fn route_event(
    app: &mut App,
    event: SettingsEvent,
) -> Task<AppEvent> {
    let command = map_event_to_command(event);
    route_command(app, command)
}

/// Route a settings command directly (used by flow routers).
pub(crate) fn route_command(
    app: &mut App,
    command: SettingsCommand,
) -> Task<AppEvent> {
    app.widgets
        .settings
        .reduce(command)
        .map(AppEvent::SettingsEffect)
}

/// Route a settings effect event to app-level tasks.
pub(crate) fn route_effect(
    app: &mut App,
    effect: SettingsEffect,
) -> Task<AppEvent> {
    use SettingsEffect::*;

    match effect {
        ReloadLoaded(load) => {
            Task::done(AppEvent::SettingsUi(SettingsEvent::ReloadLoaded(load)))
        },
        ReloadFailed(message) => Task::done(AppEvent::SettingsUi(
            SettingsEvent::ReloadFailed(message),
        )),
        SaveFailed(message) => {
            Task::done(AppEvent::SettingsUi(SettingsEvent::SaveFailed(message)))
        },
        SaveCompleted(data) => {
            Task::done(AppEvent::SettingsUi(SettingsEvent::SaveCompleted(data)))
        },
        ApplyTheme(data) => apply_theme(app, &data),
    }
}

/// Apply settings to app theme/runtime and propagate palette to terminals.
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

    Task::done(AppEvent::TerminalWorkspaceCommand(
        TerminalWorkspaceCommand::ApplyTheme {
            palette: Box::new(terminal_palette),
        },
    ))
}

fn map_event_to_command(event: SettingsEvent) -> SettingsCommand {
    use {SettingsCommand as C, SettingsEvent as E};

    match event {
        E::Reload => C::Reload,
        E::ReloadLoaded(load) => C::ReloadLoaded(load),
        E::ReloadFailed(msg) => C::ReloadFailed(msg),
        E::Save => C::Save,
        E::SaveCompleted(data) => C::SaveCompleted(data),
        E::SaveFailed(msg) => C::SaveFailed(msg),
        E::Reset => C::Reset,
        E::NodePressed { path } => C::NodePressed { path },
        E::NodeHovered { path } => C::NodeHovered { path },
        E::ShellChanged(value) => C::ShellChanged(value),
        E::EditorChanged(value) => C::EditorChanged(value),
        E::PaletteChanged { index, value } => {
            C::PaletteChanged { index, value }
        },
        E::ApplyPreset(preset) => C::ApplyPreset(preset),
    }
}
