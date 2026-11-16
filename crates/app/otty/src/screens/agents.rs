//! The screen of Agents mode. The agents feature fills it: the profiles and
//! the projects in the left panel, the sessions table of the selected
//! project, the dashboard of the selected profile or a card in the main area,
//! the left side of the status bar, and the profile dialog, the remove
//! dialog and its menus as the mode overlays. The mode has no right panel
//! (spec Decision 6): the frame hides it.

use iced::widget::{Space, container, mouse_area, responsive};
use iced::{Element, Length};
use otty_app_agents::AgentsIntent;
use otty_app_domain::mode::Mode;
use otty_app_i18n as i18n;
use otty_app_modes::ModesIntent;
use otty_app_slots::{MainArea, OverlayKind, Overlays, StatusBar as _};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::size;

use super::over_main_area;
use crate::app::App;
use crate::frame::sidebar::SidebarIntent;
use crate::message::AppEvent;

/// Left panel of Agents mode under the switcher: the profiles and the
/// projects.
pub(crate) fn left_panel<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.agents.left_panel(theme).map(AppEvent::Agents)
}

/// Main area of Agents mode: the sessions table of the selected project,
/// the dashboard of a profile, or a card while there is neither.
pub(crate) fn main<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.agents
        .main_area(theme)
        .into_element(theme)
        .map(AppEvent::Agents)
}

/// Right panel of Agents mode: empty. The frame hides the panel in this
/// mode, so it is drawn only if it is ever shown.
pub(crate) fn right_panel<'a>() -> Element<'a, AppEvent> {
    Space::new().width(Length::Fill).height(Length::Fill).into()
}

/// Status bar of Agents mode: the counts of the projects and their sessions
/// on the left, from the agents feature; on the right the sessions button,
/// which switches to Terminal mode, and the OTTY version, as in Containers
/// mode. In a bar too narrow for all of them the left side gives way first:
/// its last texts are left out, whole, and the right side keeps its size.
pub(crate) fn status_bar<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let palette = theme.theme.ui().clone();
    let sessions = app.tabs.len();

    // The width is known only during layout.
    let bar = responsive(move |room| {
        let left = app.agents.status_bar();
        bar_that_fits(left, sessions, &palette, room.width).into()
    });

    container(bar).height(size::STATUSBAR).into()
}

/// Return whether the profile dialog, the remove dialog or a menu of
/// Agents mode is open.
#[cfg(any(not(target_os = "macos"), test))]
pub(crate) fn has_overlay(app: &App) -> bool {
    app.agents.has_profile_dialog()
        || app.agents.has_remove_dialog()
        || app.agents.has_menu()
}

/// Overlays of Agents mode: the profile dialog, the remove dialog or a
/// menu, whichever is on top. A dialog lies over the whole
/// window; a menu opens at the pointer in the main area, where the pointer
/// is measured.
pub(crate) fn overlays<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent>> {
    let Some(overlay) = app.agents.overlay(theme) else {
        return Vec::new();
    };

    let kind = overlay.kind();
    let content = overlay.into_content().map(AppEvent::Agents);
    if kind == OverlayKind::Dialog {
        return vec![content];
    }

    // The menu layer covers the main area, so it reports where the pointer
    // is in place of the main area: the next menu opens at the right spot.
    let layer = mouse_area(over_main_area(content)).on_move(|position| {
        AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { position })
    });

    vec![layer.into()]
}

