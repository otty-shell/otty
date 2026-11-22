//! The main area of Plugins mode (issue 30): the page of the selected
//! plugin, with what it is (its name, version, authors and source), a tag
//! of who reviewed it (issue 36), its parts as cards with a switch each,
//! their kind, description, health, last failed call and how many places
//! they can access (issues 35, 36), an update that waits and its actions;
//! or a short line that says what to do when nothing is selected or
//! nothing is installed. What Reload and Check for updates did is a
//! notification, not a line of the page (issue 35).

use std::time::SystemTime;

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_host::{LastFailure, PluginFailureKind};
use otty_plugin_manifest::PartRef;
use otty_plugin_releases::{Source, Trust};

use super::permissions::granted_groups;
use crate::state::{PluginNews, PluginsState};
use crate::types::{ListedPlugin, part_description};
use crate::words::{error_text, kind_text, trust_note, trust_text};

/// What fills the main area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MainViewModel {
    /// Nothing to show: a title and a short line that says what to do.
    Hint {
        /// "No plugins are installed" or "No plugin selected".
        title: &'static str,
        /// What to do.
        line: &'static str,
    },
    /// The page of the selected plugin.
    Plugin(Box<PluginPageViewModel>),
}

impl MainViewModel {
    /// What fills the main area of `state`, with the ages of the last
    /// failures counted at `now`.
    pub(crate) fn from_state(state: &PluginsState, now: SystemTime) -> Self {
        if state.plugins().is_empty() {
            return Self::Hint {
                title: i18n::t(Key::PluginsEmpty),
                line: i18n::t(Key::PluginsEmptyHint),
            };
        }

        match state.selected().and_then(|id| state.plugin(id)) {
            Some(plugin) => Self::Plugin(Box::new(page(state, plugin, now))),
            None => Self::Hint {
                title: i18n::t(Key::PluginsNoneSelected),
                line: i18n::t(Key::PluginsPickHint),
            },
        }
    }
}

/// The page of one plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginPageViewModel {
    /// The plugin id.
    pub(crate) id: String,
    /// Its name.
    pub(crate) name: String,
    /// Its installed version.
    pub(crate) version: String,
    /// What it says it does, from its manifest.
    pub(crate) description: Option<String>,
    /// The tag of who reviewed it, after the version.
    pub(crate) trust: TrustTagViewModel,
    /// The SVG of its icon, or `None` for a letter icon.
    pub(crate) icon: Option<Vec<u8>>,
    /// Its authors, from its manifest.
    pub(crate) authors: Option<String>,
    /// Where it came from.
    pub(crate) source: SourceViewModel,
    /// Its parts, each with its switch.
    pub(crate) parts: Vec<PartRowViewModel>,
    /// The line of an update that waits for the user's OK, with Review.
    pub(crate) waiting: Option<String>,
    /// "0.6.0 needs a newer OTTY".
    pub(crate) too_new: Option<String>,
    /// Whether it is a local plugin: Reload in place of Check for updates.
    pub(crate) is_local: bool,
    /// Why the latest switch failed.
    pub(crate) error: Option<String>,
    /// Whether a change runs: the switches do nothing.
    pub(crate) is_busy: bool,
}

/// The tag of who reviewed a plugin, such as "Official", with its note in
/// the tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrustTagViewModel {
    /// "Official", "Community" or "Local".
    pub(crate) label: &'static str,
    /// "Official plugin: reviewed by the OTTY team".
    pub(crate) note: &'static str,
    /// Whether the tag has the warn tone: a community plugin is reviewed
    /// by its author only.
    pub(crate) is_warn: bool,
}

/// Where a plugin came from, the Source fact of its page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceViewModel {
    /// The folder on this computer it was installed from.
    Folder(String),
    /// The link of the repository it was installed from; a press opens it.
    Repository(String),
}

