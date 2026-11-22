//! The screen of Plugins mode (issue 30). The plugins feature fills it:
//! the installed plugins with the search and the Add plugin split button
//! in the left panel, the page of the selected plugin or a card that says
//! what to do in the main area, and its dialogs and the menu of a plugin
//! as the mode overlays. The menu under the chevron of Add plugin is a
//! header menu of the frame, as in Terminal mode. The mode has no right panel: the frame hides it.

use iced::widget::{Space, mouse_area};
use iced::{Element, Length};
use otty_app_plugins::PluginsIntent;
use otty_app_slots::{
    MainArea, OverlayKind, Overlays, SectionMessage, SectionState,
};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::Menu;

use super::{bar_with_sessions, over_main_area};
use crate::app::App;
use crate::frame::sidebar::types::HeaderMenu;
use crate::frame::sidebar::{SidebarIntent, SidebarViewModel};
use crate::message::AppEvent;

/// Left panel of Plugins mode under the switcher: the installed plugins.
/// The sidebar keeps whether the Add plugin menu is open.
pub(crate) fn left_panel<'a>(
    app: &'a App,
    sidebar_vm: SidebarViewModel,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    app.plugins
        .left_panel(theme, installed_section(sidebar_vm))
        .map(plugins_event)
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

/// Overlays of Plugins mode: its open dialog, over the whole window, or
/// the menu of a plugin, at the pointer in the main area, where the
/// pointer is measured.
pub(crate) fn overlays<'a>(
    app: &'a App,
    theme: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent>> {
    let Some(overlay) = app.plugins.overlay(theme) else {
        return Vec::new();
    };

    let kind = overlay.kind();
    let content = overlay.into_content().map(AppEvent::Plugins);
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

/// The Add plugin menu under the chevron of the Installed header: Add
/// plugin… and Add plugin from folder….
pub(crate) fn add_menu(app: &App, theme: ThemeProps<'_>) -> Menu<AppEvent> {
    app.plugins.add_menu(theme).map(AppEvent::Plugins)
}

/// The Installed section is always open; its Add plugin menu is open while
/// the sidebar keeps it open.
fn installed_section(sidebar_vm: SidebarViewModel) -> SectionState {
    let is_menu_open = sidebar_vm
        .open_header_menu
        .is_some_and(|open| open.menu() == HeaderMenu::AddPlugin);

    SectionState::new(true, is_menu_open)
}

/// Turn a press in the left panel into an app event: the chevron of Add
/// plugin belongs to the sidebar, the rest to Plugins mode.
fn plugins_event(message: SectionMessage<PluginsIntent>) -> AppEvent {
    match message {
        SectionMessage::OpenMenu => AppEvent::Sidebar(
            SidebarIntent::HeaderMenuOpen(HeaderMenu::AddPlugin),
        ),
        SectionMessage::Feature(intent) => AppEvent::Plugins(intent),
        // The Installed section is always open, so its title sends no
        // Toggle; one would only close the menu.
        _ => AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss),
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_domain::mode::Mode;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_modes::ModesIntent;
    use otty_app_plugins::PluginsIntent;
    use otty_app_slots::SectionMessage;
    use otty_app_theme::theme::ThemeProps;

    use super::{add_menu, installed_section, plugins_event};
    use crate::app::App;
    use crate::dispatch;
    use crate::frame::sidebar::SidebarIntent;
    use crate::message::AppEvent;
    use crate::testing::keyboard::escape_press;
    use crate::testing::window::app;
    use crate::update::update_input;
    use crate::view::view;

    fn plugins_app() -> App {
        let mut app = app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Plugins)),
        );
        app
    }

    /// Plugins mode with the Add plugin menu open by its chevron.
    fn app_with_add_menu() -> App {
        let mut app = plugins_app();
        let _ = update_input(&mut app, plugins_event(SectionMessage::OpenMenu));
        assert!(app.sidebar.has_header_menu_open());
        app
    }

    /// Issue 30: Plugins mode with nothing installed shows the card that
    /// says what to do in the window, and Add plugin… opens its dialog
    /// over the window.
    #[test]
    fn plugins_mode_says_what_to_do_and_add_plugin_opens_its_dialog() {
        let mut app = plugins_app();

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

    /// The owner, 2026-11-22: the plus of Add plugin adds a plugin from a
    /// repository, by its GitHub link, with no menu.
    #[test]
    fn the_plus_of_add_plugin_opens_the_repository_dialog() {
        let mut app = plugins_app();

        let _ = update_input(
            &mut app,
            plugins_event(SectionMessage::Feature(PluginsIntent::AddPlugin)),
        );

        assert!(!app.sidebar.has_header_menu_open());
        assert!(
            simulator(view(&app))
                .find(i18n::t(Key::PluginsAddPluginTitle))
                .is_ok()
        );
    }

    /// The chevron of Add plugin opens its menu, and the split button
    /// shows the menu open.
    #[test]
    fn the_chevron_of_add_plugin_opens_its_menu() {
        let closed = installed_section(plugins_app().sidebar.vm());

        let app = app_with_add_menu();

        assert!(!closed.is_menu_open());
        assert!(installed_section(app.sidebar.vm()).is_menu_open());
        assert!(!app.plugins.has_dialog());
    }

    /// The menu offers the two ways to add a plugin.
    #[test]
    fn the_add_plugin_menu_offers_a_repository_and_a_folder() {
        let app = plugins_app();
        let theme = ThemeProps::new(app.theme_manager.current());
        let mut sent = Vec::new();

        for key in [Key::PluginsAddPlugin, Key::PluginsAddFromFolder] {
            let mut ui = simulator(iced::Element::from(add_menu(&app, theme)));
            ui.click(i18n::t(key)).expect("menu item should be visible");
            sent.extend(ui.into_messages());
        }

        assert!(matches!(
            sent.as_slice(),
            [
                AppEvent::Plugins(PluginsIntent::AddPlugin),
                AppEvent::Plugins(PluginsIntent::AddFromFolder)
            ]
        ));
    }

    /// Each item of the menu closes it and opens its dialog.
    #[test]
    fn each_item_of_the_add_plugin_menu_closes_it_and_opens_its_dialog() {
        for (choice, title) in [
            (PluginsIntent::AddPlugin, Key::PluginsAddPluginTitle),
            (PluginsIntent::AddFromFolder, Key::PluginsAddFromFolderTitle),
        ] {
            let mut app = app_with_add_menu();

            let _ = update_input(&mut app, AppEvent::Plugins(choice));

            assert!(!app.sidebar.has_header_menu_open(), "{title:?}");
            assert!(app.plugins.has_dialog(), "{title:?}");
            assert!(simulator(view(&app)).find(i18n::t(title)).is_ok());
        }
    }

    /// Esc and a press away from the menu close it, as the other section
    /// menus, and open no dialog.
    #[test]
    fn esc_or_a_press_away_closes_the_add_plugin_menu() {
        let mut esc = app_with_add_menu();
        let mut away = app_with_add_menu();
        let mut search = app_with_add_menu();

        let _ = update_input(&mut esc, AppEvent::Keyboard(escape_press()));
        let _ = update_input(
            &mut away,
            AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss),
        );
        let _ = update_input(
            &mut search,
            AppEvent::Plugins(PluginsIntent::SearchChanged(String::from("ex"))),
        );

        for app in [esc, away, search] {
            assert!(!app.sidebar.has_header_menu_open());
            assert!(!app.plugins.has_dialog());
        }
    }

    #[test]
    fn the_right_panel_of_plugins_mode_is_hidden() {
        let app = plugins_app();

        assert!(!app.sidebar.vm().is_right_open);
    }
}
