//! `plugins.json`: what is installed, from where, with which file hashes,
//! the permissions the user granted to each part, and which parts are on
//! (spec 2026-11-19-agent-plugins, "Install and update flow", step 6). It
//! is
//! always written whole through a temporary file, so a crash leaves the
//! old file or the new one, never half of one.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use otty_plugin_manifest::{PartKind, PartRef, PluginVersion};
use serde::{Deserialize, Serialize};

use crate::error::InstallError;
use crate::github::RepositoryUrl;

/// The SHA-256 (lowercase hex) of each file of one release, by file name.
pub type FileHashes = BTreeMap<String, String>;

/// The whole of `plugins.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PluginsFile {
    #[serde(default)]
    pub(crate) plugins: Vec<InstalledPlugin>,
    /// When the last daily update check ran, in seconds since 1970.
    #[serde(default)]
    pub(crate) last_update_check: Option<u64>,
}

impl PluginsFile {
    /// The file at `path`; an empty one when it does not exist yet.
    pub(crate) fn load(path: &Path) -> Result<Self, InstallError> {
        let text = match fs::read(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            },
            Err(error) => {
                return Err(InstallError::disk("read plugins.json", &error));
            },
        };

        serde_json::from_slice(&text).map_err(|error| {
            InstallError::Disk(format!("plugins.json can not be read: {error}"))
        })
    }

    /// Writes the file whole to `path`: first to a temporary file next to
    /// it, flushed to the disk, then renamed over the old one.
    pub(crate) fn save(&self, path: &Path) -> Result<(), InstallError> {
        let text = serde_json::to_vec_pretty(self).map_err(|error| {
            InstallError::Disk(format!(
                "plugins.json can not be written: {error}"
            ))
        })?;
        let disk = |error: std::io::Error| {
            InstallError::disk("write plugins.json", &error)
        };

        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder).map_err(disk)?;
        }
        let temporary = temporary_path(path);
        let mut file = fs::File::create(&temporary).map_err(disk)?;
        file.write_all(&text).map_err(disk)?;
        file.sync_all().map_err(disk)?;
        drop(file);

        fs::rename(&temporary, path).map_err(disk)?;
        if let Some(folder) = path.parent()
            && let Ok(folder) = fs::File::open(folder)
        {
            let _ = folder.sync_all();
        }

        Ok(())
    }

    /// The installed plugin `id`.
    pub(crate) fn plugin(&self, id: &str) -> Option<&InstalledPlugin> {
        self.plugins.iter().find(|plugin| plugin.id == id)
    }

    /// The installed plugin `id`, to change.
    pub(crate) fn plugin_mut(
        &mut self,
        id: &str,
    ) -> Option<&mut InstalledPlugin> {
        self.plugins.iter_mut().find(|plugin| plugin.id == id)
    }
}

/// One installed plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) source: Source,
    #[serde(with = "version_text")]
    pub(crate) version: PluginVersion,
    pub(crate) tag: String,
    pub(crate) wasm: String,
    #[serde(default)]
    pub(crate) hashes: BTreeMap<String, FileHashes>,
    pub(crate) parts: Vec<InstalledPart>,
}

impl InstalledPlugin {
    /// The plugin id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The plugin's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the plugin came from and updates from.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// The installed version.
    pub fn version(&self) -> &PluginVersion {
        &self.version
    }

    /// The release tag of the installed version, or the version itself
    /// for a local plugin.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The file name of the plugin's `.wasm`.
    pub fn wasm(&self) -> &str {
        &self.wasm
    }

    /// The SHA-256 of each file, per release tag OTTY installed.
    pub fn hashes(&self) -> &BTreeMap<String, FileHashes> {
        &self.hashes
    }

    /// The parts, as the installed manifest lists them.
    pub fn parts(&self) -> &[InstalledPart] {
        &self.parts
    }

    /// The part `part`, if the plugin has it.
    pub fn part(&self, part: &PartRef) -> Option<&InstalledPart> {
        self.parts
            .iter()
            .find(|installed| installed.part() == *part)
    }

    /// "Official", "Community" or "Local".
    pub fn trust(&self) -> Trust {
        self.source.trust()
    }

    /// The SHA-256 of the installed `.wasm`.
    pub fn wasm_sha256(&self) -> Option<&str> {
        self.hashes
            .get(&self.tag)
            .and_then(|hashes| hashes.get(&self.wasm))
            .map(String::as_str)
    }

    /// The part `part`, to change.
    pub(crate) fn part_mut(
        &mut self,
        part: &PartRef,
    ) -> Option<&mut InstalledPart> {
        self.parts
            .iter_mut()
            .find(|installed| installed.part() == *part)
    }
}

/// One part of an installed plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPart {
    #[serde(with = "kind_key")]
    pub(crate) kind: PartKind,
    pub(crate) id: String,
    pub(crate) on: bool,
    #[serde(default)]
    pub(crate) hosts: Vec<String>,
    pub(crate) granted: Vec<String>,
}

impl InstalledPart {
    /// The part's kind and id.
    pub fn part(&self) -> PartRef {
        match self.kind {
            PartKind::Agent => PartRef::agent(&self.id),
            PartKind::Provider => PartRef::provider(&self.id),
        }
    }

    /// Whether the part is on: only parts that are on fill the registry.
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The hosts a provider part serves.
    pub fn hosts(&self) -> &[String] {
        &self.hosts
    }

    /// The permissions the user granted, in plain words.
    pub fn granted(&self) -> &[String] {
        &self.granted
    }
}

