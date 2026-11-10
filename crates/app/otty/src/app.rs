use iced::{Element, Subscription, Task, Theme};
use otty_app_domain::mode::Mode;
use otty_app_theme::theme::{BuiltinTheme, ThemeManager};

use crate::chrome::ChromeWidget;
use crate::message::{AppEvent, Message};
use crate::modes::ModesWidget;
use crate::shell_session::ShellSession;
use crate::sidebar::SidebarWidget;
use crate::{effects, subscription, update, view};

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// Root state of the window: the frame widgets and what the frame read at
/// start. A feature gets its field here when its issue moves it in.
pub(crate) struct App {
    pub(crate) theme_manager: ThemeManager,
    /// The session new terminal tabs start.
    pub(crate) shell_session: ShellSession,
    /// This computer's host name, read once at start; `None` before that or
    /// when it could not be read.
    pub(crate) host_name: Option<String>,
    pub(crate) chrome: ChromeWidget,
    pub(crate) sidebar: SidebarWidget,
    pub(crate) modes: ModesWidget,
}

impl App {
    /// Read what the frame needs from disk, build the window state and
    /// return the startup work: the shell integration files and the host
    /// name.
    pub(crate) fn new() -> (Self, Task<Message>) {
        let saved = effects::settings::read_saved();
        // The language must be active before any text is built.
        effects::locale::apply(saved.language());
        // A modes file that could not be read fell back to the defaults, and
        // the next pin writes over it. The notifications feature (issue 06)
        // tells the user once the window is ready.
        let (pinned_modes, _modes_problem) = effects::modes::load_pinned();

        let app = Self::with_saved(saved.theme(), saved.shell(), &pinned_modes);
        let first = effects::startup::run(saved.shell().to_string())
            .map(Message::Internal);

        (app, first)
    }

    /// Build the window state from what was read at start: the theme, the
    /// shell new terminals start and the pinned modes. It reads nothing from
    /// disk.
    pub(crate) fn with_saved(
        theme: BuiltinTheme,
        shell: &str,
        pinned_modes: &[Mode],
    ) -> Self {
        let mut theme_manager = ThemeManager::new();
        theme_manager.set_theme(theme);

        Self {
            theme_manager,
            // The startup work writes the shell integration files and
            // replaces this session before the first tab can open.
            shell_session: ShellSession::without_integration(shell),
            host_name: None,
            chrome: ChromeWidget::new(),
            sidebar: SidebarWidget::new(),
            modes: ModesWidget::with_pinned(pinned_modes),
        }
    }

    /// Return the window title.
    pub(crate) fn title(&self) -> String {
        String::from("OTTY")
    }

    /// Return the current iced theme.
    pub(crate) fn theme(&self) -> Theme {
        self.theme_manager.iced_theme()
    }

    /// Return the active subscriptions. A key press is input, so it goes
    /// through the input checks.
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

    /// Render the window. What the user does in it is input.
    pub(crate) fn view(&self) -> Element<'_, Message> {
        view::view(self).map(Message::Input)
    }
}
