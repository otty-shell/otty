//! The menu of a plugin in the left panel (issue 37): the actions of its
//! page, Reload for a local plugin or Check for updates for one from a
//! repository, Reinstall for a broken one from a repository, Review update… while an update waits, Show log and
//! Remove….

use otty_plugin_releases::Trust;
use otty_ui_kit::components::context_menu::MenuPlace;

use super::broken_reason;
use crate::state::PluginsState;

/// What the open menu of a plugin shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PluginMenuViewModel {
    /// The plugin id.
    pub(crate) id: String,
    /// Whether it is a local plugin: Reload in place of Check for updates.
    pub(crate) is_local: bool,
    /// Whether it is broken: one from a repository reinstalls in place of
    /// Check for updates.
    pub(crate) is_broken: bool,
    /// Whether an update waits for the user's OK: Review update….
    pub(crate) has_waiting_update: bool,
    /// Whether a change runs: the items that change the plugin do nothing.
    pub(crate) is_busy: bool,
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
}

impl PluginMenuViewModel {
    /// The open menu of `state`; `None` while none is open, or when its
    /// plugin is gone.
    pub(crate) fn from_state(state: &PluginsState) -> Option<Self> {
        let menu = state.menu()?;
        let id = menu.plugin();
        let plugin = state.plugin(id)?;

        Some(Self {
            id: id.to_string(),
            is_local: plugin.installed.trust() == Trust::Local,
            is_broken: broken_reason(state, plugin).is_some(),
            has_waiting_update: state.waiting_update(id).is_some(),
            is_busy: state.is_busy(),
            place: menu.place(),
        })
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use otty_plugin_releases::fixtures::{example_manifest, pending_update};

    use super::*;
    use crate::state::{PluginMenu, PluginNews};
    use crate::testing::{local_example, released_example};

    fn place() -> MenuPlace {
        MenuPlace::new(Point::new(40.0, 120.0), Size::new(1024.0, 768.0))
    }

    fn with_menu(state: &mut PluginsState) {
        state.open_menu(PluginMenu::new(String::from("example"), place()));
    }

    #[test]
    fn no_open_menu_shows_nothing() {
        let state = PluginsState::new(vec![released_example()]);

        assert!(PluginMenuViewModel::from_state(&state).is_none());
    }

    /// A local plugin reloads; one from a repository checks for updates.
    #[test]
    fn the_menu_knows_a_local_plugin() {
        let mut local = PluginsState::new(vec![local_example()]);
        with_menu(&mut local);
        let mut released = PluginsState::new(vec![released_example()]);
        with_menu(&mut released);

        let local = PluginMenuViewModel::from_state(&local).expect("menu");
        let released =
            PluginMenuViewModel::from_state(&released).expect("menu");

        assert!(local.is_local);
        assert!(!released.is_local);
        assert_eq!(released.place, place());
        assert!(!released.has_waiting_update);
    }

    /// Issue 17 of spec 2026-11-23-accounts-apart-from-profiles: the menu
    /// knows a broken plugin, which reinstalls.
    #[test]
    fn the_menu_knows_a_broken_plugin() {
        let mut broken =
            PluginsState::new(vec![crate::testing::broken_local_example()]);
        with_menu(&mut broken);
        let mut healthy = PluginsState::new(vec![local_example()]);
        with_menu(&mut healthy);

        let broken = PluginMenuViewModel::from_state(&broken).expect("menu");
        let healthy = PluginMenuViewModel::from_state(&healthy).expect("menu");

        assert!(broken.is_broken);
        assert!(!healthy.is_broken);
    }

    #[test]
    fn the_menu_knows_a_waiting_update() {
        let plugin = released_example();
        let update =
            pending_update(&plugin.installed, &example_manifest("0.2.0"));
        let mut state = PluginsState::new(vec![plugin]);
        state.set_news("example", PluginNews::Waiting(Box::new(update)));
        with_menu(&mut state);

        let vm = PluginMenuViewModel::from_state(&state).expect("menu");

        assert!(vm.has_waiting_update);
    }

    /// While a change runs the items that change the plugin do nothing.
    #[test]
    fn the_menu_knows_a_change_runs() {
        let mut state = PluginsState::new(vec![released_example()]);
        with_menu(&mut state);
        state.set_busy(true);

        let vm = PluginMenuViewModel::from_state(&state).expect("menu");

        assert!(vm.is_busy);
    }
}
