//! One version of a plugin, found and with a checked manifest, and its
//! files: downloaded from a release or read from a local folder, each
//! within its size limit and each icon checked.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use otty_plugin_manifest::Manifest;
use sha2::{Digest, Sha256};

use crate::error::InstallError;
use crate::github::{Release, ReleaseAsset};
use crate::http::{HttpClient, HttpError};
use crate::icon::{MAX_ICON_BYTES, check_icon};
use crate::plugins_file::{FileHashes, Source};

/// The manifest's file name in a release or a folder.
pub const MANIFEST_FILE: &str = "otty-plugin.toml";
/// The largest `.wasm` OTTY installs.
pub const MAX_WASM_BYTES: u64 = 20 * 1024 * 1024;
/// The largest manifest OTTY reads.
pub(crate) const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

/// One version of a plugin, ready to install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Package {
    manifest: Manifest,
    tag: String,
    source: Source,
    files: PackageFiles,
}

impl Package {
    /// The checked manifest.
    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The release tag, or the version for a folder.
    pub(crate) fn tag(&self) -> &str {
        &self.tag
    }

    /// Where it comes from.
    pub(crate) fn source(&self) -> &Source {
        &self.source
    }

    /// The plugin id.
    pub(crate) fn id(&self) -> &str {
        self.manifest.plugin().id()
    }

    /// The release `release` of the repository `source`, whose manifest
    /// `manifest` was downloaded as `manifest_bytes`.
    pub(crate) fn of_release(
        manifest: Manifest,
        manifest_bytes: Vec<u8>,
        release: &Release,
        source: Source,
    ) -> Self {
        Self {
            manifest,
            tag: release.tag().to_string(),
            source,
            files: PackageFiles::Release {
                manifest: manifest_bytes,
                assets: release.assets().to_vec(),
            },
        }
    }

    /// The plugin made of `files`, by file name: a made-up plugin of the
    /// tests.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn of_files(
        files: BTreeMap<String, Vec<u8>>,
        source: Source,
    ) -> Result<Self, InstallError> {
        let Some(text) = files.get(MANIFEST_FILE) else {
            return Err(InstallError::MissingFile(MANIFEST_FILE.to_string()));
        };
        let manifest = parse_manifest(text)?;

        Ok(Self {
            tag: manifest.plugin().version().to_string(),
            manifest,
            source,
            files: PackageFiles::Ready(files),
        })
    }

    /// The plugin in the local folder `folder`: its manifest, its `.wasm`
    /// and its icons.
    pub(crate) fn of_folder(folder: &Path) -> Result<Self, InstallError> {
        let folder = fs::canonicalize(folder)
            .map_err(|error| InstallError::disk("open the folder", &error))?;

        let manifest_bytes =
            read_limited(&folder, MANIFEST_FILE, MAX_MANIFEST_BYTES)?;
        let manifest = parse_manifest(&manifest_bytes)?;
        let mut files =
            BTreeMap::from([(MANIFEST_FILE.to_string(), manifest_bytes)]);
        for name in needed_files(&manifest) {
            if name == MANIFEST_FILE {
                continue;
            }
            let bytes =
                read_limited(&folder, &name, limit_of(&manifest, &name))?;
            files.insert(name, bytes);
        }

        Ok(Self {
            tag: manifest.plugin().version().to_string(),
            manifest,
            source: Source::Local { folder },
            files: PackageFiles::Ready(files),
        })
    }

    /// The files the plugin needs, by name: the manifest, the `.wasm` and
    /// the icons, downloaded when they come from a release. Each is within
    /// its size limit and each icon passed its check.
    pub(crate) fn fetch(
        &self,
        http: &dyn HttpClient,
    ) -> Result<BTreeMap<String, Vec<u8>>, InstallError> {
        let mut files = BTreeMap::new();

        for name in needed_files(&self.manifest) {
            let limit = limit_of(&self.manifest, &name);
            let bytes = match &self.files {
                PackageFiles::Release { manifest, assets } => {
                    if name == MANIFEST_FILE {
                        manifest.clone()
                    } else {
                        let Some(asset) =
                            assets.iter().find(|asset| asset.name() == name)
                        else {
                            return Err(InstallError::MissingFile(name));
                        };
                        download(http, asset, limit)?
                    }
                },
                PackageFiles::Ready(ready) => {
                    let Some(bytes) = ready.get(&name) else {
                        return Err(InstallError::MissingFile(name));
                    };
                    bytes.clone()
                },
            };

            if bytes.len() as u64 > limit {
                return Err(InstallError::TooLarge { name, limit });
            }
            if self.is_icon(&name) {
                check_icon(&bytes).map_err(|problem| {
                    InstallError::BadIcon {
                        name: name.clone(),
                        problem,
                    }
                })?;
            }
            files.insert(name, bytes);
        }

        Ok(files)
    }

    fn is_icon(&self, name: &str) -> bool {
        name != MANIFEST_FILE && name != self.manifest.plugin().wasm()
    }
}

/// Where the files of a package are.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PackageFiles {
    /// The manifest is downloaded already; the rest are release assets.
    Release {
        manifest: Vec<u8>,
        assets: Vec<ReleaseAsset>,
    },
    /// Every file is here, by name.
    Ready(BTreeMap<String, Vec<u8>>),
}

