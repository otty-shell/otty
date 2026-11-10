//! The screen of Terminal mode. The tabs feature (issue 10) fills the
//! Sessions section of the left panel and its New session menu; quick launch
//! (issue 12) the Launches section, its menu and the launch editor tab; the
//! terminal (issue 11) the main area; the explorer (issue 07) the right
//! panel.

use iced::Element;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::target_dot::TargetKind;

use super::empty_place;
use crate::app::App;
use crate::chrome::view::status_bar;
use crate::chrome::{StatusBarInput, StatusBarViewModel};
use crate::message::AppEvent;

/// Left panel of Terminal mode under the switcher: the Sessions section
/// over the Launches section, laid out by the sidebar's left panel view
/// once their features move in (issues 10 and 12).
pub(crate) fn left_panel<'a>(_theme: ThemeProps<'a>) -> Element<'a, AppEvent> {
    empty_place()
}

/// Main area of Terminal mode: the content of the active tab, or the start
/// card while no tab is open (issues 10 to 12).
pub(crate) fn main<'a>(_theme: ThemeProps<'a>) -> Element<'a, AppEvent> {
    empty_place()
}

/// Right panel of Terminal mode: the Files section (issue 07).
pub(crate) fn right_panel<'a>(_theme: ThemeProps<'a>) -> Element<'a, AppEvent> {
    empty_place()
}

/// Status bar of Terminal mode: the machine on the left and the version on
/// the right. With no tabs it names this computer; the tabs (issue 10) and
/// terminal (issue 11) features add the machine of the active tab, the
/// folder of the focused pane and the pane and session counts.
pub(crate) fn status_bar<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let machine_name = app
        .host_name
        .as_deref()
        .unwrap_or_else(|| otty_app_i18n::t(otty_app_i18n::Key::ThisComputer));

    let vm = StatusBarViewModel::new(StatusBarInput {
        machine_name,
        machine_target: TargetKind::Local,
        folder: None,
        pane_count: None,
        session_count: 0,
    });

    status_bar::view(&vm, env!("CARGO_PKG_VERSION"), theme)
}

/// Overlays of Terminal mode, bottom to top: the context menu of the
/// Launches tree (issue 12) and the context menu of a terminal pane (issue
/// 11). None of them is here yet.
pub(crate) fn overlays<'a>(
    _theme: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent>> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::testing::window::app;

    #[test]
    fn without_tabs_the_status_bar_names_this_computer_and_the_version() {
        let mut app = app();
        let theme = app.theme_manager.current().clone();

        {
            let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

            ui.find(i18n::t(Key::ThisComputer))
                .expect("this computer should be named");
            ui.find(format!("OTTY {}", env!("CARGO_PKG_VERSION")))
                .expect("the version should show");
        }

        app.host_name = Some(String::from("build-box"));
        let mut ui = simulator(status_bar(&app, ThemeProps::new(&theme)));

        ui.find("build-box").expect("the host name should show");
    }
}
