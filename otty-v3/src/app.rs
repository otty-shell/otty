#[path = "subscription.rs"]
mod subscription;
#[path = "update.rs"]
mod update;
#[path = "view.rs"]
pub(crate) mod view;

use std::collections::VecDeque;

use iced::window::Direction;
use iced::{Element, Size, Subscription, Task, Theme};
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use crate::shared::ui::fonts::FontsConfig;
use crate::shared::ui::theme::{AppTheme, ThemeManager};
use crate::state::State;
use crate::widgets::chrome::{ChromeEvent, ChromeWidget};
use crate::widgets::explorer::{ExplorerEvent, ExplorerWidget};
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchWidget};
use crate::widgets::settings::{SettingsEvent, SettingsWidget};
use crate::widgets::sidebar::{SidebarEvent, SidebarWidget};
use crate::widgets::tabs::{TabsEvent, TabsWidget};
use crate::widgets::terminal_workspace::model::ShellSession;
use crate::widgets::terminal_workspace::services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceWidget,
};

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// A pending quick launch wizard initialization request.
#[derive(Debug, Clone)]
pub(crate) enum PendingQuickLaunchWizard {
    /// Initialize a create wizard after the tab has opened.
    Create { parent_path: Vec<String> },
    /// Initialize an edit wizard after the tab has opened.
    Edit {
        path: Vec<String>,
        command: Box<crate::widgets::quick_launch::model::QuickLaunch>,
    },
}

/// A queue of cross-widget pending workflow continuations.
#[derive(Default)]
pub(crate) struct PendingWorkflows {
    quick_launch_wizards: VecDeque<PendingQuickLaunchWizard>,
    quick_launch_error_tabs: VecDeque<PendingQuickLaunchErrorTab>,
}

/// A pending quick launch error payload.
#[derive(Debug, Clone)]
pub(crate) struct PendingQuickLaunchErrorTab {
    title: String,
    message: String,
}

impl PendingQuickLaunchErrorTab {
    /// Create a pending quick launch error payload.
    pub(crate) fn new(title: String, message: String) -> Self {
        Self { title, message }
    }

    /// Consume payload and return `(title, message)`.
    pub(crate) fn into_parts(self) -> (String, String) {
        (self.title, self.message)
    }
}

impl PendingWorkflows {
    /// Queue quick launch create wizard initialization.
    pub(crate) fn push_quick_launch_wizard_create(
        &mut self,
        parent_path: Vec<String>,
    ) {
        self.quick_launch_wizards
            .push_back(PendingQuickLaunchWizard::Create { parent_path });
    }

    /// Queue quick launch edit wizard initialization.
    pub(crate) fn push_quick_launch_wizard_edit(
        &mut self,
        path: Vec<String>,
        command: Box<crate::widgets::quick_launch::model::QuickLaunch>,
    ) {
        self.quick_launch_wizards
            .push_back(PendingQuickLaunchWizard::Edit { path, command });
    }

    /// Pop the next quick launch wizard initialization continuation.
    pub(crate) fn pop_quick_launch_wizard(
        &mut self,
    ) -> Option<PendingQuickLaunchWizard> {
        self.quick_launch_wizards.pop_front()
    }

    /// Queue quick launch error tab payload.
    pub(crate) fn push_quick_launch_error_tab(
        &mut self,
        title: String,
        message: String,
    ) {
        self.quick_launch_error_tabs
            .push_back(PendingQuickLaunchErrorTab::new(title, message));
    }

    /// Pop the next quick launch error tab payload.
    pub(crate) fn pop_quick_launch_error_tab(
        &mut self,
    ) -> Option<PendingQuickLaunchErrorTab> {
        self.quick_launch_error_tabs.pop_front()
    }
}

/// App-wide events that drive the root update loop.
#[derive(Clone)]
pub(crate) enum AppEvent {
    IcedReady,
    // Sidebar widget
    Sidebar(SidebarEvent),
    // Chrome widget
    Chrome(ChromeEvent),
    // Tabs widget
    Tabs(TabsEvent),
    // Quick Launch widget
    QuickLaunch(QuickLaunchEvent),
    // Terminal Workspace widget
    TerminalWorkspace(TerminalWorkspaceEvent),
    // Explorer widget
    Explorer(ExplorerEvent),
    // Settings widget
    Settings(SettingsEvent),
    // Cross-widget workflows
    OpenTerminalTab,
    OpenSettingsTab,
    OpenFileTerminalTab { file_path: std::path::PathBuf },
    CloseTab { tab_id: u64 },
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
    pub(crate) shell_session: ShellSession,
    pub(crate) state: State,
    pub(crate) pending_workflows: PendingWorkflows,
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
        let screen_size = view::screen_size_from_window(window_size);
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
            pending_workflows: PendingWorkflows::default(),
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
    }
}
