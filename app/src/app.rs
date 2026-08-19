#[path = "subscription.rs"]
mod subscription;
#[path = "update.rs"]
mod update;
#[path = "view.rs"]
pub(crate) mod view;

use iced::{Element, Size, Subscription, Task, Theme};
use otty_ui_term::settings::{
    BackendSettings, BlockSelectionMode, FontSettings, InteractionSettings,
    Settings, ThemeSettings,
};

use crate::events::AppEvent;
use crate::fonts::FontsConfig;
use crate::layout;
use crate::state::State;
use crate::theme::{AppTheme, ThemeManager};
use crate::widgets::Widgets;
use crate::widgets::chrome::ChromeWidget;
use crate::widgets::explorer::ExplorerWidget;
use crate::widgets::quick_launch::QuickLaunchWidget;
use crate::widgets::settings::SettingsWidget;
use crate::widgets::sidebar::SidebarWidget;
use crate::widgets::tabs::TabsWidget;
use crate::widgets::terminal_workspace::TerminalWorkspaceWidget;
use crate::widgets::terminal_workspace::services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
};
use crate::widgets::terminal_workspace::types::ShellSession;

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// Root application state.
pub(crate) struct App {
    pub(crate) window_size: Size,
    pub(crate) theme_manager: ThemeManager,
    pub(crate) fonts: FontsConfig,
    pub(crate) terminal_settings: Settings,
    pub(crate) shell_session: ShellSession,
    pub(crate) state: State,
    pub(crate) widgets: Widgets,
}

impl App {
    /// Initialize the application and return the first task.
    pub(crate) fn new() -> (Self, Task<AppEvent>) {
        let settings = SettingsWidget::load();
        let mut theme_manager = ThemeManager::new();
        let initial_settings = settings.settings_data().clone();
        theme_manager.set_custom_palette(initial_settings.to_color_palette());
        let current_theme = theme_manager.current();
        let fonts = FontsConfig::default();
        let terminal_settings = terminal_settings(current_theme, &fonts);
        let shell_path = initial_settings.terminal_shell().to_string();
        let shell_session = match setup_shell_session_with_shell(&shell_path) {
            Ok(session) => session,
            Err(err) => {
                log::warn!("shell integration setup failed: {err}");
                fallback_shell_session_with_shell(&shell_path)
            },
        };

        let window_size = Size {
            width: MIN_WINDOW_WIDTH,
            height: MIN_WINDOW_HEIGHT,
        };
        let screen_size = layout::screen_size_from_window(window_size);
        let state = State::new(window_size, screen_size);

        let widgets = Widgets {
            sidebar: SidebarWidget::new(),
            chrome: ChromeWidget::new(),
            tabs: TabsWidget::new(),
            quick_launch: QuickLaunchWidget::load(),
            terminal_workspace: TerminalWorkspaceWidget::new(),
            explorer: ExplorerWidget::new(),
            settings,
        };

        let app = App {
            window_size,
            theme_manager,
            fonts,
            terminal_settings,
            shell_session,
            state,
            widgets,
        };

        (app, Task::done(()).map(|_: ()| AppEvent::IcedReady))
    }

    /// Return the window title.
    pub(crate) fn title(&self) -> String {
        String::from("OTTY")
    }

    /// Return the current iced theme.
    pub(crate) fn theme(&self) -> Theme {
        self.theme_manager.iced_theme()
    }

    /// Return active subscriptions.
    pub(crate) fn subscription(&self) -> Subscription<AppEvent> {
        subscription::subscription(self)
    }

    /// Handle an incoming event.
    pub(crate) fn update(&mut self, event: AppEvent) -> Task<AppEvent> {
        update::update(self, event)
    }

    /// Render the root view.
    pub(crate) fn view(&self) -> Element<'_, AppEvent, Theme, iced::Renderer> {
        view::view(self)
    }
}

/// Build terminal widget settings from theme and font config.
fn terminal_settings(theme: &AppTheme, fonts: &FontsConfig) -> Settings {
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
        interaction: InteractionSettings::default()
            .with_block_selection_mode(BlockSelectionMode::CommandOnly),
    }
}
