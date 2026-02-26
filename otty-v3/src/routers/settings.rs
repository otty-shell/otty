use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::settings::event::SettingsEvent;
use crate::widgets::settings::{SettingsCommand, SettingsEffect};

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
    match effect {
        SettingsEffect::ApplyTheme(data) => apply_theme(app, &data),
        SettingsEffect::SaveCompleted(data) => apply_theme(app, &data),
    }
}

/// Apply theme palette changes from settings to the terminal workspace.
fn apply_theme(
    app: &mut App,
    data: &crate::widgets::settings::model::SettingsData,
) -> Task<AppEvent> {
    let palette = data.to_color_palette();
    let terminal_palette: otty_ui_term::ColorPalette = palette.into();

    crate::routers::terminal_workspace::route_command(
        app,
        crate::widgets::terminal_workspace::TerminalWorkspaceCommand::ApplyTheme {
            palette: Box::new(terminal_palette),
        },
    )
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
