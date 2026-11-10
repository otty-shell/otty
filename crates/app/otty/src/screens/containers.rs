//! The screen of Containers mode. The containers feature fills it: the engine
//! tree in the left panel, the lists in the main area and the left side of
//! the status bar. Issue 14 adds the inspector in the right panel, the Run
//! container form in the main area, and the row menu and the dialogs as the
//! mode overlays.

use iced::Element;
use otty_app_domain::mode::Mode;
use otty_app_i18n as i18n;
use otty_app_slots::{MainArea, StatusBar as _};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};

use super::empty_place;
use crate::app::App;
use crate::message::AppEvent;
use crate::modes::ModesIntent;

/// Left panel of Containers mode under the switcher: the engine tree.
pub(crate) fn left_panel<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.containers.engine_tree(theme).map(AppEvent::Containers)
}

/// Main area of Containers mode: the open list of an engine, or a card while
/// there is none.
pub(crate) fn main<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.containers
        .main_area(theme)
        .into_element(theme)
        .map(AppEvent::Containers)
}

/// Right panel of Containers mode: the inspector, which comes with issue 14.
pub(crate) fn right_panel<'a>(_theme: ThemeProps<'a>) -> Element<'a, AppEvent> {
    empty_place()
}

/// Status bar of Containers mode: the engine and the counts of the open
/// list on the left, from the containers feature; on the right the sessions
/// button, which switches to Terminal mode, and the OTTY version.
pub(crate) fn status_bar<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let mut bar = StatusBar::new(theme.theme.ui());
    for item in app.containers.status_bar() {
        bar = bar.left(item.map(AppEvent::Containers));
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

/// Overlays of Containers mode, bottom to top: the row menu, the
/// confirmation dialog and the create dialog. They come with issue 14.
pub(crate) fn overlays<'a>(
    _theme: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent>> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_containers::{ContainersIntent, LoadedLists};
    use otty_app_slots::Feature;
    use otty_app_theme::theme::AppTheme;
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::dispatch;
    use crate::view::view;

    /// The app in Containers mode with the fixture Docker engine and its
    /// lists.
    fn containers_app_with_lists() -> App {
        let mut app = crate::testing::window::app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Containers)),
        );
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        for intent in [
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
            ContainersIntent::ListsLoaded {
                engine: docker.address().clone(),
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

    /// The page intent a press in the main area sent.
    fn page_intent(event: &AppEvent) -> Option<&ContainersIntent> {
        match event {
            AppEvent::Containers(ContainersIntent::FromPage {
                intent, ..
            }) => Some(intent),
            _ => None,
        }
    }

    #[test]
    fn containers_mode_shows_the_engine_tree_and_the_table() {
        let app = containers_app_with_lists();
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(i18n::Key::ContainersImages))
            .expect("tree should list the images");
        ui.find("otty-fx-exit3")
            .expect("table should list containers");
        ui.click("otty-fx-exit3").expect("row should be clickable");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            page_intent(event),
            Some(ContainersIntent::SelectRow(_))
        )));
    }

    #[test]
    fn a_press_on_a_list_of_the_tree_opens_it() {
        let app = containers_app_with_lists();
        let mut ui = simulator(view(&app));

        ui.click(i18n::t(i18n::Key::ContainersImages))
            .expect("tree should list the images");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersIntent::Open(_))
        )));
    }

    #[test]
    fn before_an_engine_is_found_the_main_area_says_it_searches() {
        let mut app = crate::testing::window::app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Containers)),
        );
        let mut ui = simulator(view(&app));

        ui.find(i18n::t(i18n::Key::ContainersSearching))
            .expect("the searching card should show");
    }

    #[test]
    fn with_an_engine_the_bar_shows_engine_counts_sessions_and_version() {
        let app = containers_app_with_lists();
        let theme = AppTheme::default();
        let containers = fixtures::resources(EngineKind::Docker);
        let running = containers
            .containers()
            .iter()
            .filter(|c| c.state() == otty_containers::ContainerState::Running)
            .count();
        let total = containers.containers().len();
        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        for label in [
            String::from("Docker 29.9.1"),
            format!("{total} containers, {running} running"),
            i18n::status_sessions(0),
            format!("OTTY {}", env!("CARGO_PKG_VERSION")),
        ] {
            ui.find(label.as_str())
                .expect("status bar item should be visible");
        }
        assert!(ui.find(i18n::t(i18n::Key::ContainersStatusMode)).is_err());
    }

    #[test]
    fn the_sessions_button_switches_to_terminal_mode() {
        let app = containers_app_with_lists();
        let theme = AppTheme::default();
        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        ui.click(i18n::status_sessions(0).as_str())
            .expect("button should be visible");

        let events: Vec<AppEvent> = ui.into_messages().collect();
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesIntent::Select(Mode::Terminal))]
        ));
    }
}