/// One part of a plugin as a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartRowViewModel {
    /// The part.
    pub(crate) part: PartRef,
    /// Its name, such as "Coder".
    pub(crate) name: String,
    /// Its kind for its tag: "Agent" or "Provider".
    pub(crate) kind: &'static str,
    /// What it does, one sentence from the manifest.
    pub(crate) description: Option<String>,
    /// Whether it is on.
    pub(crate) is_on: bool,
    /// Its health badge: "Working", "Paused" or "Off".
    pub(crate) health: &'static str,
    /// Whether it is paused after three failures in a row: Try again.
    pub(crate) is_paused: bool,
    /// How many places the user let it access: Can access (5) opens
    /// them; none, and it asks for no permissions.
    pub(crate) places: usize,
    /// The last call of the part that failed or that the host refused,
    /// for plugin authors (issue 19).
    pub(crate) last_failure: Option<LastFailureViewModel>,
}

/// The last failed call of a part: when, which call and the kind of
/// error, never a value, a key or session text, and its report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LastFailureViewModel {
    /// "Last failure 5 min ago: numbers, the plugin did not finish in
    /// time".
    pub(crate) text: String,
    /// The report Copy report puts on the clipboard.
    pub(crate) report: String,
}

/// The reason of `error` for the page of a plugin.
pub(crate) fn failed(error: &otty_plugin_releases::InstallError) -> PluginNews {
    PluginNews::Failed(error_text(error))
}

/// The page of `plugin`, with the ages of the last failures counted at
/// `now`.
fn page(
    state: &PluginsState,
    plugin: &ListedPlugin,
    now: SystemTime,
) -> PluginPageViewModel {
    let installed = &plugin.installed;
    let id = plugin.id();
    let manifest = plugin.manifest.as_ref();

    let parts = installed
        .parts()
        .iter()
        .map(|part| {
            let reference = part.part();
            let is_paused = is_paused(state, id, &reference);
            let name = plugin.part_name(&reference);
            let places = granted_groups(part.granted(), manifest, &name)
                .iter()
                .map(|group| group.places.len())
                .sum();
            PartRowViewModel {
                kind: kind_text(&reference),
                description: part_description(manifest, &reference),
                is_on: part.is_on(),
                health: health_text(part.is_on(), is_paused),
                is_paused,
                places,
                name,
                last_failure: last_failure(state, id, &reference)
                    .map(|last| last_failure_line(&last, now)),
                part: reference,
            }
        })
        .collect();
    let too_new = state.too_new_of(id).map(|version| {
        i18n::filled(Key::TplPluginsTooNew, &[("version", version)])
    });
    let waiting = state.waiting_update(id).map(|update| {
        i18n::filled(
            Key::TplPluginsUpdateWaits,
            &[("version", &update.to().to_string())],
        )
    });
    let authors = manifest
        .map(|manifest| manifest.plugin().authors().join(", "))
        .filter(|authors| !authors.is_empty());
    let description = manifest
        .and_then(|manifest| manifest.plugin().description())
        .map(String::from);
    // An error while a dialog is open belongs to the dialog.
    let error = state
        .error()
        .filter(|_| state.dialog().is_none())
        .map(String::from);

    PluginPageViewModel {
        id: id.to_string(),
        name: installed.name().to_string(),
        version: installed.version().to_string(),
        description,
        trust: TrustTagViewModel {
            label: trust_text(installed.trust()),
            note: trust_note(installed.trust()),
            is_warn: installed.trust() == Trust::Community,
        },
        icon: plugin.icon.clone(),
        authors,
        source: source_of(installed.source()),
        parts,
        waiting,
        too_new,
        is_local: installed.trust() == Trust::Local,
        error,
        is_busy: state.is_busy(),
    }
}

/// Where a plugin came from: the folder, or the link of the repository.
fn source_of(source: &Source) -> SourceViewModel {
    match source {
        Source::Release { url } => SourceViewModel::Repository(url.clone()),
        Source::Local { folder } => {
            SourceViewModel::Folder(folder.display().to_string())
        },
    }
}

/// The health badge of a part: off, paused after three failures, or
/// working.
fn health_text(is_on: bool, is_paused: bool) -> &'static str {
    let key = if !is_on {
        Key::PluginsHealthOff
    } else if is_paused {
        Key::PluginsHealthPaused
    } else {
        Key::PluginsHealthWorking
    };

    i18n::t(key)
}

