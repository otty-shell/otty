use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::{Element, Size, Subscription, Task, Theme};
use otty_agents::{Agent, AgentEnv, Registry};
use otty_app_agents::{AgentList, Agents};
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

use crate::frame::sidebar::SidebarWidget;
use crate::frame::title_bar::TitleBar;
use crate::message::{AppEvent, Message};
use crate::{dispatch, language, layout, startup, subscription, update, view};

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
    /// OTTY's cache folder (`~/.cache/otty`): the kept price table of
    /// Agents mode and the folder its hidden terminals run in. It is found
    /// once at start, next to `config_dir`.
    pub(crate) cache_dir: PathBuf,
    /// The system shell, read from `SHELL` at start: the default shell of
    /// the settings, used when the saved shell is missing or empty.
    pub(crate) default_shell: String,
    /// Size of the main area between the header and the status bar; the
    /// header menus are kept inside it. It follows the window size.
    pub(crate) screen_size: Size,
    pub(crate) title_bar: TitleBar,
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
    /// The profiles of Agents mode with their projects and sessions, and
    /// the agents OTTY knows and what they look at in the environment,
    /// kept for the whole run.
    pub(crate) agents: Agents,
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
        let files_home = home_or_temp(home.as_deref());
        let config_dir = config_dir(&files_home);
        let cache_dir = cache_dir(&files_home);
        let known = otty_agents::known_agents();
        let registry = registry_at_start(&known);
        let agents: AgentList = Arc::from(registry.listed_agents(known));
        let agent_env = agent_env(files_home, &agents, |variable| {
            std::env::var_os(variable)
        });

        let (saved, settings_problem) =
            otty_app_settings_io::read_at_start(&config_dir, &default_shell);
        // The language must be active before any text is built, the texts
        // of the problems too.
        language::apply(saved.language().fixed());
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
            cache_dir,
            agents,
            Arc::new(registry),
            agent_env,
            TerminalWorkspace::new(),
        );
        let told = app.tell_start_problems(problems);
        let first =
            startup::run(shell, app.config_dir.clone()).map(Message::Internal);

        (app, Task::batch([told, first]))
    }

    /// Build the window state from what was read at start: the saved
    /// settings (the theme, the shell new terminals start, ...), the pinned
    /// modes, the saved launches, the home folder, the system shell, the
    /// config and cache folders, the agents OTTY knows, the registry that
    /// names the agents and their providers, and what the agents look at
    /// in the environment. It reads nothing from disk.
    ///
    /// `terminal` is the terminal feature, which carries how the terminal
    /// of a new pane is started: the app gives the one that starts real
    /// programs, a test may give one whose programs are mocks.
    pub(crate) fn with_saved(
        saved: SettingsData,
        pinned_modes: &[Mode],
        launches: QuickLaunchFile,
        home: Option<PathBuf>,
        default_shell: String,
        config_dir: PathBuf,
        cache_dir: PathBuf,
        agents: AgentList,
        registry: Arc<Registry>,
        agent_env: AgentEnv,
        terminal: TerminalWorkspace,
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
            cache_dir,
            screen_size,
            title_bar: TitleBar::new(),
            sidebar: SidebarWidget::new(),
            modes: Modes::with_pinned(pinned_modes),
            notifications: Notifications::new(),
            explorer: Explorer::new(),
            palette: Palette::new(),
            tabs: Tabs::new(),
            settings: Settings::new(saved, default_shell.clone()),
            terminal,
            quick_launch: QuickLaunches::new(launches),
            default_shell,
            containers,
            agents: Agents::new(agents, registry, agent_env),
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
                dispatch::deliver(self, NotificationsIntent::Push(text))
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

/// Return the folder the app's own folders and the agents go by: the
/// user's `home`, or the system temp folder without one.
fn home_or_temp(home: Option<&Path>) -> PathBuf {
    home.map_or_else(std::env::temp_dir, Path::to_path_buf)
}

/// Return the folder of the app's files: `.config/otty` in `home`.
fn config_dir(home: &Path) -> PathBuf {
    home.join(".config").join("otty")
}

/// Return OTTY's cache folder: `.cache/otty` in `home`, next to the config
/// folder.
fn cache_dir(home: &Path) -> PathBuf {
    home.join(".cache").join("otty")
}

/// Return what the agents look at in the environment: `home` and, for
/// each of `agents`, the folder its config folder variable names, such as
/// `CODEX_HOME`, and the file its database variable names, such as
/// `KILO_DB`; `variable` gives the value of a variable. An empty value
/// counts as not set.
fn agent_env(
    home: PathBuf,
    agents: &[Box<dyn Agent>],
    variable: impl Fn(&str) -> Option<OsString>,
) -> AgentEnv {
    let mut env = AgentEnv::new(home);

    for agent in agents {
        let folder = agent.config_folder();
        let names = std::iter::once(folder.variable())
            .chain(folder.database_variable());
        for name in names {
            let path = variable(name).filter(|value| !value.is_empty());
            if let Some(path) = path {
                env = env.with_config_dir(name, PathBuf::from(path));
            }
        }
    }

    env
}

/// Return the registry OTTY starts with: the built-in entries of `agents`
/// and of the providers `otty-agents` reads (spec 2026-11-19-agent-plugins,
/// "Links in the interface"). A debug build leaves out the entries whose
/// ids `OTTY_HIDE_ENTRIES` lists, such as `deepseek,kilo`, so the owner can
/// see an agent or a provider no plugin serves.
fn registry_at_start(agents: &[Box<dyn Agent>]) -> Registry {
    let registry = Registry::built_in(agents);

    #[cfg(debug_assertions)]
    let registry = match std::env::var("OTTY_HIDE_ENTRIES") {
        Ok(ids) => without_entries(registry, &ids),
        Err(_) => registry,
    };

    registry
}

/// `registry` without the agents and the providers whose ids `ids` lists,
/// split by commas.
#[cfg(debug_assertions)]
fn without_entries(registry: Registry, ids: &str) -> Registry {
    ids.split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .fold(registry, |registry, id| {
            registry
                .without_agent(&otty_agents::AgentId::new(id))
                .without_provider(&otty_agents::ProviderId::new(id))
        })
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
    /// the pinned modes, the shell integration) lies in its config folder,
    /// and the kept files of Agents mode in its cache folder.
    /// A test app keeps it under the system temp folder, so no test reads
    /// or writes the files of the user running it.
    #[test]
    fn a_test_app_keeps_its_files_under_the_temp_folder() {
        let app = app();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let files_home = home_or_temp(home.as_deref());

        assert!(app.config_dir.starts_with(std::env::temp_dir()));
        assert_ne!(app.config_dir, config_dir(&files_home));
        assert!(app.cache_dir.starts_with(std::env::temp_dir()));
        assert_ne!(app.cache_dir, cache_dir(&files_home));
    }

    #[test]
    fn without_a_home_the_temp_folder_stands_for_it() {
        let home = Path::new("/home/user");

        assert_eq!(home_or_temp(Some(home)), home);
        assert_eq!(home_or_temp(None), std::env::temp_dir());
    }

    #[test]
    fn the_config_folder_is_otty_in_the_users_config_folder() {
        assert_eq!(
            config_dir(Path::new("/home/user")),
            PathBuf::from("/home/user/.config/otty")
        );
    }

    #[test]
    fn the_cache_folder_is_otty_in_the_users_cache_folder() {
        assert_eq!(
            cache_dir(Path::new("/home/user")),
            PathBuf::from("/home/user/.cache/otty")
        );
    }

    #[test]
    fn the_agents_look_at_the_home_folder_and_their_variables() {
        let agents = otty_agents::known_agents();
        let variable = |name: &str| match name {
            "CLAUDE_CONFIG_DIR" => Some(OsString::from("/data/claude")),
            "CODEX_HOME" => Some(OsString::new()),
            _ => None,
        };

        let env = agent_env(PathBuf::from("/home/user"), &agents, variable);

        assert_eq!(env.home(), Path::new("/home/user"));
        assert_eq!(
            env.config_dir_of("CLAUDE_CONFIG_DIR"),
            Some(Path::new("/data/claude"))
        );
        assert_eq!(
            env.config_dir_of("CODEX_HOME"),
            None,
            "an empty value is not set"
        );
    }

    #[test]
    fn the_agents_with_a_database_look_at_its_variable_too() {
        let agents = otty_agents::known_agents();
        let variable = |name: &str| match name {
            "XDG_DATA_HOME" => Some(OsString::from("/data")),
            "KILO_DB" => Some(OsString::from("/data/kilo-work.db")),
            _ => None,
        };

        let env = agent_env(PathBuf::from("/home/user"), &agents, variable);

        assert_eq!(
            env.config_dir_of("XDG_DATA_HOME"),
            Some(Path::new("/data"))
        );
        assert_eq!(
            env.config_dir_of("KILO_DB"),
            Some(Path::new("/data/kilo-work.db"))
        );
        assert_eq!(env.config_dir_of("OPENCODE_DB"), None);
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
