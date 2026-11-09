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

use crate::event::AppEvent;
use crate::fonts::{FontsConfig, apply_text_locale};
use crate::layout;
use crate::shell_session::ShellSession;
use crate::state::State;
use crate::theme::{AppTheme, ThemeManager};
use crate::widgets::Widgets;
use crate::widgets::chrome::ChromeWidget;
use crate::widgets::containers::ContainersWidget;
use crate::widgets::explorer::ExplorerWidget;
use crate::widgets::modes::ModesWidget;
use crate::widgets::notifications::NotificationsWidget;
use crate::widgets::notifications::event::{
    NotificationsEvent, NotificationsIntent,
};
use crate::widgets::palette::PaletteWidget;
use crate::widgets::quick_launch::QuickLaunchWidget;
use crate::widgets::settings::SettingsWidget;
use crate::widgets::sidebar::SidebarWidget;
use crate::widgets::tabs::TabsWidget;
use crate::widgets::terminal_workspace::TerminalWorkspaceWidget;

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// What iced delivers to the app: an event and where it came from.
///
/// Only this file marks events. Widgets, delivery and effects return plain
/// [`AppEvent`]s, and whatever they return comes back as
/// [`Message::Internal`].
#[derive(Clone)]
pub(crate) enum Message {
    /// A press or typed text in the interface, or a key press. The input
    /// checks in `update.rs` decide what happens to it.
    Input(AppEvent),
    /// An effect, a result of background work or a subscription event. It
    /// follows from input that already passed the checks, or arrives on its
    /// own, so nothing stops it.
    Internal(AppEvent),
}

/// Root application state.
pub(crate) struct App {
    pub(crate) window_size: Size,
    pub(crate) theme_manager: ThemeManager,
    pub(crate) fonts: FontsConfig,
    pub(crate) terminal_settings: Settings,
    pub(crate) shell_session: ShellSession,
    /// This computer's host name, read once when the window is ready; `None`
    /// before that or when it could not be read.
    pub(crate) host_name: Option<String>,
    pub(crate) state: State,
    pub(crate) widgets: Widgets,
}

impl App {
    /// Initialize the application and return the first task.
    pub(crate) fn new() -> (Self, Task<Message>) {
        let settings = SettingsWidget::load();
        apply_text_locale();
        let mut theme_manager = ThemeManager::new();
        let initial_settings = settings.saved().clone();
        theme_manager.set_theme(initial_settings.theme());
        let current_theme = theme_manager.current();
        let fonts = FontsConfig::default();
        let terminal_settings = terminal_settings(current_theme, &fonts);
        // The shell integration files and the host name are outside the
        // app, so the startup effects of `IcedReady` prepare them before the
        // first tab.
        let shell_session = ShellSession::without_integration(
            initial_settings.terminal_shell(),
        );

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
            palette: PaletteWidget::new(),
            modes: ModesWidget::load(),
            containers: ContainersWidget::new(),
            notifications: NotificationsWidget::new(),
        };

        let app = App {
            window_size,
            theme_manager,
            fonts,
            terminal_settings,
            shell_session,
            host_name: None,
            state,
            widgets,
        };

        // A file that could not be read at start fell back to the defaults,
        // and the next edit writes over it, so the user is told once the
        // window is ready.
        let load_problems: Vec<Task<Message>> = [
            app.widgets.settings.load_problem(),
            app.widgets.quick_launch.load_problem(),
            app.widgets.modes.load_problem(),
        ]
        .into_iter()
        .flatten()
        .map(|text| {
            Task::done(Message::Internal(AppEvent::Notifications(
                NotificationsEvent::Intent(NotificationsIntent::Push(
                    String::from(text),
                )),
            )))
        })
        .collect();
        let first = Task::done(Message::Internal(AppEvent::IcedReady))
            .chain(Task::batch(load_problems));

        (app, first)
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
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            subscription::subscription(self).map(Message::Internal),
            iced::keyboard::listen()
                .map(|event| Message::Input(AppEvent::Keyboard(event))),
        ])
    }

    /// Handle an incoming message. Everything it leads to is internal.
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        let task = match message {
            Message::Input(event) => update::update_input(self, event),
            Message::Internal(event) => update::update_internal(self, event),
        };

        task.map(Message::Internal)
    }

    /// Render the root view.
    pub(crate) fn view(&self) -> Element<'_, Message> {
        view::view(self).map(Message::Input)
    }
}

/// Build the settings new terminals start with from the theme and the
/// fonts.
pub(crate) fn terminal_settings(
    theme: &AppTheme,
    fonts: &FontsConfig,
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
        interaction: InteractionSettings::default()
            .with_block_selection_mode(BlockSelectionMode::CommandOnly),
    }
}
