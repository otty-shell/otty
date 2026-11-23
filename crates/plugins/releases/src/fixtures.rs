//! Made-up plugin records for the tests of other crates, such as the
//! Plugins mode: an installed plugin, an install plan, a waiting update
//! and the report of an update check, each from the text of a manifest.
//! Only with the `fixtures` feature, which only test builds turn on.

use std::collections::BTreeMap;

use otty_plugin_manifest::{Manifest, PartRef};

use crate::choose::SkippedRelease;
use crate::diff::PermissionDiff;
use crate::error::InstallError;
use crate::installer::parts_of;
use crate::package::{MANIFEST_FILE, Package};
use crate::plan::{InstallPlan, PendingUpdate, UpdateOutcome, UpdateReport};
use crate::plugins_file::{InstalledPlugin, Source};

/// The manifest of the template plugin, `assets/plugins/example/`, at
/// `version`: an agent part `example-agent` and a provider part
/// `example-cloud`.
///
/// The version line of the `[plugin]` table, the first that starts with
/// `version =`, is replaced whatever version the template has now, so a
/// new version of the template changes no test.
pub fn example_manifest(version: &str) -> String {
    let text =
        include_str!("../../../../assets/plugins/example/otty-plugin.toml");

    let mut is_replaced = false;
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            if is_replaced || !line.starts_with("version =") {
                return line.to_string();
            }

            is_replaced = true;
            format!("version = \"{version}\"")
        })
        .collect();

    lines.join("\n") + "\n"
}

/// The plugin of `manifest` as `plugins.json` records it, installed from
/// `source`, all its parts on.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_plugin_releases::Source;
/// use otty_plugin_releases::fixtures::{example_manifest, installed};
///
/// let folder = PathBuf::from("/work/example");
/// let plugin =
///     installed(&example_manifest("0.1.0"), Source::Local { folder });
///
/// assert_eq!(plugin.id(), "example");
/// assert!(plugin.parts().iter().all(|part| part.is_on()));
/// ```
pub fn installed(manifest: &str, source: Source) -> InstalledPlugin {
    let parsed = parse(manifest);
    let plugin = parsed.plugin();

    InstalledPlugin {
        id: plugin.id().to_string(),
        name: plugin.name().to_string(),
        source,
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

/// The install plan of the plugin of `manifest` from `source`.
pub fn install_plan(manifest: &str, source: Source) -> InstallPlan {
    InstallPlan::new(package(manifest, source))
}

/// The version of `manifest` as a waiting update of `installed`, with the
/// permission diff.
pub fn pending_update(
    installed: &InstalledPlugin,
    manifest: &str,
) -> PendingUpdate {
    let package = package(manifest, installed.source().clone());
    let diff = PermissionDiff::between(installed.parts(), package.manifest());

    PendingUpdate::new(installed.version().clone(), package, diff)
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

    Package::of_files(files, source).expect("a valid manifest")
}

fn parse(manifest: &str) -> Manifest {
    Manifest::parse(manifest).expect("a valid manifest")
}
