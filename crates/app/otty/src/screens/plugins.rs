//! The screen of Plugins mode (issue 30). The plugins feature fills it:
//! the installed plugins with the search and the two ways to add one in
//! the left panel, the page of the selected plugin or a card that says
//! what to do in the main area, and its dialogs as the mode overlays. The
//! mode has no right panel: the frame hides it.

use iced::widget::Space;
use iced::{Element, Length};
use otty_app_slots::{MainArea, Overlays};
use otty_app_theme::theme::ThemeProps;

use super::bar_with_sessions;
use crate::app::App;
use crate::message::AppEvent;

/// Left panel of Plugins mode under the switcher: the installed plugins.
pub(crate) fn left_panel<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.plugins.left_panel(theme).map(AppEvent::Plugins)
}

/// Main area of Plugins mode: the page of the selected plugin, or a card
/// while none is selected or none is installed.
pub(crate) fn main<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.plugins
        .main_area(theme)
        .into_element(theme)
        .map(AppEvent::Plugins)
}

/// Right panel of Plugins mode: empty. The frame hides the panel in this
/// mode, so it is drawn only if it is ever shown.
pub(crate) fn right_panel<'a>() -> Element<'a, AppEvent> {
    Space::new().width(Length::Fill).height(Length::Fill).into()
}

/// Status bar of Plugins mode: nothing of its own on the left; on the
/// right the sessions button, which switches to Terminal mode, and the
/// OTTY version, as in the other modes.
pub(crate) fn status_bar<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    bar_with_sessions(theme.theme.ui(), app.tabs.len(), Vec::new)
}

/// Overlays of Plugins mode: its open dialog, over the whole window.
pub(crate) fn overlays<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent>> {
    app.plugins
        .overlay(theme)
        .map(|dialog| dialog.into_content().map(AppEvent::Plugins))
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_domain::mode::Mode;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_modes::ModesIntent;
    use otty_app_plugins::PluginsIntent;

    use crate::dispatch;
    use crate::message::AppEvent;
    use crate::testing::window::app;
    use crate::view::view;

    /// Issue 30: Plugins mode with nothing installed shows the card that
    /// says what to do in the window, and Add plugin… opens its dialog
    /// over the window.
    #[test]
    fn plugins_mode_says_what_to_do_and_add_plugin_opens_its_dialog() {
        let mut app = app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Plugins)),
        );

        let hint_shows = simulator(view(&app))
            .find(i18n::t(Key::PluginsEmptyHint))
            .is_ok();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Plugins(PluginsIntent::AddPlugin),
        );

        assert!(hint_shows);
        assert!(app.plugins.has_dialog());
        assert!(
            simulator(view(&app))
                .find(i18n::t(Key::PluginsAddPluginTitle))
                .is_ok()
        );
    }

    #[test]
    fn the_right_panel_of_plugins_mode_is_hidden() {
        let mut app = app();

        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Plugins)),
        );

        assert!(!app.sidebar.vm().is_right_open);
    }
}
