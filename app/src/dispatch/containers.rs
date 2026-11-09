use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::containers::event::ContainersIntent;

/// Deliver a Containers intent to the widget's reducer.
pub(crate) fn deliver(
    app: &mut App,
    intent: ContainersIntent,
) -> Task<AppEvent> {
    app.widgets
        .containers
        .reduce(intent)
        .map(AppEvent::Containers)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::app::view::view;
    use crate::dispatch::dispatch;
    use crate::i18n::{self, Key};
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::child::actions::event::ActionsIntent;
    use crate::widgets::containers::event::{
        ContainersEffect, ContainersEvent,
    };
    use crate::widgets::containers::types::{
        ActionRequest, ContainersError, EngineAction, LoadedLists,
    };
    use crate::widgets::sidebar::event::SidebarIntent;

    #[test]
    fn an_intent_is_reduced_by_the_widget_and_its_effect_comes_back() {
        let (mut app, _) = App::new();

        let events =
            collect_outputs(deliver(&mut app, ContainersIntent::Enter));

        assert!(app.widgets.containers.is_active());
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::Discover
            ))]
        ));
    }

    /// An app with the right panel closed.
    fn app_with_closed_right_panel() -> App {
        let (mut app, _) = App::new();
        if app.widgets.sidebar.vm().is_right_open {
            let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        }
        assert!(!app.widgets.sidebar.vm().is_right_open);

        app
    }

    /// An app with the fixture lists of one engine loaded and the right panel
    /// closed.
    fn app_with_rows_and_closed_right_panel() -> App {
        let mut app = app_with_closed_right_panel();
        let resources = fixtures::resources(EngineKind::Docker);
        let containers = &mut app.widgets.containers;

        let _ = containers.reduce(ContainersIntent::Enter);
        let _ = containers.reduce(ContainersIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = containers.reduce(ContainersIntent::ListsLoaded {
            engine: fixtures::engine(EngineKind::Docker).address().clone(),
            lists: LoadedLists {
                containers: Ok(resources.containers().to_vec()),
                images: Ok(resources.images().to_vec()),
                volumes: Ok(resources.volumes().to_vec()),
                networks: Ok(resources.networks().to_vec()),
            },
        });

        app
    }

    /// Dispatch `event` and the events its tasks produce, as the iced
    /// runtime does. Only for events that end in `Task::done`: see
    /// `collect_outputs`.
    fn dispatch_to_the_end(app: &mut App, event: AppEvent) {
        for next in collect_outputs(dispatch(app, event)) {
            dispatch_to_the_end(app, next);
        }
    }

    /// Press the row at `row` of the shown table, to the end of what follows.
    fn press_row(app: &mut App, row: usize) {
        dispatch_to_the_end(
            app,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::SelectRow(row),
            )),
        );
    }

    /// Owner, Q1: a failed action shows a toast over the table and goes to
    /// the list, in place of a notice above the table.
    #[test]
    fn a_failed_action_shows_a_toast_over_the_table() {
        let mut app = app_with_rows_and_closed_right_panel();
        let request = ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            EngineAction::StopContainer(String::from("4f1c2b9a8e7d")),
            String::from("web-1"),
        );

        dispatch_to_the_end(
            &mut app,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Actions(ActionsIntent::Finished {
                    request,
                    result: Err(ContainersError::new("busy")),
                }),
            )),
        );

        let text = i18n::containers_action_failed(
            Key::TplContainersStopFailed,
            "web-1",
            "busy",
        );
        assert_eq!(app.widgets.notifications.unseen_count(), 1);
        let mut ui = simulator(view(&app));
        assert!(ui.find(text.as_str()).is_ok());
    }

    #[test]
    fn pressing_a_row_opens_the_closed_right_panel() {
        let mut app = app_with_rows_and_closed_right_panel();

        press_row(&mut app, 0);

        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_with_the_right_panel_open_leaves_it_open() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        assert!(app.widgets.sidebar.vm().is_right_open);

        press_row(&mut app, 0);
        press_row(&mut app, 0);

        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_the_selected_row_again_opens_the_closed_right_panel() {
        let mut app = app_with_rows_and_closed_right_panel();
        press_row(&mut app, 0);
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        assert!(!app.widgets.sidebar.vm().is_right_open);

        press_row(&mut app, 0);

        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn a_press_that_selects_no_row_leaves_the_right_panel_closed() {
        let mut no_rows = app_with_closed_right_panel();
        let mut past_the_last_row = app_with_rows_and_closed_right_panel();

        press_row(&mut no_rows, 0);
        press_row(&mut past_the_last_row, usize::MAX);

        assert!(!no_rows.widgets.sidebar.vm().is_right_open);
        assert!(!past_the_last_row.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_leaves_focus_mode_with_the_right_panel_visible() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);
        assert!(app.widgets.sidebar.vm().is_focus_mode);

        press_row(&mut app, 0);

        let sidebar = app.widgets.sidebar.vm();
        assert!(!sidebar.is_focus_mode);
        assert!(sidebar.is_right_open);
        assert!(app.widgets.sidebar.right_ratio() > 0.0);
    }

    #[test]
    fn a_row_press_changes_the_sidebar_only_through_the_effect() {
        let mut app = app_with_rows_and_closed_right_panel();

        let sent =
            collect_outputs(deliver(&mut app, ContainersIntent::SelectRow(0)));

        assert!(!app.widgets.sidebar.vm().is_right_open);
        assert!(matches!(
            sent.as_slice(),
            [AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::RevealInspector
            ))]
        ));
    }

    /// The effects of the reduced intents are not run: a port link would
    /// open a real browser page.
    #[test]
    fn the_row_menu_button_the_right_click_and_a_port_link_leave_the_panel_closed()
     {
        let mut app = app_with_rows_and_closed_right_panel();
        let rows = fixtures::resources(EngineKind::Docker).containers().len();
        let mut sent = Vec::new();

        for row in 0..rows {
            for intent in [
                ContainersIntent::OpenPortLink { row, link: 0 },
                ContainersIntent::RowMenuPressed {
                    index: row,
                    position: iced::Point::ORIGIN,
                },
                ContainersIntent::OpenRowMenu {
                    index: row,
                    position: iced::Point::ORIGIN,
                },
            ] {
                sent.extend(collect_outputs(deliver(&mut app, intent)));
                assert!(!app.widgets.sidebar.vm().is_right_open);
            }
        }

        // The rows were real: the port links found their pages.
        assert!(sent.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::OpenUrl(_)
            ))
        )));
        assert!(!sent.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::RevealInspector
            )) | AppEvent::Sidebar(_)
        )));
    }
}
