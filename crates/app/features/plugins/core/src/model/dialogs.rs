//! The dialogs of the page that show permissions (spec
//! 2026-11-19-agent-plugins, "Install and update flow", "Permissions"):
//! the install dialog with each part and its permissions in words, the
//! update dialog with what changed part by part, and the permissions of
//! an installed plugin.

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_manifest::Manifest;
use otty_plugin_releases::{InstallPlan, PartChange, PendingUpdate};

use crate::state::PluginsState;
use crate::types::{ListedPlugin, part_name};
use crate::words::{granted_text, part_permissions, part_title, trust_note};

/// One part and its permissions; in the update dialog what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartPermissionsViewModel {
    /// The part with its kind, such as "Coder (agent)".
    pub(crate) title: String,
    /// "new part" or "no longer in the plugin" in the update dialog.
    pub(crate) mark: Option<&'static str>,
    /// Its permissions; in the update dialog the new ones.
    pub(crate) lines: Vec<String>,
    /// The permissions it no longer asks for, in the update dialog.
    pub(crate) removed: Vec<String>,
}

/// The install dialog of a plugin found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstallDialogViewModel {
    /// "Install Example 0.1.0".
    pub(crate) title: String,
    /// "By OTTY team", when the manifest names its authors.
    pub(crate) authors: Option<String>,
    /// The repository the manifest names.
    pub(crate) repository: Option<String>,
    /// Who reviewed it: official, community or local.
    pub(crate) trust: &'static str,
    /// Each part with its permissions.
    pub(crate) parts: Vec<PartPermissionsViewModel>,
    /// Why the install failed.
    pub(crate) error: Option<String>,
    /// Whether the install runs.
    pub(crate) is_busy: bool,
}

/// The update dialog of a waiting update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateDialogViewModel {
    /// "Update Example 0.1.0 → 0.2.0".
    pub(crate) title: String,
    /// The parts that changed.
    pub(crate) parts: Vec<PartPermissionsViewModel>,
    /// Why the update failed.
    pub(crate) error: Option<String>,
    /// Whether the update runs.
    pub(crate) is_busy: bool,
}

/// The permissions of an installed plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionsViewModel {
    /// "Permissions of Example".
    pub(crate) title: String,
    /// Each part with the permissions the user granted.
    pub(crate) parts: Vec<PartPermissionsViewModel>,
}

/// The install dialog of `plan`.
pub(crate) fn install_dialog(
    state: &PluginsState,
    plan: &InstallPlan,
) -> InstallDialogViewModel {
    let manifest = plan.manifest();
    let plugin = manifest.plugin();

    let authors = (!plugin.authors().is_empty()).then(|| {
        i18n::filled(
            Key::TplPluginsAuthors,
            &[("authors", &plugin.authors().join(", "))],
        )
    });
    let parts = manifest
        .parts()
        .iter()
        .map(|part| PartPermissionsViewModel {
            title: part_title(&part_name(Some(manifest), part), part),
            mark: None,
            lines: part_permissions(manifest, part),
            removed: Vec::new(),
        })
        .collect();

    InstallDialogViewModel {
        title: i18n::filled(
            Key::TplPluginsInstallTitle,
            &[
                ("name", plugin.name()),
                ("version", &plugin.version().to_string()),
            ],
        ),
        authors,
        repository: plugin.repository().map(String::from),
        trust: trust_note(plan.trust()),
        parts,
        error: state.error().map(String::from),
        is_busy: state.is_busy(),
    }
}

/// The update dialog of `update` of the installed `plugin`.
pub(crate) fn update_dialog(
    state: &PluginsState,
    plugin: &ListedPlugin,
    update: &PendingUpdate,
) -> UpdateDialogViewModel {
    let new = update.manifest();
    let old = plugin.manifest.as_ref();

    let parts = update
        .diff()
        .parts()
        .iter()
        .map(|change| part_change(change, new, old))
        .collect();

    UpdateDialogViewModel {
        title: i18n::filled(
            Key::TplPluginsUpdateTitle,
            &[
                ("name", plugin.installed.name()),
                ("from", &update.from().to_string()),
                ("to", &update.to().to_string()),
            ],
        ),
        parts,
        error: state.error().map(String::from),
        is_busy: state.is_busy(),
    }
}

/// The permissions the user granted to each part of `plugin`.
pub(crate) fn permissions(plugin: &ListedPlugin) -> PermissionsViewModel {
    let manifest = plugin.manifest.as_ref();

    let parts = plugin
        .installed
        .parts()
        .iter()
        .map(|part| {
            let reference = part.part();
            PartPermissionsViewModel {
                title: part_title(&plugin.part_name(&reference), &reference),
                mark: None,
                lines: part
                    .granted()
                    .iter()
                    .map(|line| granted_text(line, manifest))
                    .collect(),
                removed: Vec::new(),
            }
        })
        .collect();

    PermissionsViewModel {
        title: i18n::filled(
            Key::TplPluginsPermissionsTitle,
            &[("name", plugin.installed.name())],
        ),
        parts,
    }
}