/// Where a plugin came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// The GitHub releases of a repository; it updates from them.
    Release {
        /// The repository link.
        url: String,
    },
    /// A local folder its author builds it in; it changes only on Reload.
    Local {
        /// The folder.
        folder: PathBuf,
    },
}

impl Source {
    /// "Official", "Community" or "Local".
    pub fn trust(&self) -> Trust {
        match self {
            Source::Release { url } => {
                let official = RepositoryUrl::parse(url)
                    .is_ok_and(|url| url.is_official());
                if official {
                    Trust::Official
                } else {
                    Trust::Community
                }
            },
            Source::Local { .. } => Trust::Local,
        }
    }

    /// Where the plugin came from, in words, for "already installed from".
    pub(crate) fn origin(&self) -> String {
        match self {
            Source::Release { url } => url.clone(),
            Source::Local { folder } => folder.display().to_string(),
        }
    }
}

/// How far a plugin is trusted, as Plugins mode marks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// Its repository is under the official organization: reviewed by the
    /// OTTY team.
    Official,
    /// Any other repository: reviewed by its author only.
    Community,
    /// Installed from a local folder.
    Local,
}

/// The temporary file `plugins.json` is written to before the rename.
fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");

    path.with_file_name(name)
}

/// A [`PluginVersion`] as its text in JSON.
mod version_text {
    use otty_plugin_manifest::PluginVersion;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        version: &PluginVersion,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&version.to_string())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PluginVersion, D::Error> {
        let text = String::deserialize(deserializer)?;

        PluginVersion::parse(&text).ok_or_else(|| {
            serde::de::Error::custom(format!("`{text}` is not a version"))
        })
    }
}

/// A [`PartKind`] as its manifest key in JSON, `agents.agent` or
/// `agents.provider`.
mod kind_key {
    use otty_plugin_manifest::PartKind;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        kind: &PartKind,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(kind.key())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PartKind, D::Error> {
        let text = String::deserialize(deserializer)?;

        PartKind::from_key(&text).ok_or_else(|| {
            serde::de::Error::custom(format!("`{text}` is not a part kind"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    fn one_plugin() -> PluginsFile {
        PluginsFile {
            plugins: vec![InstalledPlugin {
                id: "example".to_string(),
                name: "Example".to_string(),
                source: Source::Release {
                    url: "https://github.com/someone/example".to_string(),
                },
                version: PluginVersion::parse("1.0.0").expect("version"),
                tag: "v1.0.0".to_string(),
                wasm: "example.wasm".to_string(),
                hashes: BTreeMap::from([(
                    "v1.0.0".to_string(),
                    FileHashes::from([(
                        "example.wasm".to_string(),
                        "ab".repeat(32),
                    )]),
                )]),
                parts: vec![InstalledPart {
                    kind: PartKind::Provider,
                    id: "example-cloud".to_string(),
                    on: true,
                    hosts: vec!["example.com".to_string()],
                    granted: vec![
                        "Sends requests to `api.example.com`".to_string(),
                    ],
                }],
            }],
            last_update_check: Some(1_763_600_000),
        }
    }

    #[test]
    fn the_file_reads_back_what_was_written() {
        let folder = TempDir::new();
        let path = folder.path().join("config/plugins.json");

        one_plugin().save(&path).expect("saved");
        let read = PluginsFile::load(&path).expect("read");

        assert_eq!(read, one_plugin());
        assert_eq!(
            read.plugins[0].wasm_sha256(),
            Some("ab".repeat(32).as_str())
        );
    }

    #[test]
    fn a_missing_file_is_an_empty_one() {
        let folder = TempDir::new();

        let read = PluginsFile::load(&folder.path().join("plugins.json"));

        assert_eq!(read, Ok(PluginsFile::default()));
    }

    #[test]
    fn the_file_survives_a_crash_mid_write() {
        let folder = TempDir::new();
        let path = folder.path().join("plugins.json");
        one_plugin().save(&path).expect("saved");

        // A crash while writing leaves a half-written temporary file next
        // to the old one; the old one is still whole.
        fs::write(temporary_path(&path), b"{\"plugins\": [{\"id\": \"exa")
            .expect("half a file");
        let after_crash = PluginsFile::load(&path).expect("read");
        // The next write replaces the half-written file.
        PluginsFile::default().save(&path).expect("saved again");

        assert_eq!(after_crash, one_plugin());
        assert_eq!(PluginsFile::load(&path), Ok(PluginsFile::default()));
        assert!(!temporary_path(&path).exists());
    }

    #[test]
    fn a_failed_write_keeps_the_old_file() {
        let folder = TempDir::new();
        let path = folder.path().join("plugins.json");
        one_plugin().save(&path).expect("saved");
        fs::create_dir(temporary_path(&path)).expect("a folder in the way");

        let failed = PluginsFile::default().save(&path);

        assert!(matches!(failed, Err(InstallError::Disk(_))));
        assert_eq!(PluginsFile::load(&path), Ok(one_plugin()));
    }

    #[test]
    fn trust_follows_the_source() {
        let release = |url: &str| Source::Release {
            url: url.to_string(),
        };
        let official = format!(
            "https://github.com/{}/x",
            crate::github::OFFICIAL_ORGANIZATION
        );

        assert_eq!(release(&official).trust(), Trust::Official);
        assert_eq!(
            release("https://github.com/someone/x").trust(),
            Trust::Community
        );
        assert_eq!(
            Source::Local {
                folder: PathBuf::from(format!(
                    "/src/{}",
                    crate::github::OFFICIAL_ORGANIZATION
                )),
            }
            .trust(),
            Trust::Local
        );
    }
}
