//! The left panel of Plugins mode (issue 30): the installed plugins whose
//! names hold the search, each with its icon, name, the mark of who
//! reviewed it and a sign when it is off, waits for an update or failed.
//! A broken plugin has the broken plugin icon (issue 17 of spec
//! 2026-11-23-accounts-apart-from-profiles).

use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::broken_reason;
use crate::state::{PluginNews, PluginsState};
use crate::types::ListedPlugin;
use crate::words::trust_text;

/// What the left panel shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SidePanelViewModel<'a> {
    /// The text of the search field.
    pub(crate) search: &'a str,
    /// The plugins whose names hold the search, in their order.
    pub(crate) rows: Vec<PluginListRowViewModel>,
    /// How many plugins are installed.
    pub(crate) count: usize,
    /// "No plugin matches the search" when the search hides every
    /// installed plugin.
    pub(crate) note: Option<&'static str>,
}

impl<'a> SidePanelViewModel<'a> {
    /// The left panel of `state`.
    pub(crate) fn from_state(state: &'a PluginsState) -> Self {
        let search = state.search();

        let rows: Vec<PluginListRowViewModel> = state
            .plugins()
            .iter()
            .filter(|plugin| name_holds(plugin.installed.name(), search))
            .map(|plugin| row(state, plugin))
            .collect();
        let count = state.plugins().len();
        let note = (rows.is_empty() && count > 0)
            .then(|| i18n::t(Key::PluginsNoMatch));

        Self {
            search,
            rows,
            count,
            note,
        }
    }
}

/// One plugin of the left panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginListRowViewModel {
    /// The plugin id.
    pub(crate) id: String,
    /// Its name.
    pub(crate) name: String,
    /// "Official", "Community" or "Local".
    pub(crate) trust: &'static str,
    /// The SVG of its icon, or `None` for a letter icon.
    pub(crate) icon: Option<Vec<u8>>,
    /// Whether it is broken: the broken plugin icon in place of its own.
    pub(crate) is_broken: bool,
    /// What needs a look, if anything.
    pub(crate) sign: Option<PluginSign>,
    /// Whether the main area shows its page.
    pub(crate) is_selected: bool,
}

impl PluginListRowViewModel {
    /// The words on the right of the row: the mark and the sign side by
    /// side, such as "Official, failed".
    pub(crate) fn note(&self) -> String {
        match self.sign {
            Some(sign) => i18n::filled(
                Key::TplPluginsTrustAndSign,
                &[("trust", self.trust), ("sign", sign.word())],
            ),
            None => self.trust.to_string(),
        }
    }

    /// The color of the words on the right of the row.
    pub(crate) fn tone(&self) -> SessionTone {
        self.sign.map_or(SessionTone::Muted, PluginSign::tone)
    }
}

/// What needs a look on a plugin of the left panel. One shows, the first
/// of failed, update waiting and off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginSign {
    /// The plugin is broken, a part is paused after three failures, or
    /// the latest check or change failed.
    Failed,
    /// A newer version waits for the user's OK.
    UpdateWaits,
    /// Every part is off.
    Off,
}

impl PluginSign {
    /// The sign in the user's language, such as "failed".
    pub(crate) fn word(self) -> &'static str {
        let key = match self {
            Self::Failed => Key::PluginsSignFailed,
            Self::UpdateWaits => Key::PluginsSignUpdate,
            Self::Off => Key::PluginsSignOff,
        };

        i18n::t(key)
    }

    /// The color of the sign.
    pub(crate) fn tone(self) -> SessionTone {
        match self {
            Self::Failed => SessionTone::Failed,
            Self::UpdateWaits => SessionTone::Waiting,
            Self::Off => SessionTone::Muted,
        }
    }
}

/// Whether `name` holds `search`, letter case aside. An empty search
/// holds every name.
fn name_holds(name: &str, search: &str) -> bool {
    let search = search.trim().to_lowercase();

    name.to_lowercase().contains(&search)
}