/// The status bar with the right side whole and as many of the `left`
/// texts, from the first, as fit in `available`.
fn bar_that_fits(
    left: Vec<StatusItem<AgentsIntent>>,
    sessions: usize,
    palette: &UiPalette,
    available: f32,
) -> StatusBar<AppEvent> {
    let mut bar = StatusBar::new(palette)
        .right(StatusItem::button(
            i18n::status_sessions(sessions),
            AppEvent::Modes(ModesIntent::Select(Mode::Terminal)),
        ))
        .right(StatusItem::text(format!(
            "OTTY {}",
            env!("CARGO_PKG_VERSION")
        )));

    for item in left {
        let width = bar.width() + size::STATUS_GAP + item.width();
        if width > available {
            break;
        }
        bar = bar.left(item.map(AppEvent::Agents));
    }

    bar
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_agents::{
        AgentId, AgentSession, Profile, ProfileStatus, SessionId,
    };
    use otty_app_agents::{
        AgentsCtx, AgentsIntent, LoadedProfile, LoadedProfiles, LoadedSessions,
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
                saved: None,
                problem: None,
            }),
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: Ok(vec![session]),
                missing_folders: Vec::new(),
            }),
        ] {
            let _ = app.agents.reduce(intent, AgentsCtx::default());
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

    /// The sessions table of the selected project fills the main area;
    /// a right press on a row opens its menu over the main area, and its
    /// items take the press.
    #[test]
    fn a_selected_project_shows_its_sessions_table_and_the_row_menu() {
        let mut app = agents_app();
        let _ = app.agents.reduce(
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
            AgentsCtx::default(),
        );
        let mut ui = simulator(view(&app));
        ui.find("Fix the tabs").expect("the session should show");
        ui.find(i18n::t(Key::AgentsColumnTitle))
            .expect("the table should show");
        drop(ui);

        let _ = app
            .agents
            .reduce(AgentsIntent::OpenSessionMenu(0), AgentsCtx::default());
        let mut ui = simulator(view(&app));

        assert!(has_overlay(&app));
        ui.click(i18n::t(Key::AgentsCopySessionId))
            .expect("the menu should show");
        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Agents(AgentsIntent::SessionMenu(
                otty_app_agents::SessionAction::CopySessionId
            ))
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

    /// Where each status bar text is in a bar `width` wide, or `None` for
    /// one that is left out.
    fn status_texts_at(width: f32) -> Vec<Option<iced::Rectangle>> {
        let app = agents_app();
        let theme = AppTheme::default();
        let bar =
            iced::widget::container(status_bar(&app, ThemeProps::new(&theme)))
                .width(width);
        let mut ui = simulator(bar);

        [
            i18n::t(Key::AgentsStatusOneProject).to_string(),
            i18n::t(Key::AgentsStatusOneSession).to_string(),
            i18n::status_sessions(0),
            format!("OTTY {}", env!("CARGO_PKG_VERSION")),
        ]
        .iter()
        .map(|label| ui.find(label.as_str()).ok().map(|text| text.bounds()))
        .collect()
    }

    #[test]
    fn in_a_narrow_bar_the_left_side_gives_way_before_the_right_side() {
        let wide = status_texts_at(1000.0);
        let width = 200.0;

        let narrow = status_texts_at(width);

        assert!(wide.iter().all(Option::is_some), "{wide:?}");
        assert!(narrow[1].is_none(), "the last left text goes first");
        let sessions = narrow[2].expect("the sessions button stays");
        let version = narrow[3].expect("the version stays");
        assert!(version.x + version.width <= width, "{version:?}");
        assert!(
            (sessions.width - wide[2].expect("shown").width).abs() < 0.5,
            "the sessions button keeps its size: {sessions:?}"
        );
        if let Some(projects) = narrow[0] {
            assert!(projects.x + projects.width <= sessions.x, "{projects:?}");
        }
    }

    /// The whole window of `app`, `width` by `height`.
    fn window_of(
        app: &App,
        width: f32,
        height: f32,
    ) -> iced_test::Simulator<'_, AppEvent> {
        iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(width, height),
            view(app),
        )
    }

    fn dashboard_app() -> App {
        let mut app = agents_app();
        let _ = app.agents.reduce(
            AgentsIntent::SelectProfile(claude()),
            AgentsCtx::default(),
        );

        app
    }

    /// Where the hint of a profile that is not logged in, the Profile
    /// title, Log in and Edit are in a window `width` by `height`.
    fn dashboard_parts(width: f32, height: f32) -> [iced::Rectangle; 4] {
        let app = dashboard_app();
        let mut ui = window_of(&app, width, height);

        [
            Key::AgentsNotLoggedInHint,
            Key::AgentsProfileTitle,
            Key::AgentsLogIn,
            Key::AgentsEdit,
        ]
        .map(|key| {
            ui.find(i18n::t(key))
                .expect("the dashboard part should show")
                .bounds()
        })
    }

    #[test]
    fn in_a_wide_window_the_dashboard_has_two_columns() {
        let [hint, profile, log_in, edit] = dashboard_parts(1280.0, 800.0);

        assert!(profile.x > hint.x + 100.0, "{profile:?} beside {hint:?}");
        assert!(edit.x > log_in.x && edit.x + edit.width <= 1280.0);
    }

    #[test]
    fn in_the_smallest_window_the_dashboard_has_one_column_and_its_buttons() {
        let (width, height) = (crate::app::MIN_WINDOW_WIDTH, 600.0);

        let [hint, profile, log_in, edit] = dashboard_parts(width, height);

        assert!((profile.x - hint.x).abs() < 1.0, "{profile:?} {hint:?}");
        assert!(profile.y > hint.y, "{profile:?} under {hint:?}");
        for button in [log_in, edit] {
            assert!(button.x + button.width <= width, "{button:?} in reach");
        }
    }

    /// The least window leaves the main area about 670 wide: the minimum
    /// widths of the sessions table fit, so it does not scroll sideways
    /// there (it does in a main area of 592, when the panel is wider).
    #[test]
    fn in_the_smallest_window_the_sessions_table_shows_every_column() {
        let mut app = agents_app();
        let _ = app.agents.reduce(
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
            AgentsCtx::default(),
        );
        let width = crate::app::MIN_WINDOW_WIDTH;
        let mut ui = window_of(&app, width, 600.0);

        let title = ui
            .find(i18n::t(Key::AgentsColumnTitle))
            .expect("the table should show")
            .bounds();
        let updated = ui
            .find(i18n::t(Key::AgentsColumnUpdated))
            .expect("the last column should show")
            .bounds();

        assert!(title.x > 0.0, "{title:?}");
        assert!(updated.x + updated.width <= width, "{updated:?}");
    }

    #[test]
    fn the_profile_dialog_is_drawn_over_the_window_and_takes_the_click() {
        let mut app = agents_app();
        let _ = app
            .agents
            .reduce(AgentsIntent::OpenAddProfile, AgentsCtx::default());
        let mut ui = simulator(view(&app));

        assert!(has_overlay(&app));
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("the dialog should show");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Agents(AgentsIntent::Dialog(
                otty_app_agents::ProfileDialogIntent::Cancelled
            ))
        )));
    }

    /// A menu of the left panel lies in the main area at the pointer, its
    /// items take the press, and the layer reports the pointer.
    #[test]
    fn the_funnel_menu_lies_over_the_main_area_and_takes_the_press() {
        let mut app = agents_app();
        let ctx = AgentsCtx {
            cursor: iced::Point::new(60.0, 120.0),
            screen_size: iced::Size::new(1024.0, 700.0),
            has_local_pane: false,
        };
        let _ = app.agents.reduce(AgentsIntent::ToggleProjectSearch, ctx);
        let _ = app.agents.reduce(AgentsIntent::OpenFilterMenu, ctx);
        let mut ui = simulator(view(&app));

        assert!(has_overlay(&app));
        ui.click(i18n::t(Key::AgentsFilterSortByName))
            .expect("the menu should show");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Agents(AgentsIntent::ProjectFilter(
                otty_app_agents::ProjectFilterChange::SortByName(true)
            ))
        )));
    }

    #[test]
    fn the_right_panel_of_agents_mode_is_empty() {
        let mut ui = simulator(right_panel());

        assert!(ui.find(i18n::t(Key::AgentsContinue)).is_err());
    }
}
