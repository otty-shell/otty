use iced::Task;
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use super::runtime;
use crate::app::{App, Event as AppEvent};
use crate::theme::AppTheme;
use crate::widgets::{settings, terminal};

/// Route settings UI event into widget reduction.
pub(crate) fn route_event(
    app: &mut App,
    event: settings::SettingsEvent,
) -> Task<AppEvent> {
    app.widgets.settings_mut().reduce(event, &())
}

/// Route applied settings into app runtime and terminal theme update.
pub(crate) fn route_applied(
    app: &mut App,
    settings: &settings::SettingsData,
) -> Task<AppEvent> {
    let palette = settings.to_color_palette();
    app.theme_manager_mut().set_custom_palette(palette);
    let current_theme = app.theme_manager().current().clone();
    app.set_terminal_settings(build_terminal_settings(
        &current_theme,
        app.fonts(),
    ));
    let terminal_palette = current_theme.terminal_palette().clone();

    match terminal::setup_shell_session_with_shell(settings.terminal_shell()) {
        Ok(session) => app.set_shell_session(session),
        Err(err) => {
            log::warn!("shell integration setup failed: {err}");
            app.set_shell_session(terminal::fallback_shell_session_with_shell(
                settings.terminal_shell(),
            ));
        },
    }

    let ctx = runtime::make_terminal_ctx(app);
    app.widgets.terminal_mut().reduce(
        terminal::TerminalEvent::ApplyTheme {
            palette: Box::new(terminal_palette),
        },
        &ctx,
    )
}

fn build_terminal_settings(
    theme: &AppTheme,
    fonts: &crate::fonts::FontsConfig,
) -> Settings {
    let font_settings = FontSettings {
        size: fonts.terminal.size,
        font_type: fonts.terminal.font_type,
        ..FontSettings::default()
    };
    let theme_settings =
        ThemeSettings::new(Box::new(theme.terminal_palette().clone()));

    Settings {
        font: font_settings,
        theme: theme_settings,
        backend: BackendSettings::default(),
    }
}
