#[path = "routers/mod.rs"]
pub(crate) mod routers;
#[path = "subscription.rs"]
mod subscription;
#[path = "update.rs"]
mod update;
#[path = "view.rs"]
mod view;

use iced::window::Direction;
use iced::{Element, Size, Subscription, Task, Theme};
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use crate::fonts::FontsConfig;
use crate::state::State;
use crate::theme::{AppTheme, ThemeManager};
use crate::ui::widgets::action_bar;
use crate::widgets::explorer::{ExplorerEffectEvent, ExplorerUiEvent};
use crate::widgets::quick_launch_wizard::{
    QuickLaunchWizardEffectEvent, QuickLaunchWizardUiEvent,
};
use crate::widgets::sidebar::{SidebarEffectEvent, SidebarUiEvent};
use crate::widgets::terminal::{
    ShellSession, TerminalEvent, fallback_shell_session_with_shell,
    setup_shell_session_with_shell,
};
use crate::widgets::{Widgets, quick_launch, settings};

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// App-wide events that drive the root update loop.
#[derive(Clone)]
pub(crate) enum Event {
    IcedReady,
    ActionBar(action_bar::ActionBarEvent),
    SidebarUi(SidebarUiEvent),
    SidebarEffect(SidebarEffectEvent),
    ExplorerUi(ExplorerUiEvent),
    ExplorerEffect(ExplorerEffectEvent),
    QuickLaunch(quick_launch::QuickLaunchEvent),
    ActivateTab {
        tab_id: u64,
    },
    CloseTabRequested {
        tab_id: u64,
    },
    SetTabTitle {
        tab_id: u64,
        title: String,
    },
    OpenCommandTerminalTab {
        title: String,
        settings: Box<Settings>,
    },
    OpenQuickLaunchCommandTerminalTab {
        title: String,
        settings: Box<Settings>,
        command: Box<quick_launch::QuickLaunch>,
    },
    OpenQuickLaunchWizardCreateTab {
        parent_path: quick_launch::NodePath,
    },
    OpenQuickLaunchWizardEditTab {
        path: quick_launch::NodePath,
        command: Box<quick_launch::QuickLaunch>,
    },
    OpenQuickLaunchErrorTab {
        title: String,
        message: String,
    },
    OpenTerminalTab,
    OpenSettingsTab,
    SyncTerminalGridSizes,
    Terminal(TerminalEvent),
    QuickLaunchWizardUi {
        tab_id: u64,
        event: QuickLaunchWizardUiEvent,
    },
    QuickLaunchWizardEffect(QuickLaunchWizardEffectEvent),
    Settings(settings::SettingsEvent),
    SettingsApplied(settings::SettingsData),
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

pub(crate) struct App {
    window_size: Size,
    theme_manager: ThemeManager,
    fonts: FontsConfig,
    terminal_settings: Settings,
    shell_session: ShellSession,
    state: State,
    pub(crate) widgets: Widgets,
    is_fullscreen: bool,
}

impl App {
    pub(crate) fn new() -> (Self, Task<Event>) {
        let settings_state = settings::load_initial_settings_state();
        let mut theme_manager = ThemeManager::new();
        let settings_palette = settings_state.draft().to_color_palette();
        theme_manager.set_custom_palette(settings_palette);
        let current_theme = theme_manager.current();
        let fonts = FontsConfig::default();

        let terminal_settings = terminal_settings(current_theme, &fonts);
        let shell_path = settings_state.draft().terminal_shell().to_string();
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
        let screen_size = view::screen_size_from_window(window_size);
        let state = State::new(window_size, screen_size);
        let widgets = Widgets::new(settings_state);

        let app = App {
            window_size,
            theme_manager,
            fonts,
            terminal_settings,
            shell_session,
            state,
            widgets,
            is_fullscreen: false,
        };

        (app, Task::done(()).map(|_: ()| Event::IcedReady))
    }

    pub(crate) fn title(&self) -> String {
        String::from("OTTY v2")
    }

    pub(crate) fn theme(&self) -> Theme {
        self.theme_manager.iced_theme()
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        subscription::subscription(self)
    }

    pub(crate) fn update(&mut self, event: Event) -> Task<Event> {
        update::update(self, event)
    }

    pub(crate) fn view(&self) -> Element<'_, Event, Theme, iced::Renderer> {
        view::view(self)
    }

    /// Replace terminal settings snapshot.
    pub(crate) fn set_terminal_settings(&mut self, settings: Settings) {
        self.terminal_settings = settings;
    }

    /// Replace active shell session.
    pub(crate) fn set_shell_session(&mut self, session: ShellSession) {
        self.shell_session = session;
    }

    /// Return current screen size for terminal layout calculations.
    pub(crate) fn screen_size(&self) -> Size {
        self.state.screen_size
    }

    /// Apply a new window size and recompute screen size.
    pub(crate) fn set_window_size(&mut self, size: Size) {
        self.window_size = size;
        self.state.window_size = size;
        self.state
            .set_screen_size(view::screen_size_from_window(size));
    }

    /// Return mutable access to theme manager for palette updates.
    pub(crate) fn theme_manager_mut(&mut self) -> &mut ThemeManager {
        &mut self.theme_manager
    }

    /// Return read-only access to theme manager.
    pub(crate) fn theme_manager(&self) -> &ThemeManager {
        &self.theme_manager
    }

    /// Return read-only fonts configuration.
    pub(crate) fn fonts(&self) -> &FontsConfig {
        &self.fonts
    }

    /// Toggle fullscreen state and return target window mode.
    pub(crate) fn toggle_fullscreen_mode(&mut self) -> iced::window::Mode {
        self.is_fullscreen = !self.is_fullscreen;
        if self.is_fullscreen {
            iced::window::Mode::Fullscreen
        } else {
            iced::window::Mode::Windowed
        }
    }
}

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
    }
}