/// Whether the part `part` of the plugin `id` is paused.
fn is_paused(state: &PluginsState, id: &str, part: &PartRef) -> bool {
    state.running_plugin(id).is_some_and(|running| {
        running
            .parts
            .iter()
            .any(|(loaded, health)| loaded == part && health.is_paused())
    })
}

/// The last failed call of the part `part` of the plugin `id`, while it
/// runs.
fn last_failure(
    state: &PluginsState,
    id: &str,
    part: &PartRef,
) -> Option<LastFailure> {
    let running = state.running_plugin(id)?;

    running
        .parts
        .iter()
        .find(|(loaded, _)| loaded == part)
        .and_then(|(_, health)| health.last_failure())
}

/// The line of `last`, as old as it is at `now`, with its report.
fn last_failure_line(
    last: &LastFailure,
    now: SystemTime,
) -> LastFailureViewModel {
    let failure = last.failure();
    let age = now.duration_since(last.at()).unwrap_or_default();
    let kind = match failure.kind() {
        PluginFailureKind::CouldNotRead(reason) => format!(
            "{} ({reason})",
            i18n::t(Key::AgentsPluginFailureCouldNotRead)
        ),
        kind => failure_kind_text(kind).to_string(),
    };

    LastFailureViewModel {
        text: i18n::filled(
            Key::TplPluginsLastFailure,
            &[
                ("time", &i18n::time_ago(age)),
                ("call", failure.call()),
                ("kind", &kind),
            ],
        ),
        report: failure.report(),
    }
}

