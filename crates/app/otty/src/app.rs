use iced::{Element, Subscription, Task, Theme};
use otty_app_domain::mode::Mode;
use otty_app_explorer::Explorer;
use otty_app_notifications::{Notifications, NotificationsIntent};
use otty_app_palette::Palette;
use otty_app_theme::theme::{BuiltinTheme, ThemeManager};

use crate::chrome::ChromeWidget;
use crate::message::{AppEvent, Message};
use crate::modes::ModesWidget;
use crate::shell_session::ShellSession;
use crate::sidebar::SidebarWidget;
use crate::{dispatch, effects, subscription, update, view};

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
    pub(crate) notifications: Notifications,
    pub(crate) explorer: Explorer,
    pub(crate) palette: Palette,
}

impl App {
    /// Read what the frame needs from disk, build the window state and
    /// return the startup work: the shell integration files and the host
    /// name.
    pub(crate) fn new() -> (Self, Task<Message>) {
        let (saved, settings_problem) = effects::settings::read_saved();
        // The language must be active before any text is built, the texts
        // of the problems too.
        effects::locale::apply(saved.language());
        let (pinned_modes, modes_problem) = effects::modes::load_pinned();

        let mut app =
            Self::with_saved(saved.theme(), saved.shell(), &pinned_modes);
        let problems = settings_problem
            .map(|problem| problem.text())
            .into_iter()
            .chain(modes_problem);
        let told = app.tell_start_problems(problems);
        let first = effects::startup::run(saved.shell().to_string())
            .map(Message::Internal);

        (app, Task::batch([told, first]))
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
            notifications: Notifications::new(),
            explorer: Explorer::new(),
            palette: Palette::new(),
        }
    }

    /// Tell the user about each file that could not be read at start: it
    /// fell back to the defaults, and the next save writes over it. Each
    /// text, already in the user's language, shows as a toast.
    pub(crate) fn tell_start_problems(
        &mut self,
        problems: impl IntoIterator<Item = String>,
    ) -> Task<Message> {
        let told: Vec<Task<AppEvent>> = problems
            .into_iter()
            .map(|text| {
                dispatch::notifications::deliver(
                    self,
                    NotificationsIntent::Push(text),
                )
            })
            .collect();

        Task::batch(told).map(Message::Internal)
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

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::testing::window::app;

    /// A settings or modes file that could not be read at start fell back
    /// to the defaults: the user is told with a toast, which counts on the
    /// bell.
    #[test]
    fn each_start_problem_shows_as_a_toast() {
        let mut app = app();

        let _none = app.tell_start_problems([
            String::from("Could not read the settings"),
            String::from("Could not read the pinned modes"),
        ]);

        let mut ui = simulator(view::view(&app));
        assert_eq!(app.notifications.unseen_count(), 2);
        assert!(ui.find("Could not read the settings").is_ok());
        assert!(ui.find("Could not read the pinned modes").is_ok());
    }

    #[test]
    fn a_start_without_problems_tells_nothing() {
        let mut app = app();

        let _none = app.tell_start_problems(Vec::new());

        assert_eq!(app.notifications.unseen_count(), 0);
        assert_eq!(app.notifications.toast_timer(), None);
    }
}
