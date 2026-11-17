use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    /// is, where its menus open; and whether the tab in front of Terminal
    /// mode has a pane on this computer, which Continue in a split splits.
    fn parts(app: &mut App) -> (&mut Agents, AgentsCtx) {
        let ctx = AgentsCtx {
            cursor: app.sidebar.cursor(),
            screen_size: app.screen_size,
            has_local_pane: app.terminal.has_local_pane_in_front(),
        };

        (&mut app.agents, ctx)
    }

    fn read(app: &mut App, outcome: AgentsOutcome) -> Task<AppEvent> {
        match outcome {
            AgentsOutcome::LoadProfiles => agents_io::load_profiles(
                Arc::clone(app.agents.known_agents()),
                app.agent_env.clone(),
                &app.config_dir,
                app.shell_session.program(),
            )
            .map(AppEvent::Agents),
            AgentsOutcome::LoadSessions(profile) => agents_io::load_sessions(
                Arc::clone(app.agents.known_agents()),
                profile,
            )
            .map(AppEvent::Agents),
            AgentsOutcome::LoadLimits(profile) => agents_io::load_limits(
                Arc::clone(app.agents.known_agents()),
                profile,
                app.shell_session.program(),
                &cache_dir(app),
            )
            .map(AppEvent::Agents),
            AgentsOutcome::LoadLimitsInBackground(profile) => {
                agents_io::load_limits_in_background(
                    Arc::clone(app.agents.known_agents()),
                    profile,
                    app.shell_session.program(),
                    &cache_dir(app),
                )
                .map(AppEvent::Agents)
            },
            AgentsOutcome::LoadSpend(profile) => agents_io::load_spend(
                Arc::clone(app.agents.known_agents()),
                profile,
            )
            .map(AppEvent::Agents),
            AgentsOutcome::LoadBill(profile) => agents_io::load_bill(
                Arc::clone(app.agents.known_agents()),
                profile,
                app.shell_session.program(),
            )
            .map(AppEvent::Agents),
            AgentsOutcome::LoadSessionStats { profile, session } => {
                agents_io::load_session_stats(
                    Arc::clone(app.agents.known_agents()),
                    profile,
                    session,
                )
                .map(AppEvent::Agents)
            },
            AgentsOutcome::LoadProjectStats { folder, profiles } => {
                agents_io::load_project_stats(
                    Arc::clone(app.agents.known_agents()),
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
            AgentsOutcome::OpenLoginTab { title, command } => {
                let home = app.agent_env.home().to_path_buf();
                let intent = agent_tab_intent(app, title, &home, &command);
                deliver(app, intent)
            },
            AgentsOutcome::SaveProfiles { saved, create_dir } => {
                agents_io::save_profiles(&app.config_dir, saved, create_dir)
                    .map(AppEvent::Agents)
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

/// OTTY's cache folder: `.cache/otty` in the user's home folder (the system
/// temp folder without one), next to `.config/otty`.
fn cache_dir(app: &App) -> PathBuf {
    app.agent_env.home().join(".cache").join("otty")
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
    use crate::testing::window::{
        TEST_SHELL_PATH, app, app_with_scripted_terminals,
    };

    fn codex_command() -> AgentCommand {
        AgentCommand::new("codex")
            .with_arg("resume")
            .with_arg("a1")
            .with_env("CODEX_HOME", "/home/user/.codex-work")
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

        let task =
            AgentsIntent::read(&mut app, AgentsOutcome::LoadBill(profile));

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
        use iced::keyboard::key::{Named, NativeCode, Physical};
        use iced::keyboard::{Event, Key, Location, Modifiers};
        use otty_agents::{AgentSession, ProfileStatus, SessionId};
        use otty_app_agents::{LoadedProfile, LoadedProfiles, LoadedSessions};
        use otty_app_slots::Component;

        let (mut app, _terminals) = app_with_scripted_terminals();
        let _ = deliver(&mut app, ModesIntent::Select(Mode::Agents));
        let codex = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("codex"),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        );
        let session = AgentSession::new(
            SessionId::new("a1"),
            PathBuf::from("/work/otty"),
            "Fix the tabs",
            std::time::SystemTime::UNIX_EPOCH,
        );
        for intent in [
            AgentsIntent::ProfilesLoaded(LoadedProfiles {
                profiles: vec![LoadedProfile {
                    profile: codex.clone(),
                    status: ProfileStatus::NotLoggedIn,
                }],
                saved: None,
                problem: None,
                utc_offset_seconds: None,
                read_at: std::time::SystemTime::UNIX_EPOCH,
            }),
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: codex,
                sessions: Ok(vec![session]),
                missing_folders: Vec::new(),
            }),
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
            AgentsIntent::SelectSession(0),
        ] {
            let _ = app.agents.reduce(intent, AgentsCtx::default());
        }
        let enter = Event::KeyPressed {
            key: Key::Named(Named::Enter),
            modified_key: Key::Named(Named::Enter),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        };

        let _ = crate::dispatch::dispatch(&mut app, AppEvent::Keyboard(enter));

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_title(), Some("Fix the tabs"));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    /// Log in opens one command tab in the home folder of the agents and
    /// Terminal mode shows it. The terminals are mocks: no agent starts.
    #[test]
    fn log_in_opens_a_tab_in_the_home_folder() {
        let (mut app, _terminals) = app_with_scripted_terminals();
        let _ = deliver(&mut app, ModesIntent::Select(Mode::Agents));

        let _ = AgentsIntent::read(
            &mut app,
            AgentsOutcome::OpenLoginTab {
                title: String::from("Codex · Work"),
                command: AgentCommand::new("codex")
                    .with_arg("login")
                    .with_env("CODEX_HOME", "/home/user/.codex-work"),
            },
        );

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_title(), Some("Codex · Work"));
        assert_eq!(app.modes.active(), Mode::Terminal);
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
        assert!(app.agents.has_menu());
    }
}