/// The kind of a failed call in the user's language, such as "the plugin
/// did not finish in time".
fn failure_kind_text(kind: PluginFailureKind) -> &'static str {
    let key = match kind {
        PluginFailureKind::Stopped => Key::AgentsPluginFailureStopped,
        PluginFailureKind::TooSlow => Key::AgentsPluginFailureTooSlow,
        PluginFailureKind::Paused => Key::AgentsPluginFailurePaused,
        PluginFailureKind::CouldNotRead(_) => {
            Key::AgentsPluginFailureCouldNotRead
        },
    };

    i18n::t(key)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_plugin_host::fixtures::{failed_health, paused_health};
    use otty_plugin_host::{PartHealth, PluginFailure, PluginLog, PluginPart};
    use otty_plugin_manifest::Manifest;
    use otty_plugin_releases::fixtures::{example_manifest, with_part_on};

    use super::*;
    use crate::testing::{
        agent, local_example, official_example, provider, released_example,
    };
    use crate::types::RunningPlugin;

    /// `plugins` installed and `example` selected.
    fn selected(plugins: Vec<ListedPlugin>) -> PluginsState {
        let mut state = PluginsState::new(plugins);
        state.select(Some(String::from("example")));

        state
    }

    fn page_of(state: &PluginsState, now: SystemTime) -> PluginPageViewModel {
        match MainViewModel::from_state(state, now) {
            MainViewModel::Plugin(page) => *page,
            other => panic!("the page of a plugin: {other:?}"),
        }
    }

    /// Issue 30: with nothing installed, the main area says to add a
    /// plugin.
    #[test]
    fn with_nothing_installed_the_line_says_to_add_a_plugin() {
        let state = PluginsState::new(Vec::new());

        let vm = MainViewModel::from_state(&state, SystemTime::now());

        assert_eq!(
            vm,
            MainViewModel::Hint {
                title: i18n::t(Key::PluginsEmpty),
                line: i18n::t(Key::PluginsEmptyHint),
            }
        );
    }

    /// Issue 30: with plugins installed and none selected, the main area
    /// says to pick one.
    #[test]
    fn with_nothing_selected_the_line_says_to_pick_a_plugin() {
        let state = PluginsState::new(vec![official_example()]);

        let vm = MainViewModel::from_state(&state, SystemTime::now());

        assert_eq!(
            vm,
            MainViewModel::Hint {
                title: i18n::t(Key::PluginsNoneSelected),
                line: i18n::t(Key::PluginsPickHint),
            }
        );
    }

    /// Issue 30, 35, 36: the page says what the plugin is: its name,
    /// version, authors and source, and a tag of who reviewed it with its
    /// note in the tooltip. The source of a plugin from a repository is
    /// the repository.
    #[test]
    fn the_page_says_what_the_plugin_is() {
        let state = selected(vec![official_example()]);

        let page = page_of(&state, SystemTime::now());

        assert_eq!(page.name, "Example");
        assert_eq!(page.version, "0.1.0");
        assert_eq!(page.authors.as_deref(), Some("OTTY team"));
        assert_eq!(
            page.source,
            SourceViewModel::Repository(String::from(
                "https://github.com/otty-shell/otty-plugin-example"
            ))
        );
        assert_eq!(
            page.trust,
            TrustTagViewModel {
                label: i18n::t(Key::PluginsOfficial),
                note: i18n::t(Key::PluginsOfficialNote),
                is_warn: false,
            }
        );
        assert!(page.description.is_some());
        assert!(!page.is_local);
    }

    /// Owner, 2026-11-22: a community plugin is reviewed by its author
    /// only, so its tag has the warn tone.
    #[test]
    fn a_community_plugin_has_a_warn_tag() {
        let state = selected(vec![released_example()]);

        let page = page_of(&state, SystemTime::now());

        assert_eq!(
            page.trust,
            TrustTagViewModel {
                label: i18n::t(Key::PluginsCommunity),
                note: i18n::t(Key::PluginsCommunityNote),
                is_warn: true,
            }
        );
    }

    /// Issue 09, 35, 36: the page lists the parts with their kind,
    /// description, switch, health and how many places the user let them
    /// access.
    #[test]
    fn each_part_shows_its_kind_description_and_health() {
        let state = selected(vec![official_example()]);

        let page = page_of(&state, SystemTime::now());

        let names: Vec<(&str, &str)> = page
            .parts
            .iter()
            .map(|part| (part.name.as_str(), part.kind))
            .collect();
        assert_eq!(
            names,
            [
                ("Example Agent", i18n::t(Key::PluginsPartAgent)),
                ("Example Cloud", i18n::t(Key::PluginsPartProvider)),
            ]
        );
        assert!(page.parts.iter().all(|part| part.is_on));
        assert!(
            page.parts
                .iter()
                .all(|part| part.health == i18n::t(Key::PluginsHealthWorking))
        );
        assert_eq!(
            page.parts[0].description.as_deref(),
            Some("Finds Example Agent sessions, tokens and logins.")
        );
        assert_eq!(
            page.parts[1].description.as_deref(),
            Some("Reads the limits of Example Cloud accounts.")
        );
        // Two folders, a variable and two login files; a site, the
        // accounts and the keys of one host.
        assert_eq!(page.parts[0].places, 5);
        assert_eq!(page.parts[1].places, 3);
    }

    /// Issue 36: a part whose manifest says nothing of what it does shows
    /// no description.
    #[test]
    fn a_part_without_a_description_shows_none() {
        let text = example_manifest("0.1.0").replace(
            "description = \"Finds Example Agent sessions, tokens and logins.\"\n",
            "",
        );
        let mut plugin = official_example();
        plugin.manifest = Some(Manifest::parse(&text).expect("valid"));
        let state = selected(vec![plugin]);

        let page = page_of(&state, SystemTime::now());

        assert_eq!(page.parts[0].description, None);
        assert!(page.parts[1].description.is_some());
    }

    /// Issue 09, 35, 36: a local plugin shows its folder as the source,
    /// no row of the repository its manifest names, and Reload in place
    /// of Check for updates.
    #[test]
    fn a_local_plugin_shows_its_folder_and_reloads() {
        let state = selected(vec![local_example()]);

        let page = page_of(&state, SystemTime::now());

        assert_eq!(page.trust.label, i18n::t(Key::PluginsLocal));
        assert_eq!(page.trust.note, i18n::t(Key::PluginsLocalNote));
        assert_eq!(
            page.source,
            SourceViewModel::Folder(String::from("/work/otty-plugin-example"))
        );
        assert!(page.is_local);
    }

    #[test]
    fn a_part_that_is_off_shows_its_switch_off() {
        let mut plugin = official_example();
        plugin.installed = with_part_on(plugin.installed, &agent(), false);
        let state = selected(vec![plugin]);

        let page = page_of(&state, SystemTime::now());

        assert!(!page.parts[0].is_on);
        assert_eq!(page.parts[0].health, i18n::t(Key::PluginsHealthOff));
        assert!(page.parts[1].is_on);
    }

    /// Decision 8: a part paused after three failures in a row shows Try
    /// again; the plugin's other part does not.
    #[test]
    fn a_paused_part_is_marked_paused() {
        let mut state = selected(vec![official_example()]);
        state.set_running(vec![RunningPlugin {
            plugin: String::from("example"),
            log: PluginLog::new(),
            parts: vec![
                (agent(), paused_health()),
                (provider(), PartHealth::default()),
            ],
        }]);

        let page = page_of(&state, SystemTime::now());

        assert!(page.parts[0].is_paused);
        assert_eq!(page.parts[0].health, i18n::t(Key::PluginsHealthPaused));
        assert!(!page.parts[1].is_paused);
    }

    /// Issue 19: the card of a part shows its last failed call, with its
    /// time and kind and no value, and Copy report; the plugin's other
    /// part, which did not fail, shows none.
    #[test]
    fn a_part_shows_its_last_failed_call_and_its_report() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let failed_at = now - Duration::from_secs(5 * 60);
        let part = PluginPart::new(
            "example",
            "Example",
            "0.1.0",
            "https://github.com/otty-shell/otty-plugin-example",
            "example-agent",
        );
        let failure = PluginFailure::new(
            part,
            "numbers",
            PluginFailureKind::CouldNotRead("host error: not allowed"),
        );
        let mut state = selected(vec![official_example()]);
        state.set_running(vec![RunningPlugin {
            plugin: String::from("example"),
            log: PluginLog::new(),
            parts: vec![
                (agent(), failed_health(&failure, failed_at)),
                (provider(), PartHealth::default()),
            ],
        }]);

        let page = page_of(&state, now);

        let kind = format!(
            "{} (host error: not allowed)",
            i18n::t(Key::AgentsPluginFailureCouldNotRead)
        );
        assert_eq!(
            page.parts[0].last_failure,
            Some(LastFailureViewModel {
                text: i18n::filled(
                    Key::TplPluginsLastFailure,
                    &[
                        ("time", &i18n::time_ago(Duration::from_secs(300))),
                        ("call", "numbers"),
                        ("kind", &kind),
                    ],
                ),
                report: failure.report(),
            })
        );
        assert_eq!(page.parts[1].last_failure, None);
    }

    /// Issue 35: a release for a newer OTTY stays on the page; a failed
    /// check does not, it is a notification.
    #[test]
    fn a_release_for_a_newer_otty_stays_on_the_page() {
        let mut state = selected(vec![official_example()]);
        state.set_news("example", PluginNews::Failed(String::from("offline")));
        state.set_too_new("example", Some(String::from("0.6.0")));

        let page = page_of(&state, SystemTime::now());

        assert_eq!(
            page.too_new,
            Some(i18n::filled(Key::TplPluginsTooNew, &[("version", "0.6.0")]))
        );
        assert_eq!(page.waiting, None);
    }

    /// The error of an open dialog shows in the dialog, not on the page.
    #[test]
    fn an_error_shows_on_the_page_only_without_a_dialog() {
        let mut state = selected(vec![official_example()]);
        state.set_error(Some(String::from("disk full")));
        let on_page = page_of(&state, SystemTime::now()).error;

        state.show(crate::state::Dialog::Log(String::from("example")));
        state.set_error(Some(String::from("disk full")));

        assert_eq!(on_page.as_deref(), Some("disk full"));
        assert_eq!(page_of(&state, SystemTime::now()).error, None);
    }
}
