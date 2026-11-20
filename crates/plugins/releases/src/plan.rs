//! What the install and update dialogs show before the user agrees, and
//! what an update check reports.

use otty_plugin_manifest::{Manifest, PartRef, PluginVersion};

use crate::choose::SkippedRelease;
use crate::diff::PermissionDiff;
use crate::error::InstallError;
use crate::package::Package;
use crate::plugins_file::{Served, Source, Trust, Winner};

/// A plugin found and checked, waiting for the install dialog: its
/// manifest with each part's permissions, how far it is trusted, and each
/// id or host another plugin already serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    package: Package,
    conflicts: Vec<Conflict>,
}

impl InstallPlan {
    /// The checked manifest: name, authors, repository, version, parts.
    pub fn manifest(&self) -> &Manifest {
        self.package.manifest()
    }

    /// The release tag, or the version of a local plugin.
    pub fn tag(&self) -> &str {
        self.package.tag()
    }

    /// Where the plugin comes from.
    pub fn source(&self) -> &Source {
        self.package.source()
    }

    /// "Official", "Community" or "Local".
    pub fn trust(&self) -> Trust {
        self.package.source().trust()
    }

    /// The ids and hosts of its parts another plugin, or a reader built
    /// into OTTY, already serves: the dialog asks which one serves each.
    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    pub(crate) fn package(&self) -> &Package {
        &self.package
    }

    pub(crate) fn new(package: Package, conflicts: Vec<Conflict>) -> Self {
        Self { package, conflicts }
    }
}

/// One id or host of a new part that is already served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    part: PartRef,
    served: Served,
    current: Winner,
}

impl Conflict {
    /// The new part.
    pub fn part(&self) -> &PartRef {
        &self.part
    }

    /// The id or host both would serve.
    pub fn served(&self) -> &Served {
        &self.served
    }

    /// Who serves it now.
    pub fn current(&self) -> &Winner {
        &self.current
    }

    pub(crate) fn new(part: PartRef, served: Served, current: Winner) -> Self {
        Self {
            part,
            served,
            current,
        }
    }
}

/// A newer version that waits for the user, or is about to install: the
/// permission changes part by part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUpdate {
    from: PluginVersion,
    package: Package,
    diff: PermissionDiff,
    conflicts: Vec<Conflict>,
}

impl PendingUpdate {
    /// The plugin id.
    pub fn plugin_id(&self) -> &str {
        self.package.id()
    }

    /// The installed version.
    pub fn from(&self) -> &PluginVersion {
        &self.from
    }

    /// The new version.
    pub fn to(&self) -> &PluginVersion {
        self.package.manifest().plugin().version()
    }

    /// The new manifest.
    pub fn manifest(&self) -> &Manifest {
        self.package.manifest()
    }

    /// The permission changes, part by part.
    pub fn diff(&self) -> &PermissionDiff {
        &self.diff
    }

    /// The ids and hosts of its new parts another plugin, or a reader
    /// built into OTTY, already serves: the update dialog asks which one
    /// serves each, as the install dialog does.
    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    pub(crate) fn package(&self) -> &Package {
        &self.package
    }

    pub(crate) fn new(
        from: PluginVersion,
        package: Package,
        diff: PermissionDiff,
        conflicts: Vec<Conflict>,
    ) -> Self {
        Self {
            from,
            package,
            diff,
            conflicts,
        }
    }
}

/// What an update check, a reload or the start-up install of the built-in
/// plugins did for one plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateReport {
    plugin_id: String,
    outcome: Result<UpdateOutcome, InstallError>,
    too_new: Option<SkippedRelease>,
}

impl UpdateReport {
    /// The plugin id.
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// What happened, or why nothing could.
    pub fn outcome(&self) -> &Result<UpdateOutcome, InstallError> {
        &self.outcome
    }

    /// A newer release skipped because it needs a newer OTTY: the row
    /// says "0.6.0 needs a newer OTTY".
    pub fn too_new(&self) -> Option<&SkippedRelease> {
        self.too_new.as_ref()
    }

    pub(crate) fn new(
        plugin_id: &str,
        outcome: Result<UpdateOutcome, InstallError>,
        too_new: Option<SkippedRelease>,
    ) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            outcome,
            too_new,
        }
    }
}

/// What happened to one plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// Nothing newer.
    UpToDate,
    /// A built-in plugin installed on first start.
    Installed,
    /// Installed on its own: "Updated to 0.5.0".
    Updated {
        /// The version before.
        from: PluginVersion,
        /// The version now.
        to: PluginVersion,
    },
    /// The new version asks for new permissions or parts; the installed
    /// one keeps running until the user agrees.
    Waiting(Box<PendingUpdate>),
}
