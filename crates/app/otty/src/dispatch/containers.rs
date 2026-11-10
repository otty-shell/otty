use iced::Task;
use otty_app_containers::{ContainersIntent, ContainersOutcome};
use otty_app_notifications::NotificationsIntent;
use otty_app_slots::Feature;

use super::{notifications, sidebar};
use crate::app::App;
use crate::effects;
use crate::message::AppEvent;
use crate::sidebar::SidebarIntent;

/// Give a Containers mode intent to the containers feature and read what it
/// reports.
///
/// The active mode reaches the feature as
/// [`ContainersIntent::ModeChanged`], and whether the right panel is on
/// screen as [`ContainersIntent::RightPanelHidden`] after every update (see
/// `update.rs`).
pub(crate) fn deliver(
    app: &mut App,
    intent: ContainersIntent,
) -> Task<AppEvent> {
    let outcomes = app.containers.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: ContainersOutcome) -> Task<AppEvent> {
    match outcome {
        ContainersOutcome::Discover => effects::containers::discover_engines(),
        ContainersOutcome::DiscoverInBackground => {
            effects::containers::discover_in_background()
        },
        ContainersOutcome::LoadLists(info) => {
            effects::containers::load_lists(info)
        },
        ContainersOutcome::LoadList {
            info,
            resource,
            containers,
        } => effects::containers::load_list(info, resource, containers),
        ContainersOutcome::CopyToClipboard(text) => {
            effects::containers::copy_to_clipboard(text)
        },
        ContainersOutcome::OpenUrl(url) => effects::containers::open_url(url),
        ContainersOutcome::RevealInspector => {
            sidebar::deliver(app, SidebarIntent::RevealRight)
        },
        ContainersOutcome::Failed(text) => {
            notifications::deliver(app, NotificationsIntent::Push(text))
        },
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_containers::LoadedLists;
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::testing::window::app;

    /// An app with the right panel closed.
    fn app_with_closed_right_panel() -> App {
        let mut app = app();
        if app.sidebar.vm().is_right_open {
            let _ = sidebar::deliver(&mut app, SidebarIntent::ToggleRight);
        }
        assert!(!app.sidebar.vm().is_right_open);

        app
    }

    /// An app with the fixture lists of one engine loaded and the right panel
    /// closed.
    fn app_with_rows_and_closed_right_panel() -> App {
        let mut app = app_with_closed_right_panel();
        let resources = fixtures::resources(EngineKind::Docker);

        for intent in [
            ContainersIntent::Enter,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
            ContainersIntent::ListsLoaded {
                engine: fixtures::engine(EngineKind::Docker).address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        ] {
            let _ = app.containers.reduce(intent, ());
        }

        app
    }

    #[test]
    fn an_intent_is_reduced_by_the_feature_and_its_engine_work_starts() {
        let mut app = app();

        let task = deliver(&mut app, ContainersIntent::Enter);

        assert!(app.containers.is_active());
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn pressing_a_row_opens_the_closed_right_panel() {
        let mut app = app_with_rows_and_closed_right_panel();

        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));

        assert!(app.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_with_the_right_panel_open_leaves_it_open() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = sidebar::deliver(&mut app, SidebarIntent::ToggleRight);
        assert!(app.sidebar.vm().is_right_open);

        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));
        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));

        assert!(app.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_the_selected_row_again_opens_the_closed_right_panel() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));
        let _ = sidebar::deliver(&mut app, SidebarIntent::ToggleRight);
        assert!(!app.sidebar.vm().is_right_open);

        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));

        assert!(app.sidebar.vm().is_right_open);
    }

    #[test]
    fn a_press_that_selects_no_row_leaves_the_right_panel_closed() {
        let mut no_rows = app_with_closed_right_panel();
        let mut past_the_last_row = app_with_rows_and_closed_right_panel();

        let _ = deliver(&mut no_rows, ContainersIntent::SelectRow(0));
        let _ =
            deliver(&mut past_the_last_row, ContainersIntent::SelectRow(9999));

        assert!(!no_rows.sidebar.vm().is_right_open);
        assert!(!past_the_last_row.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_leaves_focus_mode_with_the_right_panel_visible() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = sidebar::deliver(&mut app, SidebarIntent::ToggleFocusMode);
        assert!(app.sidebar.vm().is_focus_mode);

        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));

        let sidebar = app.sidebar.vm();
        assert!(!sidebar.is_focus_mode);
        assert!(sidebar.is_right_open);
        assert!(app.sidebar.right_ratio() > 0.0);
    }

    /// A port link opens a page and leaves the panel as it is. The browser
    /// is not started: only the reduce runs here.
    #[test]
    fn a_port_link_leaves_the_right_panel_closed() {
        let mut app = app_with_rows_and_closed_right_panel();
        let rows = fixtures::resources(EngineKind::Docker).containers().len();
        let mut opened = Vec::new();

        for row in 0..rows {
            opened.extend(
                app.containers.reduce(
                    ContainersIntent::OpenPortLink { row, link: 0 },
                    (),
                ),
            );
        }

        assert!(!app.sidebar.vm().is_right_open);
        assert!(
            opened.iter().any(|outcome| matches!(
                outcome,
                ContainersOutcome::OpenUrl(_)
            ))
        );
        assert!(!opened.contains(&ContainersOutcome::RevealInspector));
    }

    /// A browser that can not be started is told in a toast, in the same
    /// update as its answer.
    #[test]
    fn a_link_that_can_not_be_opened_shows_a_toast() {
        let mut app = app();

        let _ = deliver(
            &mut app,
            ContainersIntent::LinkOpened(Err(String::from(
                "cannot run xdg-open: not found",
            ))),
        );

        let text = otty_app_i18n::with_error(
            otty_app_i18n::Key::TplLinkOpenFailed,
            "cannot run xdg-open: not found",
        );
        assert_eq!(app.notifications.unseen_count(), 1);
        let mut ui = simulator(crate::view::view(&app));
        assert!(ui.find(text.as_str()).is_ok());
    }
}
