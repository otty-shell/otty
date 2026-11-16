//! The screen of Agents mode. The agents feature fills it: the profiles and
//! the projects in the left panel, the sessions of the selected project or
//! a card in the main area, and the left side of the status bar. The right
//! panel is not used (spec Decision 6).

use iced::widget::Space;
use iced::{Element, Length};
use otty_app_domain::mode::Mode;
use otty_app_i18n as i18n;
use otty_app_modes::ModesIntent;
use otty_app_slots::{MainArea, StatusBar as _};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};

use crate::app::App;
use crate::message::AppEvent;

/// Left panel of Agents mode under the switcher: the profiles and the
/// projects.
pub(crate) fn left_panel<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.agents.left_panel(theme).map(AppEvent::Agents)
}

/// Main area of Agents mode: the sessions of the selected project, or a
/// card while there is none.
pub(crate) fn main<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.agents
        .main_area(theme)
        .into_element(theme)
        .map(AppEvent::Agents)
}

/// Right panel of Agents mode: empty, the mode does not use it.
pub(crate) fn right_panel<'a>() -> Element<'a, AppEvent> {
    Space::new().width(Length::Fill).height(Length::Fill).into()
}

/// Status bar of Agents mode: the counts of the projects and their sessions
/// on the left, from the agents feature; on the right the sessions button,
/// which switches to Terminal mode, and the OTTY version, as in Containers
/// mode.
pub(crate) fn status_bar<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let mut bar = StatusBar::new(theme.theme.ui());
    for item in app.agents.status_bar() {
        bar = bar.left(item.map(AppEvent::Agents));
    }

    bar.right(StatusItem::button(
        i18n::status_sessions(app.tabs.len()),
        AppEvent::Modes(ModesIntent::Select(Mode::Terminal)),
    ))
    .right(StatusItem::text(format!(
        "OTTY {}",
        env!("CARGO_PKG_VERSION")
    )))
    .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_agents::{
        AgentId, AgentSession, Profile, ProfileStatus, SessionId,
    };
    use otty_app_agents::{
        AgentsIntent, LoadedProfile, LoadedProfiles, LoadedSessions,
    };
    use otty_app_i18n::Key;
    use otty_app_slots::Component;
    use otty_app_theme::theme::AppTheme;

    use super::*;
    use crate::dispatch;
    use crate::testing::window::app;
    use crate::view::view;

    fn claude() -> Profile {
        Profile::default_for(
            AgentId::new("claude-code"),
            "Claude Code",
            PathBuf::from("/home/user/.claude"),
        )
    }

    /// An app in Agents mode with one Claude Code session in `/work/otty`.
    fn agents_app() -> App {
        let mut app = app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Agents)),
        );
        let session = AgentSession::new(
            SessionId::new("c1"),
            PathBuf::from("/work/otty"),
            "Fix the tabs",
            std::time::SystemTime::UNIX_EPOCH,
        );
        for intent in [
            AgentsIntent::ProfilesLoaded(LoadedProfiles {
                profiles: vec![LoadedProfile {
                    profile: claude(),
                    status: ProfileStatus::NotLoggedIn,
                }],
                problem: None,
            }),
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: Ok(vec![session]),
                missing_folders: Vec::new(),
            }),
        ] {
            let _ = app.agents.reduce(intent, ());
        }

        app
    }

    #[test]
    fn agents_mode_shows_the_profiles_and_projects_and_selects_a_project() {
        let app = agents_app();
        let mut ui = simulator(view(&app));

        ui.find("Claude Code").expect("the profile should show");
        ui.find(i18n::t(Key::AgentsStartTitle))
            .expect("the start card should show");
        ui.click("otty").expect("the project should show");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Agents(AgentsIntent::SelectProject(folder))
                if folder == &PathBuf::from("/work/otty")
        )));
    }

    #[test]
    fn a_selected_project_shows_its_sessions_with_continue() {
        let mut app = agents_app();
        let _ = app.agents.reduce(
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
            (),
        );
        let mut ui = simulator(view(&app));

        ui.find("Fix the tabs").expect("the session should show");
        ui.click(i18n::t(Key::AgentsContinue))
            .expect("Continue should show");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Agents(AgentsIntent::Continue { .. })
        )));
    }

    #[test]
    fn the_bar_shows_the_counts_the_sessions_button_and_the_version() {
        let app = agents_app();
        let theme = AppTheme::default();
        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        for label in [
            i18n::t(Key::AgentsStatusOneProject).to_string(),
            i18n::t(Key::AgentsStatusOneSession).to_string(),
            i18n::status_sessions(0),
            format!("OTTY {}", env!("CARGO_PKG_VERSION")),
        ] {
            ui.find(label.as_str())
                .expect("status bar item should be visible");
        }
    }

    #[test]
    fn the_right_panel_of_agents_mode_is_empty() {
        let mut ui = simulator(right_panel());

        assert!(ui.find(i18n::t(Key::AgentsContinue)).is_err());
    }
}
