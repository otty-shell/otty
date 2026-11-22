//! What the install and update dialogs show before the user agrees, and
//! what an update check reports.

use otty_plugin_manifest::{Manifest, PluginVersion};

use crate::choose::SkippedRelease;
use crate::diff::PermissionDiff;
use crate::error::InstallError;
use crate::package::Package;
use crate::plugins_file::{Source, Trust};

/// A plugin found and checked, waiting for the install dialog: its
/// manifest with each part's permissions and how far it is trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    package: Package,
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

    pub(crate) fn package(&self) -> &Package {
        &self.package
    }

    pub(crate) fn new(package: Package) -> Self {
        Self { package }
    }
}

/// A newer version that waits for the user, or is about to install: the
/// permission changes part by part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUpdate {
    from: PluginVersion,
    package: Package,
    diff: PermissionDiff,
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

    pub(crate) fn package(&self) -> &Package {
        &self.package
    }

    pub(crate) fn new(
        from: PluginVersion,
        package: Package,
        diff: PermissionDiff,
    ) -> Self {
        Self {
            from,
            package,
            diff,
        }
    }
}

/// What an update check or a reload did for one plugin.
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
