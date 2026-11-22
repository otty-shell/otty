//! Plugins mode of the OTTY app (spec 2026-11-19-agent-plugins,
//! Decision 12, "Install and update flow", "UI changes"; issue 30): the
//! installed plugins in the left panel with a search by name, Add
//! plugin… by a GitHub link and, in the menu under its chevron, Add
//! plugin from folder…; the page of the
//! selected plugin in the main area with a switch for each of its parts,
//! their permissions and health; the install dialog with the permissions
//! of each part, the update dialog with the permission changes, and the
//! daily update check. Settings and the links and needs of Agents mode
//! open the mode at a [`PluginsPlace`]. Two plugins may serve one id:
//! every part that is on reads, and nothing is asked (issue 29).
//!
//! The mode touches no network and no disk. It reports what it needs as
//! a [`PluginsOutcome`]; `otty-app-plugins-io` does it and answers with a
//! [`PluginsIntent`]. When the installed plugins change, the mode reports
//! [`PluginsOutcome::LoadParts`] and the binary loads the parts that are
//! on into Agents mode, and [`PluginsOutcome::Notes`] with what Agents
//! mode shows of the plugins: the parts that are off and the updates that
//! wait.
//!
//! [`PluginsPlace`]: otty_app_domain::plugins::PluginsPlace
//!
//! # Example
//!
//! ```
//! use std::path::PathBuf;
//!
//! use otty_app_domain::plugins::PluginsPlace;
//! use otty_app_plugins::{Plugins, PluginsIntent, PluginsOutcome, URL_FIELD};
//! use otty_app_slots::Component;
//!
//! let mut plugins = Plugins::new(Vec::new(), PathBuf::from("/home/user"));
//!
//! let opened =
//!     plugins.reduce(PluginsIntent::Open(PluginsPlace::AddPlugin), ());
//! let _ = plugins.reduce(
//!     PluginsIntent::UrlChanged(String::from(
//!         "https://github.com/otty-shell/otty-plugin-example",
//!     )),
//!     (),
//! );
//! let find = plugins.reduce(PluginsIntent::Find, ());
//!
//! assert_eq!(opened, vec![PluginsOutcome::FocusField(URL_FIELD)]);
//! assert!(plugins.has_dialog());
//! assert_eq!(
//!     find,
//!     vec![PluginsOutcome::FindRelease(String::from(
//!         "https://github.com/otty-shell/otty-plugin-example"
//!     ))]
//! );
//! ```

mod event;
mod model;
mod notes;
mod reducer;
mod state;
#[cfg(test)]
mod testing;
mod types;
mod view;
mod words;

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub use event::{ChangeAnswer, PluginsIntent, PluginsOutcome};
use iced::widget::Id;
use iced::{Element, Subscription};
use model::{DialogViewModel, MainViewModel, SidePanelViewModel};
use otty_app_domain::plugins::PluginNotes;
use otty_app_slots::{
    Component, MainArea, MainContent, Overlay, Overlays, SectionMessage,
    SectionState,
};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::Menu;
use state::PluginsState;
pub use types::{ListedPlugin, PartsToLoad, RunningPlugin};

/// The id of the GitHub link field of Add plugin…; the binary moves the
/// keyboard focus to it when the dialog opens.
pub const URL_FIELD: Id = view::URL_FIELD;

/// How often the mode asks whether the daily update check is due.
const UPDATE_TICK: Duration = Duration::from_secs(60 * 60);

/// Plugins mode: the installed plugins, the parts that run, what the
/// update checks said, the selected plugin, the search and the dialog
/// while it is open.
pub struct Plugins {
    state: PluginsState,
}

impl Plugins {
    /// Whether a dialog of the mode is open: the install or update dialog,
    /// Add plugin…, the log or the question before Remove.
    pub fn has_dialog(&self) -> bool {
        self.state.dialog().is_some()
    }

    /// What Agents mode shows of the installed plugins: the parts that are
    /// off and the updates that wait.
    pub fn notes(&self) -> PluginNotes {
        notes::notes(&self.state)
    }

    /// The parts that are on of the installed plugins, to load into Agents
    /// mode at start.
    pub fn parts_to_load(&self) -> Vec<PartsToLoad> {
        self.state
            .plugins()
            .iter()
            .map(ListedPlugin::parts_to_load)
            .collect()
    }

