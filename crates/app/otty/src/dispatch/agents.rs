use std::path::{Path, PathBuf};

use iced::Task;
use iced::widget::{operation, pane_grid};
use otty_agents::AgentCommand;
use otty_app_agents::{Agents, AgentsCtx, AgentsIntent, AgentsOutcome};
use otty_app_agents_io as agents_io;
use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::mode::Mode;
use otty_app_domain::tab::TabMachine;
use otty_app_modes::ModesIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;
use otty_ui_term::settings::Settings;

use super::{Delivery, deliver, terminal};
use crate::app::App;
use crate::message::AppEvent;
use crate::terminal_settings;

/// The agents feature, behind Agents mode, reads what it reports.
///
/// The active mode reaches the feature as [`AgentsIntent::ModeChanged`]
/// (see `dispatch/modes.rs`), and the start of OTTY as
/// [`AgentsIntent::LoadInBackground`] (see `startup.rs`). A real failure the
/// feature reports as [`AgentsOutcome::Failed`] becomes a notification.
impl Delivery for AgentsIntent {
    type Component = Agents;

    /// The feature reads where the pointer is and how big the main area
    /// is, where its menus open; whether the tab in front of Terminal mode
    /// has a pane on this computer, which Continue in a split splits; and
    /// the folder of that pane, where the New session dialog opens.
    fn parts(app: &mut App) -> (&mut Agents, AgentsCtx) {
        let ctx = AgentsCtx {
            cursor: app.sidebar.cursor(),
            screen_size: app.screen_size,
            has_local_pane: app.terminal.has_local_pane_in_front(),
            local_folder: app.terminal.focused_local_folder(),
        };

        (&mut app.agents, ctx)
    }

    fn read(app: &mut App, outcome: AgentsOutcome) -> Task<AppEvent> {
        match outcome {
            AgentsOutcome::LoadProfiles => agents_io::load_profiles(
                app.agents.known_agents(),
                app.agents.env(),
                &app.config_dir,
                app.shell_session.program(),
            )
            .map(AppEvent::Agents),
            AgentsOutcome::LoadVersion(agent) => agents_io::load_version(
                app.agents.known_agents(),
                agent,
                app.shell_session.program(),
            )
            .map(AppEvent::Agents),
            AgentsOutcome::LoadSessions(profile) => {
                agents_io::load_sessions(app.agents.known_agents(), profile)
                    .map(AppEvent::Agents)
            },
            AgentsOutcome::LoadLimits(profile) => agents_io::load_limits(
                app.agents.known_agents(),
                profile,
                app.shell_session.program(),
                &app.cache_dir,
            )
            .map(AppEvent::Agents),
            AgentsOutcome::LoadLimitsInBackground(profile) => {
                agents_io::load_limits_in_background(
                    app.agents.known_agents(),
                    profile,
                    app.shell_session.program(),
                    &app.cache_dir,
                )
                .map(AppEvent::Agents)
            },
            AgentsOutcome::LoadSpend(profile) => {
                agents_io::load_spend(app.agents.known_agents(), profile)
                    .map(AppEvent::Agents)
            },
            AgentsOutcome::LoadBill { profile, admin_key } => {
                agents_io::load_bill(
                    app.agents.known_agents(),
                    profile,
                    admin_key,
                    app.shell_session.program(),
                )
                .map(AppEvent::Agents)
            },
            AgentsOutcome::LoadPrices { fetch_again } => {
                agents_io::load_prices(&app.cache_dir, fetch_again)
                    .map(AppEvent::Agents)
            },
            AgentsOutcome::LoadSessionStats { profile, session } => {
                agents_io::load_session_stats(
                    app.agents.known_agents(),
                    profile,
                    session,
                )
                .map(AppEvent::Agents)
            },
            AgentsOutcome::LoadProjectStats { folder, profiles } => {
                agents_io::load_project_stats(
                    app.agents.known_agents(),
                    folder,
                    profiles,
                )
                .map(AppEvent::Agents)
            },
            AgentsOutcome::OpenTab {
                title,
                folder,
                command,
            } => {
                let intent = agent_tab_intent(app, title, &folder, &command);
                deliver(app, intent)
            },
            // Terminal mode shows first, so the panes take its grid size;
            // then the new pane opens on the right of the focused one.
            AgentsOutcome::OpenSplit { folder, command } => {
                let settings = agent_settings(app, &folder, &command);
                Task::batch([
                    deliver(app, ModesIntent::Select(Mode::Terminal)),
                    deliver(
                        app,
                        TerminalIntent::SplitFocusedPaneWith {
                            axis: pane_grid::Axis::Vertical,
                            settings: Box::new(settings),
                        },
                    ),
                ])
            },
            // A shell tab like New terminal, whose shell starts in the
            // folder, as Open in terminal of Files does.
            AgentsOutcome::OpenFolder(folder) => deliver(
                app,
                TabsIntent::OpenTerminalTab {
                    folder: Some(folder),
                },
            ),
            AgentsOutcome::CopyCommand(command) => {
                agents_io::copy_command(&command).map(AppEvent::Agents)
            },
            AgentsOutcome::CopyText(text) => {
                agents_io::copy_text(text).map(AppEvent::Agents)
            },
            AgentsOutcome::SaveProfiles { saved, create_dir } => {
                agents_io::save_profiles(&app.config_dir, saved, create_dir)
                    .map(AppEvent::Agents)
            },
            AgentsOutcome::LookUpFolder(query) => {
                agents_io::look_up_folder(query).map(AppEvent::Agents)
            },
            AgentsOutcome::FocusNewSessionFolder => {
                operation::focus(otty_app_agents::NEW_SESSION_FOLDER)
            },
            AgentsOutcome::Failed(text) => {
                deliver(app, NotificationsIntent::Push(text))
            },
            AgentsOutcome::FocusProjectSearch => {
                operation::focus(otty_app_agents::PROJECT_SEARCH)
            },
            AgentsOutcome::MenuOpened => super::take_keyboard(),
        }
    }
}

