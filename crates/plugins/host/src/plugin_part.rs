//! How a part of a plugin is named in reports, whatever domain it serves:
//! the plugin, its version and repository, and the part (issue 21).

use otty_plugin_manifest::Manifest;

/// One part of a plugin as reports and the registry of a domain name it:
/// the plugin's id, name, version and repository, and the id of the part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPart {
    plugin: String,
    name: String,
    version: String,
    repository: String,
    part: String,
}

impl PluginPart {
    /// The id of the plugin, such as `example`.
    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    /// The name of the plugin, such as "Example".
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The version of the plugin, such as `0.4.0`.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The git repository of the plugin.
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// The id of the part in the plugin, such as `coder`.
    pub fn part(&self) -> &str {
        &self.part
    }

    /// The part `part` of the plugin `plugin`, named `name`, at `version`
    /// from `repository`.
    ///
    /// ```
    /// use otty_plugin_host::PluginPart;
    ///
    /// let coder = PluginPart::new(
    ///     "example",
    ///     "Example",
    ///     "0.4.0",
    ///     "https://github.com/otty-shell/otty-plugin-example",
    ///     "coder",
    /// );
    ///
    /// assert_eq!(coder.part(), "coder");
    /// ```
    pub fn new(
        plugin: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        repository: impl Into<String>,
        part: impl Into<String>,
    ) -> Self {
        Self {
            plugin: plugin.into(),
            name: name.into(),
            version: version.into(),
            repository: repository.into(),
            part: part.into(),
        }
    }

    /// The part `part` of the plugin of `manifest`; a plugin with no
    /// repository, such as a local one, has an empty one.
    pub fn of_manifest(manifest: &Manifest, part: &str) -> Self {
        let plugin = manifest.plugin();

        Self::new(
            plugin.id(),
            plugin.name(),
            plugin.version().to_string(),
            plugin.repository().unwrap_or_default(),
            part,
        )
    }

    /// The lines a report names the part with: the plugin, its version,
    /// its repository and the part. Never anything the part read.
    pub fn report_lines(&self) -> [String; 3] {
        [
            format!("Plugin: {} {}", self.plugin, self.version),
            format!("Repository: {}", self.repository),
            format!("Part: {}", self.part),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_names_the_plugin_its_version_repository_and_part() {
        let part = PluginPart::new(
            "example",
            "Example",
            "0.4.0",
            "https://github.com/otty-shell/otty-plugin-example",
            "coder",
        );

        assert_eq!(
            part.report_lines(),
            [
                "Plugin: example 0.4.0",
                "Repository: https://github.com/otty-shell/otty-plugin-example",
                "Part: coder",
            ]
        );
    }
}