fn row(state: &PluginsState, plugin: &ListedPlugin) -> PluginListRowViewModel {
    let id = plugin.id();
    let is_broken = broken_reason(state, plugin).is_some();

    PluginListRowViewModel {
        id: id.to_string(),
        name: plugin.installed.name().to_string(),
        trust: trust_text(plugin.installed.trust()),
        icon: plugin.icon.clone(),
        is_broken,
        sign: sign(state, plugin, is_broken),
        is_selected: state.selected() == Some(id),
    }
}

/// What needs a look on `plugin`, if anything; a broken one failed.
fn sign(
    state: &PluginsState,
    plugin: &ListedPlugin,
    is_broken: bool,
) -> Option<PluginSign> {
    let id = plugin.id();

    let is_paused = state.running_plugin(id).is_some_and(|running| {
        running.parts.iter().any(|(_, health)| health.is_paused())
    });
    let has_failed_news =
        matches!(state.news_of(id), Some(PluginNews::Failed(_)));
    if is_broken || is_paused || has_failed_news {
        return Some(PluginSign::Failed);
    }
    if state.waiting_update(id).is_some() {
        return Some(PluginSign::UpdateWaits);
    }
    let parts = plugin.installed.parts();
    if !parts.is_empty() && parts.iter().all(|part| !part.is_on()) {
        return Some(PluginSign::Off);
    }

    None
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::fixtures::paused_health;
    use otty_plugin_host::{PartHealth, PluginLog};
    use otty_plugin_releases::fixtures::{
        example_manifest, pending_update, with_part_on,
    };

    use super::*;
    use crate::testing::{
        agent, fork, local_example, official_example, provider,
        released_example,
    };
    use crate::types::RunningPlugin;

    fn names(vm: &SidePanelViewModel<'_>) -> Vec<String> {
        vm.rows.iter().map(|row| row.name.clone()).collect()
    }

    /// Issue 30: the search keeps the plugins whose names hold it, letter
    /// case aside.
    #[test]
    fn the_search_filters_by_name_ignoring_case() {
        let mut state = PluginsState::new(vec![official_example(), fork()]);

        state.set_search(String::from("FORK"));
        let fork_only = names(&SidePanelViewModel::from_state(&state));
        state.set_search(String::from(" example "));
        let both = names(&SidePanelViewModel::from_state(&state));

        assert_eq!(fork_only, ["Example Fork"]);
        assert_eq!(both, ["Example", "Example Fork"]);
    }

    /// The search looks at the names only, not the ids or the marks.
    #[test]
    fn the_search_does_not_look_at_the_marks() {
        let mut state = PluginsState::new(vec![official_example()]);
        state.set_search(String::from("official"));

        let vm = SidePanelViewModel::from_state(&state);

        assert!(vm.rows.is_empty());
        assert_eq!(vm.count, 1);
        assert_eq!(vm.note, Some(i18n::t(Key::PluginsNoMatch)));
    }

    /// With nothing installed the panel has no rows and no note: the main
    /// area says what to do.
    #[test]
    fn no_plugin_installed_has_no_rows_and_no_note() {
        let state = PluginsState::new(Vec::new());

        let vm = SidePanelViewModel::from_state(&state);

        assert!(vm.rows.is_empty());
        assert_eq!(vm.note, None);
    }

    #[test]
    fn a_row_shows_its_mark_and_whether_it_is_selected() {
        let mut state =
            PluginsState::new(vec![official_example(), local_example()]);
        state.select(Some(String::from("example")));

        let vm = SidePanelViewModel::from_state(&state);

        assert_eq!(vm.rows[0].trust, i18n::t(Key::PluginsOfficial));
        assert!(vm.rows[0].is_selected);
        assert_eq!(vm.rows[0].note(), i18n::t(Key::PluginsOfficial));
        assert_eq!(vm.rows[1].trust, i18n::t(Key::PluginsLocal));
    }

    /// A plugin with every part off is marked off; one part off is not.
    #[test]
    fn a_plugin_with_every_part_off_is_marked_off() {
        let mut one_off = official_example();
        one_off.installed = with_part_on(one_off.installed, &agent(), false);
        let mut all_off = fork();
        all_off.installed = with_part_on(all_off.installed, &agent(), false);
        all_off.installed = with_part_on(all_off.installed, &provider(), false);
        let state = PluginsState::new(vec![one_off, all_off]);

        let vm = SidePanelViewModel::from_state(&state);

        assert_eq!(vm.rows[0].sign, None);
        assert_eq!(vm.rows[1].sign, Some(PluginSign::Off));
        // Issue 16 of spec 2026-11-23-accounts-apart-from-profiles: the
        // mark and the sign side by side take a comma, no middle dot.
        assert_eq!(
            vm.rows[1].note(),
            i18n::filled(
                Key::TplPluginsTrustAndSign,
                &[
                    ("trust", i18n::t(Key::PluginsCommunity)),
                    ("sign", i18n::t(Key::PluginsSignOff)),
                ]
            )
        );
        assert!(!vm.rows[1].note().contains('·'));
    }

    #[test]
    fn a_waiting_update_is_signed_on_its_row() {
        let plugin = released_example();
        let update =
            pending_update(&plugin.installed, &example_manifest("0.2.0"));
        let mut state = PluginsState::new(vec![plugin]);
        state.set_news("example", PluginNews::Waiting(Box::new(update)));

        let vm = SidePanelViewModel::from_state(&state);

        assert_eq!(vm.rows[0].sign, Some(PluginSign::UpdateWaits));
        assert_eq!(vm.rows[0].tone(), SessionTone::Waiting);
    }

    /// A paused part or a failed check marks the plugin failed, before
    /// anything else.
    #[test]
    fn a_paused_part_or_a_failed_check_is_signed_failed() {
        let mut state = PluginsState::new(vec![official_example(), fork()]);
        state.set_running(vec![RunningPlugin {
            plugin: String::from("example"),
            log: PluginLog::new(),
            parts: vec![
                (agent(), paused_health()),
                (provider(), PartHealth::default()),
            ],
        }]);
        state.set_news(
            "example-fork",
            PluginNews::Failed(String::from("GitHub refuses")),
        );

        let vm = SidePanelViewModel::from_state(&state);

        assert_eq!(vm.rows[0].sign, Some(PluginSign::Failed));
        assert_eq!(vm.rows[1].sign, Some(PluginSign::Failed));
        assert_eq!(vm.rows[1].tone(), SessionTone::Failed);
    }

    /// Issue 17 of spec 2026-11-23-accounts-apart-from-profiles: a plugin
    /// whose installed manifest can not be read, or that failed to load,
    /// has the broken plugin icon and is signed failed; a healthy one
    /// keeps its own icon.
    #[test]
    fn a_broken_plugin_has_the_broken_icon_and_is_signed_failed() {
        let mut healthy = fork();
        healthy.icon = Some(b"<svg/>".to_vec());
        let unread = PluginsState::new(vec![
            crate::testing::broken_local_example(),
            healthy,
        ]);
        let mut unloaded = PluginsState::new(vec![released_example()]);
        unloaded.set_load_failures(vec![crate::testing::load_failure(
            "example",
            "The plugin's .wasm can not be read: no file",
        )]);

        let unread = SidePanelViewModel::from_state(&unread);
        let unloaded = SidePanelViewModel::from_state(&unloaded);

        assert!(unread.rows[0].is_broken);
        assert_eq!(unread.rows[0].sign, Some(PluginSign::Failed));
        assert!(!unread.rows[1].is_broken);
        assert_eq!(unread.rows[1].sign, None);
        assert_eq!(unread.rows[1].icon, Some(b"<svg/>".to_vec()));
        assert!(unloaded.rows[0].is_broken);
        assert_eq!(unloaded.rows[0].sign, Some(PluginSign::Failed));
    }
}