    /// Left panel of the mode under the switcher: the installed plugins,
    /// the search and the Add plugin split button. Its plus sends
    /// [`PluginsIntent::AddPlugin`]; its chevron sends
    /// [`SectionMessage::OpenMenu`], and the frame opens
    /// [`Plugins::add_menu`] and keeps whether it is open in `section`.
    pub fn left_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
        section: SectionState,
    ) -> Element<'a, SectionMessage<PluginsIntent>> {
        let vm = SidePanelViewModel::from_state(&self.state);

        view::side_panel::view(vm, section, theme.theme.ui())
    }

    /// The menu under the chevron of Add plugin: Add plugin… by a GitHub
    /// link and Add plugin from folder…. The frame opens, places and
    /// closes it, as the menus of the other section headers.
    pub fn add_menu(&self, theme: ThemeProps<'_>) -> Menu<PluginsIntent> {
        view::add_menu::menu(theme.theme.ui())
    }

    /// No dialog and nothing selected, with the installed plugins
    /// `plugins` as `otty-app-plugins-io` read them at start; `~` in a
    /// typed folder stands for `home`.
    pub fn new(plugins: Vec<ListedPlugin>, home: PathBuf) -> Self {
        Self {
            state: PluginsState::new(plugins).with_home(home),
        }
    }
}

impl Component for Plugins {
    type Intent = PluginsIntent;
    type Outcome = PluginsOutcome;
    type Ctx<'a> = ();

    fn reduce(
        &mut self,
        intent: PluginsIntent,
        _ctx: (),
    ) -> Vec<PluginsOutcome> {
        reducer::reduce(&mut self.state, intent)
    }

    /// The timer of the daily update check: once an hour the mode asks
    /// whether a day has passed since the last check.
    fn subscription(&self) -> Subscription<PluginsIntent> {
        iced::time::every(UPDATE_TICK).map(|_| PluginsIntent::UpdateTick)
    }
}

impl MainArea for Plugins {
    type Screen = PluginsScreen;

    /// The page of the selected plugin, or the card that says what to do;
    /// the ages of the last failures count up to the current time.
    fn main_area<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> MainContent<'a, PluginsIntent, PluginsScreen> {
        let vm = MainViewModel::from_state(&self.state, SystemTime::now());
        let id = match &vm {
            MainViewModel::Hint { .. } => PluginsScreen::Hint,
            MainViewModel::Plugin(page) => {
                PluginsScreen::Plugin(page.id.clone())
            },
        };

        MainContent::Screen {
            id,
            content: view::details::view(vm, theme.theme.ui()),
        }
    }
}

impl Overlays for Plugins {
    /// The open dialog, in the middle of the window over a dimmed layer.
    /// Esc and Cancel close it.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, PluginsIntent>> {
        let vm = DialogViewModel::from_state(&self.state)?;

        Some(Overlay::dialog(view::dialog(vm, theme.theme.ui())))
    }
}

/// Which screen fills the main area of Plugins mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginsScreen {
    /// The card that says what to do.
    Hint,
    /// The page of the plugin with this id.
    Plugin(String),
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_domain::plugins::PluginsPlace;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_slots::OverlayKind;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::testing::official_example;

    /// Issue 30: the left panel lists the plugins and the main area shows
    /// the page of the selected one.
    #[test]
    fn the_left_panel_lists_the_plugins_and_the_main_area_the_selected() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut plugins =
            Plugins::new(vec![official_example()], PathBuf::from("/home/user"));
        let _ = plugins.reduce(
            PluginsIntent::Open(PluginsPlace::Plugin(String::from("example"))),
            (),
        );

        let mut panel = simulator(
            plugins.left_panel(theme, SectionState::new(true, false)),
        );
        let mut main = simulator(plugins.main_area(theme).into_element(theme));

        assert!(panel.find("Example").is_ok());
        assert!(main.find(i18n::t(Key::PluginsOfficialNote)).is_ok());
        assert!(plugins.overlay(theme).is_none());
    }

    /// A dialog of the mode lies over the window.
    #[test]
    fn a_dialog_of_the_mode_is_an_overlay() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut plugins = Plugins::new(Vec::new(), PathBuf::from("/home/user"));

        let _ = plugins.reduce(PluginsIntent::AddPlugin, ());

        let dialog = plugins.overlay(theme).expect("the dialog");
        assert!(plugins.has_dialog());
        assert_eq!(dialog.kind(), OverlayKind::Dialog);
        let mut ui = simulator(dialog.into_content());
        assert!(ui.find(i18n::t(Key::PluginsAddPluginTitle)).is_ok());
    }

    #[test]
    fn the_parts_that_are_on_are_loaded_at_start() {
        let plugins =
            Plugins::new(vec![official_example()], PathBuf::from("/home/user"));

        let parts = plugins.parts_to_load();

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].on.len(), 2);
        assert!(plugins.notes().off_parts.is_empty());
    }
}
