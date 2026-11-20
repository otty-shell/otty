//! The version of a plugin release: semver, as the manifest writes it.

use std::fmt;

/// A plugin version such as `1.2.0` or `0.4.0-beta.1`, as the manifest's
/// `[plugin] version` names it (the release tag without `v`). Versions
/// order as semver does.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PluginVersion {
    version: semver::Version,
}

impl PluginVersion {
    /// The semver version.
    pub fn semver(&self) -> &semver::Version {
        &self.version
    }

    /// The version in `text`, or `None` when it is not semver.
    ///
    /// ```
    /// use otty_plugin_manifest::PluginVersion;
    ///
    /// assert!(PluginVersion::parse("1.2.0").is_some());
    /// assert!(PluginVersion::parse("0.4.0-beta.1").is_some());
    /// assert!(PluginVersion::parse("v1.2.0").is_none());
    /// assert!(PluginVersion::parse("1.2").is_none());
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        let version = semver::Version::parse(text).ok()?;

        Some(Self { version })
    }
}

impl fmt::Display for PluginVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_version_has_no_leading_v() {
        assert!(PluginVersion::parse("1.2.0").is_some());
        assert_eq!(PluginVersion::parse("v1.2.0"), None);
    }
}
