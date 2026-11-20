//! Made-up plugin records for the tests of other crates, such as the
//! Plugins page: an installed plugin, an install plan, a waiting update
//! and the report of an update check, each from the text of a manifest.
//! Only with the `fixtures` feature, which only test builds turn on.

use std::collections::BTreeMap;

use otty_plugin_manifest::{Manifest, PartRef};

use crate::choose::SkippedRelease;
use crate::diff::PermissionDiff;
use crate::error::InstallError;
use crate::installer::parts_of;
use crate::package::{MANIFEST_FILE, Package};
use crate::plan::{
    Conflict, InstallPlan, PendingUpdate, UpdateOutcome, UpdateReport,
};
use crate::plugins_file::{InstalledPlugin, Served, Source, Winner};

/// The manifest of the template plugin, `assets/plugins/example/`, at
/// `version`: an agent part `example-agent` and a provider part
/// `example-cloud`.
pub fn example_manifest(version: &str) -> String {
    include_str!("../../../../assets/plugins/example/otty-plugin.toml")
        .replacen(
            "version = \"0.1.0\"",
            &format!("version = \"{version}\""),
            1,
        )
}

/// The plugin of `manifest` as `plugins.json` records it, installed from
/// `source`, all its parts on; `built_in` for a copy built into OTTY.
///
/// ```
/// use otty_plugin_releases::Source;
/// use otty_plugin_releases::fixtures::{example_manifest, installed};
///
/// let plugin = installed(&example_manifest("0.1.0"), Source::Embedded, true);
///
/// assert_eq!(plugin.id(), "example");
/// assert!(plugin.parts().iter().all(|part| part.is_on()));
/// ```
pub fn installed(
    manifest: &str,
    source: Source,
    built_in: bool,
) -> InstalledPlugin {
    let parsed = parse(manifest);
    let plugin = parsed.plugin();

    InstalledPlugin {
        id: plugin.id().to_string(),
        name: plugin.name().to_string(),
        source,
        built_in,
        version: plugin.version().clone(),
        tag: plugin.version().to_string(),
        wasm: plugin.wasm().to_string(),
        hashes: BTreeMap::new(),
        parts: parts_of(&parsed),
    }
}

/// `plugin` with the part `part` turned on or off.
pub fn with_part_on(
    mut plugin: InstalledPlugin,
    part: &PartRef,
    on: bool,
) -> InstalledPlugin {
    if let Some(installed) = plugin.part_mut(part) {
        installed.on = on;
    }

    plugin
}

/// The install plan of the plugin of `manifest` from `source`, with the
/// ids and hosts other plugins already serve.
pub fn install_plan(
    manifest: &str,
    source: Source,
    conflicts: Vec<Conflict>,
) -> InstallPlan {
    InstallPlan::new(package(manifest, source), conflicts)
}

/// The id or host `served` of the new part `part`, served now by
/// `current`.
pub fn conflict(part: PartRef, served: Served, current: Winner) -> Conflict {
    Conflict::new(part, served, current)
}

/// The version of `manifest` as a waiting update of `installed`, with the
/// permission diff and the ids and hosts of its new parts other plugins
/// serve, `conflicts`.
pub fn pending_update(
    installed: &InstalledPlugin,
    manifest: &str,
    conflicts: Vec<Conflict>,
) -> PendingUpdate {
    let package = package(manifest, installed.source().clone());
    let diff = PermissionDiff::between(installed.parts(), package.manifest());

    PendingUpdate::new(installed.version().clone(), package, diff, conflicts)
}

/// What a check of `plugin_id` did, with the newest release skipped for
/// needing a newer OTTY, as its tag and reason.
pub fn report(
    plugin_id: &str,
    outcome: Result<UpdateOutcome, InstallError>,
    too_new: Option<(&str, &str)>,
) -> UpdateReport {
    let too_new = too_new.map(|(tag, reason)| SkippedRelease {
        tag: tag.to_string(),
        reason: reason.to_string(),
    });

    UpdateReport::new(plugin_id, outcome, too_new)
}

fn package(manifest: &str, source: Source) -> Package {
    let files = BTreeMap::from([(
        MANIFEST_FILE.to_string(),
        manifest.as_bytes().to_vec(),
    )]);
    let built_in = source == Source::Embedded;

    Package::of_files(files, source, built_in).expect("a valid manifest")
}

fn parse(manifest: &str) -> Manifest {
    Manifest::parse(manifest).expect("a valid manifest")
}