/// What changed in one part, its lines in the user's language: the new
/// ones from the new manifest, the removed ones from the installed one.
fn part_change(
    change: &PartChange,
    new: &Manifest,
    old: Option<&Manifest>,
) -> PartPermissionsViewModel {
    let in_new = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .map(|line| granted_text(line, Some(new)))
            .collect()
    };
    let in_old = |lines: &[String]| -> Vec<String> {
        lines.iter().map(|line| granted_text(line, old)).collect()
    };

    match change {
        PartChange::New { part, lines } => PartPermissionsViewModel {
            title: part_title(&part_name(Some(new), part), part),
            mark: Some(i18n::t(Key::PluginsNewPart)),
            lines: in_new(lines),
            removed: Vec::new(),
        },
        PartChange::Removed { part } => PartPermissionsViewModel {
            title: part_title(&part_name(old, part), part),
            mark: Some(i18n::t(Key::PluginsPartRemoved)),
            lines: Vec::new(),
            removed: Vec::new(),
        },
        PartChange::Changed {
            part,
            added,
            removed,
        } => PartPermissionsViewModel {
            title: part_title(&part_name(Some(new), part), part),
            mark: None,
            lines: in_new(added),
            removed: in_old(removed),
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_releases::Source;
    use otty_plugin_releases::fixtures::{
        example_manifest, install_plan, pending_update,
    };

    use super::*;
    use crate::testing::{agent, provider, released_example};
    use crate::words::permission_text;

    fn sends_requests(host: &str) -> String {
        i18n::filled(Key::TplPluginsPermSendsRequests, &[("host", host)])
    }

    /// Issue 09: the install dialog lists each part with its permissions
    /// in words, and who reviewed the plugin.
    #[test]
    fn the_install_dialog_lists_the_permissions_of_each_part() {
        let state = PluginsState::new(Vec::new());
        let plan = install_plan(
            &example_manifest("0.1.0"),
            Source::Release {
                url: String::from("https://github.com/someone/example"),
            },
        );

        let dialog = install_dialog(&state, &plan);

        assert_eq!(
            dialog.title,
            i18n::filled(
                Key::TplPluginsInstallTitle,
                &[("name", "Example"), ("version", "0.1.0")]
            )
        );
        assert_eq!(dialog.trust, i18n::t(Key::PluginsCommunityNote));
        let titles: Vec<&str> = dialog
            .parts
            .iter()
            .map(|part| part.title.as_str())
            .collect();
        assert_eq!(
            titles,
            [
                part_title("Example Agent", &agent()),
                part_title("Example Cloud", &provider()),
            ]
        );
        assert!(dialog.parts[0].lines.contains(&permission_text(
            &otty_plugin_manifest::PermissionLine::ReadsVariable(String::from(
                "EXAMPLE_AGENT_HOME"
            ))
        )));
        assert!(
            dialog.parts[1]
                .lines
                .contains(&sends_requests("api.example.com"))
        );
    }

    /// Issue 09: the update dialog shows the permission diff per part:
    /// the new ones and the ones no longer asked for.
    #[test]
    fn the_update_dialog_shows_the_permission_diff_per_part() {
        let plugin = released_example();
        let state = PluginsState::new(vec![plugin.clone()]);
        let newer = example_manifest("0.2.0").replace(
            "http = [\"api.example.com\"]",
            "http = [\"v2.example.com\"]",
        );
        let update = pending_update(&plugin.installed, &newer);

        let dialog = update_dialog(&state, &plugin, &update);

        assert_eq!(
            dialog.title,
            i18n::filled(
                Key::TplPluginsUpdateTitle,
                &[("name", "Example"), ("from", "0.1.0"), ("to", "0.2.0")]
            )
        );
        assert_eq!(
            dialog.parts,
            [PartPermissionsViewModel {
                title: part_title("Example Cloud", &provider()),
                mark: None,
                lines: vec![sends_requests("v2.example.com")],
                removed: vec![sends_requests("api.example.com")],
            }]
        );
    }

    /// A new part is marked new with all its permissions.
    #[test]
    fn a_new_part_of_an_update_is_marked_new() {
        let mut plugin = released_example();
        let agent_only = example_manifest("0.1.0")
            .split("[[agents.provider]]")
            .next()
            .expect("the agent half")
            .replace("agents.provider = \"0.1\"\n", "");
        plugin.installed = otty_plugin_releases::fixtures::installed(
            &agent_only,
            plugin.installed.source().clone(),
        );
        let state = PluginsState::new(vec![plugin.clone()]);
        let update =
            pending_update(&plugin.installed, &example_manifest("0.2.0"));

        let dialog = update_dialog(&state, &plugin, &update);

        assert_eq!(dialog.parts.len(), 1);
        assert_eq!(dialog.parts[0].mark, Some(i18n::t(Key::PluginsNewPart)));
        assert!(
            dialog.parts[0]
                .lines
                .contains(&sends_requests("api.example.com"))
        );
    }

    #[test]
    fn the_permissions_of_an_installed_plugin_are_the_granted_ones() {
        let permissions = permissions(&released_example());

        assert_eq!(permissions.parts.len(), 2);
        assert!(
            permissions.parts[1]
                .lines
                .contains(&sends_requests("api.example.com"))
        );
    }
}
