use std::path::{Path, PathBuf};

use iced::{Element, Size, Subscription, Task, Theme};
use otty_app_containers::{Containers, ContainersIntent};
use otty_app_domain::mode::Mode;
use otty_app_explorer::Explorer;
use otty_app_modes::Modes;
use otty_app_notifications::{Notifications, NotificationsIntent};
use otty_app_palette::Palette;
use otty_app_quick_launch::{QuickLaunchFile, QuickLaunches};
use otty_app_settings::{Settings, SettingsData};
use otty_app_shell::ShellSession;
use otty_app_slots::Component;
use otty_app_tabs::Tabs;
use otty_app_terminal::TerminalWorkspace;
use otty_app_theme::theme::ThemeManager;

use crate::frame::chrome::ChromeWidget;
use crate::frame::sidebar::SidebarWidget;
use crate::message::{AppEvent, Message};
use crate::{dispatch, effects, layout, subscription, update, view};

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;
/// The shell new terminals start by default when `SHELL` is not set.
const FALLBACK_SHELL: &str = "/bin/bash";

/// Root state of the window: the frame widgets and what the frame read at
/// start. A feature gets its field here when its issue moves it in.
pub(crate) struct App {
    pub(crate) theme_manager: ThemeManager,
    /// The session new terminal tabs start.
    pub(crate) shell_session: ShellSession,
    /// The user's home folder, read from `HOME` at start. Terminal tabs
    /// get it when they open and write folders inside it as `~`.
    pub(crate) home: Option<PathBuf>,
    /// The folder of the app's files (`~/.config/otty`): the settings, the
    /// launches, the pinned modes and the shell integration. It is found
    /// once at start and given to every read and write of those files.
    pub(crate) config_dir: PathBuf,
    /// The system shell, read from `SHELL` at start: the default shell of
    /// the settings, used when the saved shell is missing or empty.
    pub(crate) default_shell: String,
    /// Size of the main area between the header and the status bar; the
    /// header menus are kept inside it. It follows the window size.
    pub(crate) screen_size: Size,
    pub(crate) chrome: ChromeWidget,
    pub(crate) sidebar: SidebarWidget,
    pub(crate) modes: Modes,
    pub(crate) notifications: Notifications,
    pub(crate) explorer: Explorer,
    pub(crate) palette: Palette,
    /// The open tabs and the Sessions section of the left panel. This
    /// computer's host name is kept there.
    pub(crate) tabs: Tabs,
    /// The saved settings and the Settings dialog. New terminals read the
    /// saved editor and the equalize-panes switch from here, and the theme
    /// they start with from `theme_manager`.
    pub(crate) settings: Settings,
    /// The terminal tabs with their panes and live sessions.
    pub(crate) terminal: TerminalWorkspace,
    /// The saved launches with the Launches section, the launch editors and
    /// the SSH secret prompt.
    pub(crate) quick_launch: QuickLaunches,
    /// The engines of Containers mode with their lists.
    pub(crate) containers: Containers,
}

impl App {
    /// Read the home folder from `HOME` and the system shell from `SHELL`,
    /// find the config folder, read what the frame needs from it (the
    /// settings, the launches and the pinned modes), build the window state
    /// and return the startup work: the shell integration files, the host
    /// name and then the first tab.
    pub(crate) fn new() -> (Self, Task<Message>) {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let default_shell = std::env::var("SHELL")
            .unwrap_or_else(|_| String::from(FALLBACK_SHELL));
        let config_dir = config_dir(home.as_deref());

        let (saved, settings_problem) =
            otty_app_settings_io::read_at_start(&config_dir, &default_shell);
        // The language must be active before any text is built, the texts
        // of the problems too.
        effects::locale::apply(saved.language().fixed());
        let (launches, launches_problem) =
            otty_app_quick_launch_io::load_at_start(&config_dir);
        let (pinned_modes, modes_problem) =
            otty_app_modes_io::load_at_start(&config_dir);
        let shell = saved.terminal_shell().to_string();
        let problems = start_problems(
            settings_problem.map(|problem| problem.text()),
            launches_problem,
            modes_problem,
        );

        let mut app = Self::with_saved(
            saved,
            &pinned_modes,
            launches,
            home,
            default_shell,
            config_dir,
        );
        let told = app.tell_start_problems(problems);
        let first = effects::startup::run(shell, app.config_dir.clone())
            .map(Message::Internal);

        (app, Task::batch([told, first]))
    }

