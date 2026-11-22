//! The main area of Plugins mode (issue 30): the page of the selected
//! plugin, with what it is (its name, version, authors, repository and
//! source), who reviewed it, its parts with a switch each, their
//! permissions, health and last failed call, what the latest check said
//! and its actions; or a short line that says what to do when nothing is
//! selected or nothing is installed.

use std::time::SystemTime;

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_host::{LastFailure, PluginFailureKind};
use otty_plugin_manifest::PartRef;
use otty_plugin_releases::{Source, Trust};

use crate::state::{PluginNews, PluginsState};
use crate::types::ListedPlugin;
use crate::words::{error_text, granted_text, part_title, trust_note};

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
    /// Who reviewed it: "Official plugin: reviewed by the OTTY team".
    pub(crate) trust_note: &'static str,
    /// The SVG of its icon, or `None` for a letter icon.
    pub(crate) icon: Option<Vec<u8>>,
    /// Its authors, from its manifest.
    pub(crate) authors: Option<String>,
    /// Its repository, for Open repository.
    pub(crate) repository: Option<String>,
    /// Where it came from and how it updates.
    pub(crate) source: String,
    /// Its parts, each with its switch.
    pub(crate) parts: Vec<PartRowViewModel>,
    /// What the latest check or change said, such as "Updated to 0.5.0".
    pub(crate) news: Option<String>,
    /// Whether the line of the news is a failure.
    pub(crate) is_news_failure: bool,
    /// The line of an update that waits for the user's OK, with Review.
    pub(crate) waiting: Option<String>,
    /// Whether it is a local plugin: Reload in place of Check for updates.
    pub(crate) is_local: bool,
    /// Why the latest switch failed.
    pub(crate) error: Option<String>,
    /// Whether a change runs: the switches do nothing.
    pub(crate) is_busy: bool,
}

/// One part of a plugin with its switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartRowViewModel {
    /// The part.
    pub(crate) part: PartRef,
    /// Its name and kind, such as "Coder (agent)".
    pub(crate) title: String,
    /// Whether it is on.
    pub(crate) is_on: bool,
    /// "Working", "Off" or "Paused after three failures".
    pub(crate) health: &'static str,
    /// Whether it is paused after three failures in a row: Try again.
    pub(crate) is_paused: bool,
    /// The permissions the user granted to it, in their language.
    pub(crate) permissions: Vec<String>,
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
            PartRowViewModel {
                title: part_title(&plugin.part_name(&reference), &reference),
                is_on: part.is_on(),
                health: health_text(part.is_on(), is_paused),
                is_paused,
                permissions: part
                    .granted()
                    .iter()
                    .map(|line| granted_text(line, manifest))
                    .collect(),
                last_failure: last_failure(state, id, &reference)
                    .map(|last| last_failure_line(&last, now)),
                part: reference,
            }
        })
        .collect();
    let (news, is_news_failure) = news_line(state, id);
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
        trust_note: trust_note(installed.trust()),
        icon: plugin.icon.clone(),
        authors,
        repository: plugin.repository().map(String::from),
        source: source_text(installed.source()),
        parts,
        news,
        is_news_failure,
        waiting,
        is_local: installed.trust() == Trust::Local,
        error,
        is_busy: state.is_busy(),
    }
}

/// Where a plugin came from and how it updates, in the user's language.
fn source_text(source: &Source) -> String {
    match source {
        Source::Release { .. } => {
            i18n::t(Key::PluginsSourceRelease).to_string()
        },
        Source::Local { folder } => i18n::filled(
            Key::TplPluginsSourceFolder,
            &[("folder", &folder.display().to_string())],
        ),
    }
}

/// The health of a part: off, paused after three failures, or working.
fn health_text(is_on: bool, is_paused: bool) -> &'static str {
    let key = if !is_on {
        Key::PluginsHealthOff
    } else if is_paused {
        Key::PluginsPaused
    } else {
        Key::PluginsHealthWorking
    };

    i18n::t(key)
}

