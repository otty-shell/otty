#[path = "subscription.rs"]
mod subscription;
#[path = "update.rs"]
mod update;
#[path = "view.rs"]
pub(crate) mod view;

use iced::window::Direction;
use iced::{Element, Size, Subscription, Task, Theme};
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use crate::shared::ui::fonts::FontsConfig;
use crate::shared::ui::theme::{AppTheme, ThemeManager};
use crate::state::State;
use crate::widgets::chrome::{ChromeEffect, ChromeEvent, ChromeWidget};
use crate::widgets::explorer::{ExplorerEffect, ExplorerEvent, ExplorerWidget};
use crate::widgets::quick_launch::{
    QuickLaunchEffect, QuickLaunchEvent, QuickLaunchWidget,
};
use crate::widgets::settings::SettingsWidget;
use crate::widgets::settings::event::{SettingsEffect, SettingsEvent};
use crate::widgets::sidebar::{SidebarEffect, SidebarEvent, SidebarWidget};
use crate::widgets::tabs::{TabsEffect, TabsEvent, TabsWidget};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEffect, TerminalWorkspaceEvent, TerminalWorkspaceWidget,
};

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// Cross-widget orchestration events.
#[derive(Debug, Clone)]
pub(crate) enum AppFlowEvent {
    OpenTerminalTab,
    OpenSettingsTab,
    OpenQuickLaunchWizardCreateTab {
        parent_path: Vec<String>,
    },
    OpenQuickLaunchWizardEditTab {
        path: Vec<String>,
        command: Box<crate::widgets::quick_launch::model::QuickLaunch>,
    },
    OpenQuickLaunchCommandTerminalTab {
        title: String,
        settings: otty_ui_term::settings::Settings,
        command: Box<crate::widgets::quick_launch::model::QuickLaunch>,
    },
    OpenQuickLaunchErrorTab {
        title: String,
        message: String,
    },
    OpenFileTerminalTab {
        file_path: std::path::PathBuf,
    },
    CloseTab {
        tab_id: u64,
    },
}

/// App-wide events that drive the root update loop.
#[derive(Clone)]
pub(crate) enum AppEvent {
    IcedReady,
    // Sidebar widget
    SidebarUi(SidebarEvent),
    SidebarEffect(SidebarEffect),
    // Chrome widget
    ChromeUi(ChromeEvent),
    ChromeEffect(ChromeEffect),
    // Tabs widget
    TabsUi(TabsEvent),
    TabsEffect(TabsEffect),
    // Quick Launch widget
    QuickLaunchUi(QuickLaunchEvent),
    QuickLaunchEffect(QuickLaunchEffect),
    // Terminal Workspace widget
    TerminalWorkspaceUi(TerminalWorkspaceEvent),
    TerminalWorkspaceEffect(TerminalWorkspaceEffect),
    // Explorer widget
    ExplorerUi(ExplorerEvent),
    ExplorerEffect(ExplorerEffect),
    // Settings widget
    SettingsUi(SettingsEvent),
    SettingsEffect(SettingsEffect),
    // Cross-widget flows
    Flow(AppFlowEvent),
    // Direct operations
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

/// Container for all widget instances.
pub(crate) struct Widgets {
    pub(crate) sidebar: SidebarWidget,
    pub(crate) chrome: ChromeWidget,
    pub(crate) tabs: TabsWidget,
    pub(crate) quick_launch: QuickLaunchWidget,
    pub(crate) terminal_workspace: TerminalWorkspaceWidget,
    pub(crate) explorer: ExplorerWidget,
    pub(crate) settings: SettingsWidget,
}

/// Root application state.
pub(crate) struct App {
    pub(crate) window_size: Size,
    pub(crate) theme_manager: ThemeManager,
    pub(crate) fonts: FontsConfig,
    pub(crate) terminal_settings: Settings,
    pub(crate) state: State,
    pub(crate) widgets: Widgets,
}

impl App {
    /// Initialize the application and return the first task.
    pub(crate) fn new() -> (Self, Task<AppEvent>) {
        let theme_manager = ThemeManager::new();
        let current_theme = theme_manager.current();
        let fonts = FontsConfig::default();
        let terminal_settings = terminal_settings(current_theme, &fonts);

        let window_size = Size {
            width: MIN_WINDOW_WIDTH,
            height: MIN_WINDOW_HEIGHT,
        };
        let screen_size = view::screen_size_from_window(window_size);
        let state = State::new(window_size, screen_size);

        let widgets = Widgets {
            sidebar: SidebarWidget::new(),
            chrome: ChromeWidget::new(),
            tabs: TabsWidget::new(),
            quick_launch: QuickLaunchWidget::load(),
            terminal_workspace: TerminalWorkspaceWidget::new(),
            explorer: ExplorerWidget::new(),
            settings: SettingsWidget::load(),
        };

        let app = App {
            window_size,
            theme_manager,
            fonts,
            terminal_settings,
            state,
            widgets,
        };

        (app, Task::done(()).map(|_: ()| AppEvent::IcedReady))
    }

    /// Return the window title.
    pub(crate) fn title(&self) -> String {
        if let Some(tab_title) = self.widgets.tabs.active_tab_title() {
            format!("{tab_title} — OTTY")
        } else {
            String::from("OTTY")
        }
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
    }
}