/// The SHA-256 of `bytes` in lowercase hex.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The SHA-256 of each of `files`, by name.
pub(crate) fn hashes_of(files: &BTreeMap<String, Vec<u8>>) -> FileHashes {
    files
        .iter()
        .map(|(name, bytes)| (name.clone(), sha256_hex(bytes)))
        .collect()
}

/// The manifest in `bytes`, checked.
pub(crate) fn parse_manifest(bytes: &[u8]) -> Result<Manifest, InstallError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        InstallError::Manifest(otty_plugin_manifest::ManifestError::Shape(
            "it is not UTF-8 text".to_string(),
        ))
    })?;

    Ok(Manifest::parse(text)?)
}

/// The release file `asset`, at most `limit` bytes.
pub(crate) fn download(
    http: &dyn HttpClient,
    asset: &ReleaseAsset,
    limit: u64,
) -> Result<Vec<u8>, InstallError> {
    http.get(asset.url(), limit).map_err(|error| match error {
        HttpError::TooLarge(_) => InstallError::TooLarge {
            name: asset.name().to_string(),
            limit,
        },
        HttpError::RateLimited => InstallError::RateLimited,
        _ => InstallError::Download {
            name: asset.name().to_string(),
            reason: error.to_string(),
        },
    })
}

/// The manifest, the `.wasm` and each icon the manifest names, once each.
fn needed_files(manifest: &Manifest) -> Vec<String> {
    let mut names = vec![
        MANIFEST_FILE.to_string(),
        manifest.plugin().wasm().to_string(),
    ];
    let icons = manifest
        .agents()
        .iter()
        .map(|agent| agent.icon())
        .chain(manifest.providers().iter().map(|provider| provider.icon()));
    for icon in icons {
        if !names.iter().any(|name| name == icon) {
            names.push(icon.to_string());
        }
    }

    names
}

/// The size limit of the file `name` of the plugin of `manifest`.
fn limit_of(manifest: &Manifest, name: &str) -> u64 {
    if name == MANIFEST_FILE {
        MAX_MANIFEST_BYTES
    } else if name == manifest.plugin().wasm() {
        MAX_WASM_BYTES
    } else {
        MAX_ICON_BYTES
    }
}

/// The file `name` of `folder`, refused when larger than `limit`.
fn read_limited(
    folder: &Path,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, InstallError> {
    let path = folder.join(name);
    let file = fs::File::open(&path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => {
            InstallError::MissingFile(name.to_string())
        },
        _ => InstallError::disk(&format!("read {}", path.display()), &error),
    })?;

    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            InstallError::disk(&format!("read {}", path.display()), &error)
        })?;
    if bytes.len() as u64 > limit {
        return Err(InstallError::TooLarge {
            name: name.to_string(),
            limit,
        });
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, example_files, write_folder};

    /// A local folder the made-up plugin of a test comes from.
    fn test_folder() -> Source {
        Source::Local {
            folder: std::path::PathBuf::from("/work/example"),
        }
    }

    #[test]
    fn a_folder_gives_its_manifest_wasm_and_icons() {
        let folder = TempDir::new();
        write_folder(folder.path(), &example_files("0.1.0"));
        std::fs::write(folder.path().join("notes.txt"), b"not needed")
            .expect("write");

        let package = Package::of_folder(folder.path()).expect("a plugin");
        let files = package
            .fetch(&crate::http::MockHttpClient::new())
            .expect("files");

        assert_eq!(package.id(), "example");
        assert_eq!(package.tag(), "0.1.0");
        assert!(matches!(package.source(), Source::Local { .. }));
        assert_eq!(
            files.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "example-agent.svg",
                "example-cloud.svg",
                "example.wasm",
                MANIFEST_FILE
            ]
        );
    }

    #[test]
    fn a_folder_without_an_icon_is_refused() {
        let folder = TempDir::new();
        let mut files = example_files("0.1.0");
        files.remove("example-cloud.svg");
        write_folder(folder.path(), &files);

        let error = Package::of_folder(folder.path()).expect_err("refused");

        assert_eq!(
            error,
            InstallError::MissingFile("example-cloud.svg".to_string())
        );
    }

    #[test]
    fn a_large_icon_is_refused() {
        let mut files = example_files("0.1.0");
        files.insert("example-agent.svg".to_string(), vec![b' '; 17 * 1024]);
        let package =
            Package::of_files(files, test_folder()).expect("a plugin");

        let error = package
            .fetch(&crate::http::MockHttpClient::new())
            .expect_err("refused");

        assert_eq!(
            error,
            InstallError::TooLarge {
                name: "example-agent.svg".to_string(),
                limit: MAX_ICON_BYTES,
            }
        );
    }

    #[test]
    fn an_icon_with_a_script_is_refused() {
        let mut files = example_files("0.1.0");
        files.insert(
            "example-cloud.svg".to_string(),
            br#"<svg xmlns="http://www.w3.org/2000/svg"><script>x()</script></svg>"#.to_vec(),
        );
        let package =
            Package::of_files(files, test_folder()).expect("a plugin");

        let error = package
            .fetch(&crate::http::MockHttpClient::new())
            .expect_err("refused");

        assert!(matches!(error, InstallError::BadIcon { .. }), "{error}");
    }

    #[test]
    fn the_hash_is_lowercase_hex_sha256() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