/// What the latest check said of the plugin `id`, and whether it is a
/// failure. A newer version that needs a newer OTTY comes after an
/// update.
fn news_line(state: &PluginsState, id: &str) -> (Option<String>, bool) {
    let too_new = state.too_new_of(id).map(|version| {
        i18n::filled(Key::TplPluginsTooNew, &[("version", version)])
    });

    match state.news_of(id) {
        Some(PluginNews::Updated(version)) => {
            let updated =
                i18n::filled(Key::TplPluginsUpdatedTo, &[("version", version)]);
            let line = match too_new {
                Some(too_new) => format!("{updated} · {too_new}"),
                None => updated,
            };
            (Some(line), false)
        },
        Some(PluginNews::Failed(reason)) => (Some(reason.clone()), true),
        Some(PluginNews::UpToDate) => {
            let up_to_date = i18n::t(Key::PluginsUpToDate).to_string();
            (Some(too_new.unwrap_or(up_to_date)), false)
        },
        _ => (too_new, false),
    }
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
        kind => kind_text(kind).to_string(),
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
fn kind_text(kind: PluginFailureKind) -> &'static str {
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
    use otty_plugin_releases::fixtures::with_part_on;

    use super::*;
    use crate::testing::{agent, local_example, official_example, provider};
    use crate::types::RunningPlugin;

    fn sends_requests(host: &str) -> String {
        i18n::filled(Key::TplPluginsPermSendsRequests, &[("host", host)])
    }

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

    /// Issue 30: the page says what the plugin is: its name, version,
    /// authors, repository, source and who reviewed it.
    #[test]
    fn the_page_says_what_the_plugin_is() {
        let state = selected(vec![official_example()]);

        let page = page_of(&state, SystemTime::now());

        assert_eq!(page.name, "Example");
        assert_eq!(page.version, "0.1.0");
        assert_eq!(page.authors.as_deref(), Some("OTTY team"));
        assert_eq!(
            page.repository.as_deref(),
            Some("https://github.com/otty-shell/otty-plugin-example")
        );
        assert_eq!(page.source, i18n::t(Key::PluginsSourceRelease));
        assert_eq!(page.trust_note, i18n::t(Key::PluginsOfficialNote));
        assert!(page.description.is_some());
        assert!(!page.is_local);
    }

    /// Issue 09: the page lists the parts with their kind, switch, health
    /// and the permissions the user granted.
    #[test]
    fn each_part_shows_its_switch_health_and_permissions() {
        let state = selected(vec![official_example()]);

        let page = page_of(&state, SystemTime::now());

        let titles: Vec<&str> =
            page.parts.iter().map(|part| part.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                part_title("Example Agent", &agent()),
                part_title("Example Cloud", &provider()),
            ]
        );
        assert!(page.parts.iter().all(|part| part.is_on));
        assert!(
            page.parts
                .iter()
                .all(|part| part.health == i18n::t(Key::PluginsHealthWorking))
        );
        assert!(
            page.parts[1]
                .permissions
                .contains(&sends_requests("api.example.com"))
        );
    }

    /// Issue 09: a local plugin shows its folder as the source and Reload
    /// in place of Check for updates.
    #[test]
    fn a_local_plugin_shows_its_folder_and_reloads() {
        let state = selected(vec![local_example()]);

        let page = page_of(&state, SystemTime::now());

        assert_eq!(page.trust_note, i18n::t(Key::PluginsLocalNote));
        assert_eq!(
            page.source,
            i18n::filled(
                Key::TplPluginsSourceFolder,
                &[("folder", "/work/otty-plugin-example")]
            )
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
        assert_eq!(page.parts[0].health, i18n::t(Key::PluginsPaused));
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

    #[test]
    fn an_update_and_a_release_for_a_newer_otty_show_on_the_page() {
        let mut state = selected(vec![official_example()]);
        state.set_news("example", PluginNews::Updated(String::from("0.5.0")));
        state.set_too_new("example", Some(String::from("0.6.0")));

        let page = page_of(&state, SystemTime::now());

        let updated =
            i18n::filled(Key::TplPluginsUpdatedTo, &[("version", "0.5.0")]);
        let too_new =
            i18n::filled(Key::TplPluginsTooNew, &[("version", "0.6.0")]);
        assert_eq!(page.news, Some(format!("{updated} · {too_new}")));
        assert!(!page.is_news_failure);
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
