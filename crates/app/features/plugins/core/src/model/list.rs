//! The list of the installed plugins (spec 2026-11-19-agent-plugins, "UI
//! changes"): each with its icon, name, version, how far it is trusted,
//! its parts with a switch each and the last call of each that failed,
//! what the latest check said, and its actions.

use std::time::SystemTime;

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_host::{LastFailure, PluginFailureKind};
use otty_plugin_manifest::PartRef;
use otty_plugin_releases::Trust;

use crate::state::{PluginNews, PluginsState};
use crate::types::ListedPlugin;
use crate::words::{error_text, part_title, trust_text};

/// One plugin of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginRowViewModel {
    /// The plugin id.
    pub(crate) id: String,
    /// Its name.
    pub(crate) name: String,
    /// Its installed version.
    pub(crate) version: String,
    /// "Official", "Community" or "Local".
    pub(crate) trust: &'static str,
    /// The SVG of its icon, or `None` for a letter icon.
    pub(crate) icon: Option<Vec<u8>>,
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
    /// Its repository, for Open repository.
    pub(crate) repository: Option<String>,
    /// Whether its actions ("⋯") are shown.
    pub(crate) are_actions_open: bool,
    /// Whether "Read by" opened the page at it.
    pub(crate) is_highlighted: bool,
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
    /// Whether it is paused after three failures in a row: Try again.
    pub(crate) is_paused: bool,
    /// The last call of the part that failed or that the host refused,
    /// for plugin authors (issue 19).
    pub(crate) last_failure: Option<LastFailureViewModel>,
}

/// The last failed call of a part: when, which call and the kind of
/// error, never a value, a key or session text, and its report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LastFailureViewModel {
    /// "Last failure 5 min ago: read-limits, the plugin did not finish in
    /// time".
    pub(crate) text: String,
    /// The report Copy report puts on the clipboard.
    pub(crate) report: String,
}

/// The rows of the installed plugins, in their order, with the ages of
/// the last failures counted at `now`.
pub(crate) fn rows(
    state: &PluginsState,
    now: SystemTime,
) -> Vec<PluginRowViewModel> {
    state
        .plugins()
        .iter()
        .map(|plugin| row(state, plugin, now))
        .collect()
}

/// The reason of `error` for the row of a plugin.
pub(crate) fn failed(error: &otty_plugin_releases::InstallError) -> PluginNews {
    PluginNews::Failed(error_text(error))
}

fn row(
    state: &PluginsState,
    plugin: &ListedPlugin,
    now: SystemTime,
) -> PluginRowViewModel {
    let installed = &plugin.installed;
    let id = plugin.id();

    let parts = installed
        .parts()
        .iter()
        .map(|part| {
            let reference = part.part();
            PartRowViewModel {
                title: part_title(&plugin.part_name(&reference), &reference),
                is_on: part.is_on(),
                is_paused: is_paused(state, id, &reference),
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
    let repository = plugin.repository().map(String::from);

    PluginRowViewModel {
        id: id.to_string(),
        name: installed.name().to_string(),
        version: installed.version().to_string(),
        trust: trust_text(installed.trust()),
        icon: plugin.icon.clone(),
        parts,
        news,
        is_news_failure,
        waiting,
        is_local: installed.trust() == Trust::Local,
        repository,
        are_actions_open: state.actions_open() == Some(id),
        is_highlighted: state.highlighted() == Some(id),
    }
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

    /// Issue 09: the list model of the installed plugins and their parts.
    #[test]
    fn each_plugin_lists_its_parts_with_their_kind_and_switch() {
        let state = PluginsState::new(vec![official_example()]);

        let rows = rows(&state, SystemTime::now());

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.name, "Example");
        assert_eq!(row.version, "0.1.0");
        assert_eq!(row.trust, i18n::t(Key::PluginsOfficial));
        assert_eq!(
            row.parts,
            [
                PartRowViewModel {
                    part: agent(),
                    title: part_title("Example Agent", &agent()),
                    is_on: true,
                    is_paused: false,
                    last_failure: None,
                },
                PartRowViewModel {
                    part: provider(),
                    title: part_title("Example Cloud", &provider()),
                    is_on: true,
                    is_paused: false,
                    last_failure: None,
                },
            ]
        );
        assert!(!row.is_local);
        assert_eq!(
            row.repository.as_deref(),
            Some("https://github.com/otty-shell/otty-plugin-example")
        );
    }

    /// Issue 09: a local plugin shows "Local" and Reload in place of Check
    /// for updates.
    #[test]
    fn a_local_plugin_is_marked_local_and_reloads() {
        let state = PluginsState::new(vec![local_example()]);

        let rows = rows(&state, SystemTime::now());

        assert_eq!(rows[0].trust, i18n::t(Key::PluginsLocal));
        assert!(rows[0].is_local);
    }

    #[test]
    fn a_part_that_is_off_shows_its_switch_off() {
        let mut plugin = official_example();
        plugin.installed = with_part_on(plugin.installed, &agent(), false);
        let state = PluginsState::new(vec![plugin]);

        let rows = rows(&state, SystemTime::now());

        assert!(!rows[0].parts[0].is_on);
        assert!(rows[0].parts[1].is_on);
    }

    /// Decision 8: a part paused after three failures in a row shows Try
    /// again; the plugin's other part does not.
    #[test]
    fn a_paused_part_is_marked_paused() {
        let mut state = PluginsState::new(vec![official_example()]);
        state.set_running(vec![RunningPlugin {
            plugin: String::from("example"),
            log: PluginLog::new(),
            parts: vec![
                (agent(), paused_health()),
                (provider(), PartHealth::default()),
            ],
        }]);

        let rows = rows(&state, SystemTime::now());

        assert!(rows[0].parts[0].is_paused);
        assert!(!rows[0].parts[1].is_paused);
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
            "read-limits",
            PluginFailureKind::CouldNotRead("host error: not allowed"),
        );
        let mut state = PluginsState::new(vec![official_example()]);
        state.set_running(vec![RunningPlugin {
            plugin: String::from("example"),
            log: PluginLog::new(),
            parts: vec![
                (agent(), failed_health(&failure, failed_at)),
                (provider(), PartHealth::default()),
            ],
        }]);

        let rows = rows(&state, now);

        let kind = format!(
            "{} (host error: not allowed)",
            i18n::t(Key::AgentsPluginFailureCouldNotRead)
        );
        assert_eq!(
            rows[0].parts[0].last_failure,
            Some(LastFailureViewModel {
                text: i18n::filled(
                    Key::TplPluginsLastFailure,
                    &[
                        ("time", &i18n::time_ago(Duration::from_secs(300))),
                        ("call", "read-limits"),
                        ("kind", &kind),
                    ],
                ),
                report: failure.report(),
            })
        );
        assert_eq!(rows[0].parts[1].last_failure, None);
    }

    #[test]
    fn an_update_and_a_release_for_a_newer_otty_show_on_the_row() {
        let mut state = PluginsState::new(vec![official_example()]);
        state.set_news("example", PluginNews::Updated(String::from("0.5.0")));
        state.set_too_new("example", Some(String::from("0.6.0")));

        let rows = rows(&state, SystemTime::now());

        let updated =
            i18n::filled(Key::TplPluginsUpdatedTo, &[("version", "0.5.0")]);
        let too_new =
            i18n::filled(Key::TplPluginsTooNew, &[("version", "0.6.0")]);
        assert_eq!(rows[0].news, Some(format!("{updated} · {too_new}")));
        assert!(!rows[0].is_news_failure);
    }
}
