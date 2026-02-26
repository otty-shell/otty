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

use crate::features::explorer::ExplorerEvent;
use crate::features::quick_launch_wizard::QuickLaunchWizardEvent;
use crate::features::terminal::{
    ShellSession, TerminalEvent, fallback_shell_session_with_shell,
    setup_shell_session_with_shell,
};
use crate::features::{Features, quick_launch, settings};
use crate::fonts::FontsConfig;
use crate::state::State;
use crate::theme::{AppTheme, ThemeManager};
use crate::ui::widgets::{action_bar, sidebar_menu, sidebar_workspace};

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// App-wide events that drive the root update loop.
#[derive(Clone)]
pub(crate) enum Event {
    IcedReady,
    ActionBar(action_bar::ActionBarEvent),
    Sidebar(sidebar_menu::SidebarMenuEvent),
    SidebarWorkspace(sidebar_workspace::SidebarWorkspaceEvent),
    Explorer(ExplorerEvent),
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
    QuickLaunchWizard {
        tab_id: u64,
        event: QuickLaunchWizardEvent,
    },
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
    features: Features,
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
        let features = Features::new(settings_state);

        let app = App {
            window_size,
            theme_manager,
            fonts,
            terminal_settings,
            shell_session,
            state,
            features,
            is_fullscreen: false,
        };

        (app, Task::done(()).map(|_: ()| Event::IcedReady))
    }

    pub(crate) fn title(&self) -> String {
        String::from("OTTY")
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