    /// Build the window state from what was read at start: the saved
    /// settings (the theme, the shell new terminals start, ...), the pinned
    /// modes, the saved launches, the home folder, the system shell and the
    /// config folder. It reads nothing from disk.
    pub(crate) fn with_saved(
        saved: SettingsData,
        pinned_modes: &[Mode],
        launches: QuickLaunchFile,
        home: Option<PathBuf>,
        default_shell: String,
        config_dir: PathBuf,
    ) -> Self {
        let mut theme_manager = ThemeManager::new();
        theme_manager.set_theme(saved.theme());
        // The startup work writes the shell integration files and replaces
        // this session before the first tab can open.
        let shell_session =
            ShellSession::without_integration(saved.terminal_shell());

        // The window opens at its smallest size; its first resize event
        // brings the real one.
        let window_size = Size::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT);
        let screen_size = layout::screen_size_from_window(window_size);
        // The containers row menu is kept inside the window.
        let mut containers = Containers::new();
        let _none =
            containers.reduce(ContainersIntent::WindowResized(window_size), ());

        Self {
            theme_manager,
            shell_session,
            home,
            config_dir,
            screen_size,
            chrome: ChromeWidget::new(),
            sidebar: SidebarWidget::new(),
            modes: Modes::with_pinned(pinned_modes),
            notifications: Notifications::new(),
            explorer: Explorer::new(),
            palette: Palette::new(),
            tabs: Tabs::new(),
            settings: Settings::new(saved, default_shell.clone()),
            terminal: TerminalWorkspace::new(),
            quick_launch: QuickLaunches::new(launches),
            default_shell,
            containers,
        }
    }

    /// Tell the user about each file that could not be read at start (the
    /// settings, the launches, the pinned modes): it fell back to the
    /// defaults, and the next save writes over it. Each text, already in the
    /// user's language, shows as a toast.
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

/// Return the folder of the app's files: `.config/otty` in the user's
/// `home`, or a folder in the system temp folder without a home.
fn config_dir(home: Option<&Path>) -> PathBuf {
    match home {
        Some(home) => home.join(".config").join("otty"),
        None => std::env::temp_dir().join("otty"),
    }
}

/// Return the texts of the files that could not be read at start, in the
/// order they are read and told: the settings, the launches, then the
/// pinned modes.
fn start_problems(
    settings: Option<String>,
    launches: Option<String>,
    modes: Option<String>,
) -> Vec<String> {
    settings.into_iter().chain(launches).chain(modes).collect()
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

    /// Every file the app reads and writes (the settings, the launches,
    /// the pinned modes, the shell integration) lies in its config folder.
    /// A test app keeps it under the system temp folder, so no test reads
    /// or writes the files of the user running it.
    #[test]
    fn a_test_app_keeps_its_files_under_the_temp_folder() {
        let app = app();
        let home = std::env::var_os("HOME").map(PathBuf::from);

        assert!(app.config_dir.starts_with(std::env::temp_dir()));
        assert_ne!(app.config_dir, config_dir(home.as_deref()));
    }

    #[test]
    fn the_config_folder_is_otty_in_the_users_config_folder() {
        let home = Path::new("/home/user");

        assert_eq!(
            config_dir(Some(home)),
            PathBuf::from("/home/user/.config/otty")
        );
        assert_eq!(config_dir(None), std::env::temp_dir().join("otty"));
    }

    /// With several broken files the user is told about them in the order
    /// they are read: the settings, the launches, then the pinned modes.
    #[test]
    fn start_problems_come_in_the_order_the_files_are_read() {
        let problems = start_problems(
            Some(String::from("settings")),
            Some(String::from("launches")),
            Some(String::from("modes")),
        );

        assert_eq!(problems, ["settings", "launches", "modes"]);
        assert_eq!(
            start_problems(None, Some(String::from("launches")), None),
            ["launches"]
        );
    }

    #[test]
    fn a_start_without_problems_tells_nothing() {
        let mut app = app();

        let _none = app.tell_start_problems(Vec::new());

        assert_eq!(app.notifications.unseen_count(), 0);
        assert_eq!(app.notifications.subscription().units(), 0);
    }
}
