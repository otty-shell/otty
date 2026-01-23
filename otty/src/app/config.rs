use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use crate::app::fonts::FontsConfig;
use crate::app::theme::AppTheme;
use crate::services::ShellSession;

/// App-owned configuration shared with screens.
#[derive(Clone)]
pub(crate) struct AppConfig {
    pub(crate) shell_name: String,
    pub(crate) terminal_settings: Settings,
}

impl AppConfig {
    pub(crate) fn new(
        shell: ShellSession,
        theme: &AppTheme,
        fonts: &FontsConfig,
    ) -> Self {
        let font_settings = FontSettings {
            size: fonts.terminal.size,
            font_type: fonts.terminal.font_type,
            ..FontSettings::default()
        };
        let theme_settings =
            ThemeSettings::new(Box::new(theme.terminal_palette().clone()));

        let settings = Settings {
            font: font_settings,
            theme: theme_settings,
            backend: BackendSettings::default().with_session(shell.session),
        };

        Self {
            shell_name: shell.name,
            terminal_settings: settings,
        }
    }
}
