//! The dialogs of Plugins mode that show permissions (spec
//! 2026-11-19-agent-plugins, "Install and update flow", "Permissions"):
//! the install dialog with each part and its permissions grouped as on the
//! page of a plugin (issue 35), the update dialog with what changed part by
//! part.

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_manifest::Manifest;
use otty_plugin_releases::{InstallPlan, PartChange, PendingUpdate};

use super::permissions::{
    PermissionGroupViewModel, PlaceMark, granted_place, grouped, part_places,
};
use crate::state::PluginsState;
use crate::types::{ListedPlugin, part_name};
use crate::words::{kind_text, trust_note};

/// One part and its permissions; in the update dialog what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartPermissionsViewModel {
    /// The part's name, such as "Coder".
    pub(crate) name: String,
    /// Its kind for its tag: "Agent" or "Provider".
    pub(crate) kind: &'static str,
    /// "new part" or "no longer in the plugin" in the update dialog.
    pub(crate) mark: Option<&'static str>,
    /// Whether the new version no longer has the part: no permissions show.
    pub(crate) is_removed: bool,
    /// Its permissions grouped; in the update dialog the new ones marked
    /// new and the ones it no longer asks for after them.
    pub(crate) groups: Vec<PermissionGroupViewModel>,
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
            name: part_name(Some(manifest), part),
            kind: kind_text(part),
            mark: None,
            is_removed: false,
            groups: grouped(part_places(manifest, part)),
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

/// What changed in one part. A new part shows all its permissions; a
/// changed part shows all the permissions of the new version, the new ones
/// marked new, then the ones it no longer asks for, placed by the installed
/// manifest.
fn part_change(
    change: &PartChange,
    new: &Manifest,
    old: Option<&Manifest>,
) -> PartPermissionsViewModel {
    match change {
        PartChange::New { part, .. } => PartPermissionsViewModel {
            name: part_name(Some(new), part),
            kind: kind_text(part),
            mark: Some(i18n::t(Key::PluginsNewPart)),
            is_removed: false,
            groups: grouped(part_places(new, part)),
        },
        PartChange::Removed { part } => PartPermissionsViewModel {
            name: part_name(old, part),
            kind: kind_text(part),
            mark: Some(i18n::t(Key::PluginsPartRemoved)),
            is_removed: true,
            groups: Vec::new(),
        },
        PartChange::Changed {
            part,
            added,
            removed,
        } => {
            let added: Vec<_> = added
                .iter()
                .map(|line| granted_place(line, Some(new)))
                .collect();
            let mut places: Vec<_> = part_places(new, part)
                .into_iter()
                .map(|place| {
                    if added.contains(&place) {
                        place.marked(PlaceMark::New)
                    } else {
                        place
                    }
                })
                .collect();
            places.extend(removed.iter().map(|line| {
                granted_place(line, old).marked(PlaceMark::NoLonger)
            }));

            PartPermissionsViewModel {
                name: part_name(Some(new), part),
                kind: kind_text(part),
                mark: None,
                is_removed: false,
                groups: grouped(places),
            }
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
    use crate::model::permissions::{PermissionGroup, PlaceViewModel};
    use crate::testing::{agent, provider, released_example};

    fn site(host: &str, mark: Option<PlaceMark>) -> PlaceViewModel {
        PlaceViewModel {
            group: PermissionGroup::Sites,
            text: String::from(host),
            is_words: false,
            note: None,
            mark,
        }
    }

    /// Issue 09, 35: the install dialog lists each part with its kind and
    /// its permissions grouped as on the page, and who reviewed the plugin.
    #[test]
    fn the_install_dialog_groups_the_permissions_of_each_part() {
        let state = PluginsState::new(Vec::new());
        let plan = install_plan(
            &example_manifest("0.1.0"),
            Source::Release {
                url: String::from("https://github.com/someone/example"),
            },
        );
        let manifest = plan.manifest().clone();

        let dialog = install_dialog(&state, &plan);

        assert_eq!(
            dialog.title,
            i18n::filled(
                Key::TplPluginsInstallTitle,
                &[("name", "Example"), ("version", "0.1.0")]
            )
        );
        assert_eq!(dialog.trust, i18n::t(Key::PluginsCommunityNote));
        let names: Vec<(&str, &str)> = dialog
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
        assert_eq!(
            dialog.parts[0].groups,
            grouped(part_places(&manifest, &agent()))
        );
        assert_eq!(
            dialog.parts[1].groups,
            grouped(part_places(&manifest, &provider()))
        );
        assert_eq!(
            dialog.parts[1].groups[0].label,
            i18n::t(Key::PluginsGroupSites)
        );
    }

    /// Issue 09, 35: the update dialog shows the permissions of a changed
    /// part, the new ones marked new and the ones no longer asked for.
    #[test]
    fn the_update_dialog_marks_what_is_new_and_what_is_no_longer() {
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
        assert_eq!(dialog.parts.len(), 1);
        let part = &dialog.parts[0];
        assert_eq!(part.name, "Example Cloud");
        assert_eq!(part.mark, None);
        assert_eq!(part.groups[0].label, i18n::t(Key::PluginsGroupSites));
        assert_eq!(
            part.groups[0].places,
            [
                site("v2.example.com", Some(PlaceMark::New)),
                site("api.example.com", Some(PlaceMark::NoLonger)),
            ]
        );
        assert!(
            part.groups[1]
                .places
                .iter()
                .all(|place| place.mark.is_none())
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
        assert_eq!(
            dialog.parts[0].groups[0].places,
            [site("api.example.com", None)]
        );
    }
}
