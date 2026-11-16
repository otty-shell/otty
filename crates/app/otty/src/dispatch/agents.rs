use std::path::Path;
use std::sync::Arc;

use iced::Task;
use otty_agents::AgentCommand;
use otty_app_agents::{Agents, AgentsIntent, AgentsOutcome};
use otty_app_agents_io as agents_io;
use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::tab::TabMachine;
use otty_app_notifications::NotificationsIntent;
use otty_app_tabs::TabsIntent;

use super::{Delivery, deliver, terminal};
use crate::app::App;
use crate::message::AppEvent;
use crate::terminal_settings;

/// The agents feature, behind Agents mode, reads what it reports.
///
/// The active mode reaches the feature as [`AgentsIntent::ModeChanged`]
/// (see `dispatch/modes.rs`).
impl Delivery for AgentsIntent {
    type Component = Agents;

    fn parts(app: &mut App) -> (&mut Agents, ()) {
        (&mut app.agents, ())
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
            )
            .map(AppEvent::Agents),
            AgentsOutcome::OpenTab {
                title,
                folder,
                command,
            } => {
                let intent = agent_tab_intent(app, title, &folder, &command);
                deliver(app, intent)
            },
            AgentsOutcome::OpenLoginTab { title, command } => {
                let home = app.agent_env.home().to_path_buf();
                let intent = agent_tab_intent(app, title, &home, &command);
                deliver(app, intent)
            },
            AgentsOutcome::LoadCandidates { agent, profiles } => {
                agents_io::load_candidates(
                    Arc::clone(app.agents.known_agents()),
                    app.agent_env.clone(),
                    agent,
                    profiles,
                )
                .map(AppEvent::Agents)
            },
            AgentsOutcome::SaveProfiles { saved, create_dir } => {
                agents_io::save_profiles(&app.config_dir, saved, create_dir)
                    .map(AppEvent::Agents)
            },
            AgentsOutcome::Failed(text) => {
                deliver(app, NotificationsIntent::Push(text))
            },
        }
    }
}

/// The command tab on this computer that runs the agent `command` in
/// `folder` (the project folder, or the home folder for a login), through the user's login shell, so an agent installed by a
/// version manager is found when OTTY was started from the desktop (spec
/// Decision 3). The tab stays open when the agent fails, so its error can
/// be read. The tabs show Terminal mode, as for every opened tab.
fn agent_tab_intent(
    app: &App,
    title: String,
    folder: &Path,
    command: &AgentCommand,
) -> TabsIntent {
    let session = app.shell_session.command_session(
        folder,
        command.program(),
        command.args(),
        command.env(),
    );
    let settings =
        terminal_settings::for_session(&terminal::base_settings(app), session);

    TabsIntent::OpenCommandTab {
        title,
        settings: Box::new(settings),
        exit_behavior: ExitBehavior::CloseOnSuccess,
        machine: TabMachine::ThisComputer,
    }
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
    fn the_found_folders_and_the_save_run_in_the_background() {
        let mut app = app();

        let candidates = AgentsIntent::read(
            &mut app,
            AgentsOutcome::LoadCandidates {
                agent: otty_agents::AgentId::new("codex"),
                profiles: Vec::new(),
            },
        );
        let save = AgentsIntent::read(
            &mut app,
            AgentsOutcome::SaveProfiles {
                saved: otty_agents::SavedProfiles::default(),
                create_dir: None,
            },
        );

        assert_eq!(candidates.units(), 1);
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
}