/// Show Agents mode, then open the New session dialog over it on `folder`
/// (the home folder without one). A command and Files both start a session
/// in a folder this way.
pub(super) fn new_session_in(
    app: &mut App,
    folder: Option<PathBuf>,
) -> Task<AppEvent> {
    let shown = deliver(app, ModesIntent::Select(Mode::Agents));
    let opened = deliver(app, AgentsIntent::OpenNewSessionDialog(folder));

    Task::batch([shown, opened])
}

/// The command tab on this computer that runs the agent `command` in
/// `folder` (the project folder, or the home folder for a login). The tab
/// stays open when the agent fails, so its error can be read. The tabs show
/// Terminal mode, as for every opened tab.
fn agent_tab_intent(
    app: &App,
    title: String,
    folder: &Path,
    command: &AgentCommand,
) -> TabsIntent {
    let settings = agent_settings(app, folder, command);

    TabsIntent::OpenCommandTab {
        title,
        settings: Box::new(settings),
        exit_behavior: ExitBehavior::CloseOnSuccess,
        machine: TabMachine::ThisComputer,
    }
}

/// The terminal settings that run the agent `command` in `folder` through
/// the user's login shell, so an agent installed by a version manager is
/// found when OTTY was started from the desktop (spec Decision 3).
fn agent_settings(
    app: &App,
    folder: &Path,
    command: &AgentCommand,
) -> Settings {
    let session = app.shell_session.command_session(
        folder,
        command.program(),
        command.args(),
        command.env(),
    );

    terminal_settings::for_session(&terminal::base_settings(app), session)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_app_domain::mode::Mode;
    use otty_app_modes::ModesIntent;
    use otty_ui_term::settings::SessionKind;

    use super::*;
    use crate::testing::agents::{
        PROJECT, SESSION_TITLE, show_claude_with_a_session,
    };
    use crate::testing::keyboard::enter_press;
    use crate::testing::window::{
        TEST_SHELL_PATH, app, app_with_scripted_terminals,
    };

    fn codex_command() -> AgentCommand {
        AgentCommand::new("codex")
            .with_arg("resume")
            .with_arg("a1")
            .with_env("CODEX_HOME", "/home/user/.codex-work")
    }

    /// Show Agents mode with one Claude Code session and select it in the
    /// sessions table of its project.
    fn select_the_session(app: &mut App) {
        show_claude_with_a_session(app);

        for intent in [
            AgentsIntent::SelectProject(PathBuf::from(PROJECT)),
            AgentsIntent::SelectSession(0),
        ] {
            let _ = deliver(app, intent);
        }
    }

    #[test]
    fn agents_mode_reads_the_profiles_when_it_opens() {
        let mut app = app();

        let task = deliver(&mut app, ModesIntent::Select(Mode::Agents));

        assert!(app.agents.is_active());
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn the_sessions_of_a_profile_are_read_in_the_background() {
        let mut app = app();
        let profile = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("codex"),
            "Codex",
            PathBuf::from("/no/such/.codex"),
        );

        let task =
            AgentsIntent::read(&mut app, AgentsOutcome::LoadSessions(profile));

        assert_eq!(task.units(), 1);
    }

    #[test]
    fn the_limits_of_a_profile_are_read_in_the_background() {
        let mut app = app();
        let profile = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/no/such/.stranger"),
        );

        let task =
            AgentsIntent::read(&mut app, AgentsOutcome::LoadLimits(profile));

        assert_eq!(task.units(), 1);
    }

    /// While Agents mode is away the limits are read without starting the
    /// agent's program; the read is still one background task.
    #[test]
    fn the_limits_of_a_profile_are_read_with_the_mode_away() {
        let mut app = app();
        let profile = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/no/such/.stranger"),
        );

        let task = AgentsIntent::read(
            &mut app,
            AgentsOutcome::LoadLimitsInBackground(profile),
        );

        assert_eq!(task.units(), 1);
    }

    /// At the start of OTTY, with Terminal mode on screen, Agents mode
    /// starts reading the profiles and Containers mode looking for the
    /// engines; neither mode shows.
    #[test]
    fn the_first_reads_start_with_no_mode_on_screen() {
        use otty_app_containers::ContainersIntent;

        let mut app = app();

        let agents = deliver(&mut app, AgentsIntent::LoadInBackground);
        let containers = deliver(&mut app, ContainersIntent::LoadInBackground);

        assert_eq!(agents.units(), 1);
        assert_eq!(containers.units(), 1);
        assert_eq!(app.modes.active(), Mode::Terminal);
        assert!(!app.agents.is_active());
        assert!(!app.containers.is_active());
    }

    #[test]
    fn the_tokens_of_a_profile_are_counted_in_the_background() {
        let mut app = app();
        let profile = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/no/such/.stranger"),
        );

        let task =
            AgentsIntent::read(&mut app, AgentsOutcome::LoadSpend(profile));

        assert_eq!(task.units(), 1);
    }

    #[test]
    fn the_bill_of_a_profile_is_read_in_the_background() {
        let mut app = app();
        let profile = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/no/such/.stranger"),
        );

        let admin_key = otty_agents::AdminKeyRef::EnvVar(String::from(
            "OTTY_TEST_NO_SUCH_ADMIN_KEY",
        ));

        let task = AgentsIntent::read(
            &mut app,
            AgentsOutcome::LoadBill { profile, admin_key },
        );

        assert_eq!(task.units(), 1);
    }

    /// The statistics of a session and of a project are read in the
    /// background; nothing is read here, the profiles are made up.
    #[test]
    fn the_statistics_are_read_in_the_background() {
        let mut app = app();
        let profile = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/no/such/.stranger"),
        );

        let session = AgentsIntent::read(
            &mut app,
            AgentsOutcome::LoadSessionStats {
                profile: profile.clone(),
                session: otty_agents::SessionId::new("s1"),
            },
        );
        let project = AgentsIntent::read(
            &mut app,
            AgentsOutcome::LoadProjectStats {
                folder: PathBuf::from("/work/otty"),
                profiles: vec![profile],
            },
        );

        assert_eq!(session.units(), 1);
        assert_eq!(project.units(), 1);
    }

    /// The folder typed in the New session dialog is looked up in the
    /// background; nothing is read here, the folder is made up.
    #[test]
    fn the_typed_folder_is_looked_up_in_the_background() {
        let mut app = app();

        let task = AgentsIntent::read(
            &mut app,
            AgentsOutcome::LookUpFolder(otty_app_agents::FolderQuery {
                asker: otty_app_agents::FolderAsker::NewSessionDialog,
                folder: PathBuf::from("/no/such/folder"),
                list_in: PathBuf::from("/no/such"),
                prefix: String::from("folder"),
            }),
        );

        assert_eq!(task.units(), 1);
    }

    /// The feature is told the folder of the focused tab on this computer,
    /// where the New session dialog opens. The terminals are mocks.
    #[test]
    fn the_feature_is_told_the_folder_of_the_focused_local_tab() {
        let (mut app, _terminals) = app_with_scripted_terminals();
        let before = AgentsIntent::parts(&mut app).1.local_folder;
        let _ = deliver(
            &mut app,
            TabsIntent::OpenTerminalTab {
                folder: Some(PathBuf::from("/work/otty")),
            },
        );

        let told = AgentsIntent::parts(&mut app).1.local_folder;

        assert_eq!(before, None);
        assert_eq!(told, Some(PathBuf::from("/work/otty")));
    }

    /// The agent runs through the user's shell started as a login shell,
    /// in the project folder, with the variables of its profile.
    #[test]
    fn an_agent_tab_runs_the_agent_through_the_login_shell() {
        let app = app();

        let intent = agent_tab_intent(
            &app,
            String::from("Fix the tabs"),
            Path::new("/work/otty"),
            &codex_command(),
        );

        let TabsIntent::OpenCommandTab {
            title,
            settings,
            exit_behavior,
            machine,
        } = intent
        else {
            panic!("expected one command tab");
        };
        assert_eq!(title, "Fix the tabs");
        assert_eq!(exit_behavior, ExitBehavior::CloseOnSuccess);
        assert_eq!(machine, TabMachine::ThisComputer);
        let SessionKind::Local(options) = &settings.engine.session else {
            panic!("expected a local session");
        };
        assert_eq!(options.program(), TEST_SHELL_PATH);
        assert_eq!(
            options.args(),
            ["-l", "-i", "-c", "exec 'codex' 'resume' 'a1'"]
        );
        assert_eq!(
            options.working_directory(),
            &Some(PathBuf::from("/work/otty"))
        );
        assert_eq!(
            options.envs().get("CODEX_HOME").map(String::as_str),
            Some("/home/user/.codex-work")
        );
    }

    /// Continue opens one command tab and Terminal mode shows it. The
    /// terminals of this app are mocks, so no shell and no agent starts.
    #[test]
    fn opening_a_session_opens_one_tab_in_terminal_mode() {
        let (mut app, _terminals) = app_with_scripted_terminals();
        let _ = deliver(&mut app, ModesIntent::Select(Mode::Agents));

        let _ = AgentsIntent::read(
            &mut app,
            AgentsOutcome::OpenTab {
                title: String::from("Fix the tabs"),
                folder: PathBuf::from("/work/otty"),
                command: codex_command(),
            },
        );

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_title(), Some("Fix the tabs"));
        assert_eq!(app.modes.active(), Mode::Terminal);
        assert!(!app.agents.is_active());
    }

    /// Continue in a split splits the focused pane of the shell tab in
    /// front and shows Terminal mode; the feature is told that the pane is
    /// there. The terminals are mocks: no agent starts.
    #[test]
    fn continue_in_a_split_splits_the_pane_in_front_and_shows_terminal_mode() {
        let (mut app, _terminals) = app_with_scripted_terminals();
        let before = AgentsIntent::parts(&mut app).1.has_local_pane;
        crate::testing::window::open_terminal_tab(&mut app);
        let _ = deliver(&mut app, ModesIntent::Select(Mode::Agents));
        let told = AgentsIntent::parts(&mut app).1.has_local_pane;

        let _ = AgentsIntent::read(
            &mut app,
            AgentsOutcome::OpenSplit {
                folder: PathBuf::from("/work/otty"),
                command: codex_command(),
            },
        );

        assert!(!before);
        assert!(told);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.terminal.pane_count(), Some(2));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    /// The agent never reports its folder, as a shell does, so Files and
    /// the status bar show the project folder the tab was started in, also
    /// after another tab was in front.
    #[test]
    fn a_continued_session_shows_its_project_folder_in_files() {
        let (mut app, _terminals) = app_with_scripted_terminals();
        let _ = deliver(&mut app, ModesIntent::Select(Mode::Agents));
        let _ = AgentsIntent::read(
            &mut app,
            AgentsOutcome::OpenTab {
                title: String::from("Fix the tabs"),
                folder: PathBuf::from("/work/otty"),
                command: codex_command(),
            },
        );
        let agent_tab = app.tabs.active_tab_id().expect("the agent tab");
        let shell_tab = crate::testing::window::open_shell_tab(&mut app);
        let _ =
            deliver(&mut app, TabsIntent::ActivateTab { tab_id: shell_tab });

        let _ =
            deliver(&mut app, TabsIntent::ActivateTab { tab_id: agent_tab });

        assert_eq!(
            app.explorer.watched_folders(),
            vec![PathBuf::from("/work/otty")]
        );
        assert_eq!(app.terminal.focused_folder_path(), Some("/work/otty"));
    }

    /// Open folder in terminal opens a shell tab, not the agent, and
    /// Terminal mode shows it.
    #[test]
    fn the_folder_opens_in_a_shell_tab() {
        let (mut app, _terminals) = app_with_scripted_terminals();
        let _ = deliver(&mut app, ModesIntent::Select(Mode::Agents));

        let _ = AgentsIntent::read(
            &mut app,
            AgentsOutcome::OpenFolder(PathBuf::from("/work/otty")),
        );

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_title(), Some(app.shell_session.name()));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    #[test]
    fn the_resume_command_and_the_id_go_to_the_clipboard() {
        let mut app = app();

        let command = AgentsIntent::read(
            &mut app,
            AgentsOutcome::CopyCommand(codex_command()),
        );
        let id = AgentsIntent::read(
            &mut app,
            AgentsOutcome::CopyText(String::from("a1")),
        );

        assert_eq!(command.units(), 1);
        assert_eq!(id.units(), 1);
    }

    /// Enter that no widget took continues the selected session of the
    /// sessions table in a command tab. The terminals are mocks: no agent
    /// starts.
    #[test]
    fn enter_continues_the_selected_session_in_a_tab() {
        let (mut app, _terminals) = app_with_scripted_terminals();
        select_the_session(&mut app);

        let _ = crate::dispatch::dispatch(
            &mut app,
            AppEvent::Keyboard(enter_press()),
        );

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_title(), Some(SESSION_TITLE));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    /// Enter that no widget took in Terminal or Containers mode never
    /// reaches Agents mode, even with a session selected there. The
    /// terminals are mocks: no agent starts.
    #[test]
    fn enter_in_another_mode_does_not_reach_agents_mode() {
        let (mut app, _terminals) = app_with_scripted_terminals();
        select_the_session(&mut app);

        for mode in [Mode::Terminal, Mode::Containers] {
            let _ = deliver(&mut app, ModesIntent::Select(mode));
            let _ = crate::dispatch::dispatch(
                &mut app,
                AppEvent::Keyboard(enter_press()),
            );
        }

        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.modes.active(), Mode::Containers);
    }

    #[test]
    fn the_save_runs_in_the_background() {
        let mut app = app();

        let save = AgentsIntent::read(
            &mut app,
            AgentsOutcome::SaveProfiles {
                saved: otty_agents::SavedProfiles::default(),
                create_dir: None,
            },
        );

        assert_eq!(save.units(), 1);
    }

    #[test]
    fn a_problem_of_the_feature_shows_a_toast() {
        let mut app = app();

        let _ = AgentsIntent::read(
            &mut app,
            AgentsOutcome::Failed(String::from("Could not read agents.json")),
        );

        assert_eq!(app.notifications.unseen_count(), 1);
        let mut ui = simulator(crate::view::view(&app));
        assert!(ui.find("Could not read agents.json").is_ok());
    }

    /// The search field takes the keyboard when it opens, and a menu of the
    /// left panel takes it from the field.
    #[test]
    fn the_search_field_and_the_menus_move_the_keyboard_focus() {
        use crate::testing::task_outputs::{focuses_nothing, focuses_widget};

        let mut app = app();
        let _ = deliver(&mut app, ModesIntent::Select(Mode::Agents));

        let search = deliver(&mut app, AgentsIntent::ToggleProjectSearch);
        let menu = deliver(&mut app, AgentsIntent::OpenFilterMenu);

        assert!(focuses_widget(search, &otty_app_agents::PROJECT_SEARCH));
        assert!(focuses_nothing(menu));
        assert_eq!(
            app.agents.top_overlay(),
            Some(otty_app_agents::AgentsOverlay::Menu)
        );
    }
}
