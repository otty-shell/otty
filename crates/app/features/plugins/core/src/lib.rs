//! The Plugins page of the OTTY app (spec 2026-11-19-agent-plugins,
//! Decision 12, "Install and update flow", "UI changes"): a wide dialog
//! over every mode, opened from Settings and from the links and needs of
//! Agents mode, with the installed plugins and a switch for each of their
//! parts, Add plugin… by a GitHub link, Add plugin from folder…, the
//! install dialog with the permissions of each part, the update dialog
//! with the permission changes, and the daily update check. Two plugins
//! may serve one id: every part that is on reads, and nothing is asked
//! (issue 29).
//!
//! The page touches no network and no disk. It reports what it needs as a
//! [`PluginsOutcome`]; `otty-app-plugins-io` does it and answers with a
//! [`PluginsIntent`]. When the installed plugins change, the page reports
//! [`PluginsOutcome::LoadParts`] and the binary loads the parts that are
//! on into Agents mode, and [`PluginsOutcome::Notes`] with what Agents
//! mode shows of the plugins: the parts that are off and the updates that
//! wait.
//!
//! # Example
//!
//! ```
//! use std::path::PathBuf;
//!
//! use otty_app_domain::plugins::PluginsPlace;
//! use otty_app_plugins::{Plugins, PluginsIntent, PluginsOutcome};
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
//! assert_eq!(opened[0], PluginsOutcome::Opened);
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
use std::time::Duration;

pub use event::{ChangeAnswer, PluginsIntent, PluginsOutcome};
use iced::Subscription;
use iced::widget::Id;
use model::PageViewModel;
use otty_app_domain::plugins::PluginNotes;
use otty_app_slots::{Component, Overlay, Overlays};
use otty_app_theme::theme::ThemeProps;
use state::PluginsState;
pub use types::{ListedPlugin, PartsToLoad, RunningPlugin};

/// The id of the GitHub link field of Add plugin…; the binary moves the
/// keyboard focus to it when the page opens there.
pub const URL_FIELD: Id = view::URL_FIELD;

/// How often the page asks whether the daily update check is due.
const UPDATE_TICK: Duration = Duration::from_secs(60 * 60);

/// The Plugins page: the installed plugins, the parts that run, what the
/// update checks said, and the dialog while it is open.
pub struct Plugins {
    state: PluginsState,
}

impl Plugins {
    /// Whether the page is open.
    pub fn is_open(&self) -> bool {
        self.state.is_open()
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

    /// The page closed, with the installed plugins `plugins` as
    /// `otty-app-plugins-io` read them at start; `~` in a typed folder
    /// stands for `home`.
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

    /// The timer of the daily update check: once an hour the page asks
    /// whether a day has passed since the last check.
    fn subscription(&self) -> Subscription<PluginsIntent> {
        iced::time::every(UPDATE_TICK).map(|_| PluginsIntent::UpdateTick)
    }
}

impl Overlays for Plugins {
    /// The open page: a wide dialog in the middle of the window, over a
    /// dimmed layer. Esc and the close cross close it.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, PluginsIntent>> {
        let vm = PageViewModel::from_state(&self.state)?;

        Some(Overlay::dialog(view::view(view::PageProps { vm, theme })))
    }
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

    #[test]
    fn the_open_page_is_a_dialog_with_its_plugins() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut plugins =
            Plugins::new(vec![official_example()], PathBuf::from("/home/user"));
        let closed = plugins.overlay(theme).is_none();

        let _ = plugins.reduce(PluginsIntent::Open(PluginsPlace::List), ());

        let page = plugins.overlay(theme).expect("the page");
        assert!(closed);
        assert_eq!(page.kind(), OverlayKind::Dialog);
        let mut ui = simulator(page.into_content());
        assert!(ui.find(i18n::t(Key::PluginsTitle)).is_ok());
        assert!(ui.find("Example").is_ok());
        assert!(ui.find(i18n::t(Key::PluginsOfficial)).is_ok());
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
