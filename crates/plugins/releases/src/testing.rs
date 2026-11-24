//! Test-only values shared by the tests of this crate: temporary folders,
//! the test copy of the template plugin in `assets/plugins/example/`, a
//! made-up GitHub behind a mocked [`HttpClient`] and a compiler that
//! accepts every plugin.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use otty_plugin_manifest::Manifest;
use serde_json::json;

use crate::compiler::MockPluginCompiler;
use crate::http::{HttpError, MockHttpClient};
use crate::installer::{Installer, Places, parts_of};
use crate::package::MANIFEST_FILE;
use crate::plugins_file::InstalledPart;

/// The built copy of the template plugin, as `plugins/build.sh` makes it.
const EXAMPLE_FOLDER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../assets/plugins/example"
);

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

/// What each address of the made-up GitHub answers.
type Bodies = BTreeMap<String, Result<Vec<u8>, HttpError>>;

/// A folder of its own for one test, removed when dropped.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn new() -> Self {
        let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let name =
            format!("otty-plugin-releases-{}-{number}", std::process::id());
        let path = std::env::temp_dir().join(name);

        std::fs::create_dir_all(&path).expect("create test dir");

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A made-up GitHub: the bodies of its addresses, shared with every
/// mocked client made from it, so a test can publish a new release
/// between two checks.
#[derive(Clone, Default)]
pub(crate) struct FakeGithub {
    bodies: Arc<Mutex<Bodies>>,
}

impl FakeGithub {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Publishes `releases` as the whole releases list of `repo`
    /// (`owner/name`), with their files.
    pub(crate) fn publish(&self, repo: &str, releases: Vec<FakeRelease>) {
        let mut bodies =
            self.bodies.lock().unwrap_or_else(PoisonError::into_inner);
        let mut list = Vec::new();

        for release in releases {
            let mut assets = Vec::new();
            for (name, bytes) in release.files {
                let url = format!(
                    "https://github.com/{repo}/releases/download/{}/{name}",
                    release.tag
                );
                assets
                    .push(json!({ "name": name, "browser_download_url": url }));
                bodies.insert(url, Ok(bytes));
            }
            list.push(json!({
                "tag_name": release.tag,
                "prerelease": release.prerelease,
                "assets": assets,
            }));
        }

        let api = format!(
            "https://api.github.com/repos/{repo}/releases?per_page=100"
        );
        let body = serde_json::to_vec(&list).expect("JSON");
        bodies.insert(api, Ok(body));
    }

    /// Makes the address `url` answer `error`.
    pub(crate) fn fail(&self, url: &str, error: HttpError) {
        let mut bodies =
            self.bodies.lock().unwrap_or_else(PoisonError::into_inner);

        bodies.insert(url.to_string(), Err(error));
    }

    /// A mocked client that answers from this GitHub; unknown addresses
    /// are not found.
    pub(crate) fn client(&self) -> MockHttpClient {
        let bodies = Arc::clone(&self.bodies);
        let mut client = MockHttpClient::new();

        client.expect_get().returning(move |url, limit| {
            let bodies = bodies.lock().unwrap_or_else(PoisonError::into_inner);
            match bodies.get(url) {
                Some(Ok(body)) if body.len() as u64 > limit => {
                    Err(HttpError::TooLarge(limit))
                },
                Some(answer) => answer.clone(),
                None => Err(HttpError::NotFound),
            }
        });

        client
    }
}

/// One made-up release: its tag and files.
pub(crate) struct FakeRelease {
    tag: String,
    prerelease: bool,
    files: BTreeMap<String, Vec<u8>>,
}

impl FakeRelease {
    /// A release of the template plugin with the tag `tag` and the
    /// manifest version `version`.
    pub(crate) fn example(tag: &str, version: &str) -> Self {
        Self {
            tag: tag.to_string(),
            prerelease: false,
            files: example_files(version),
        }
    }

    /// The same release, marked a pre-release on GitHub.
    pub(crate) fn marked_prerelease(mut self) -> Self {
        self.prerelease = true;

        self
    }

    /// The same release with the file `name` replaced.
    pub(crate) fn with_file(mut self, name: &str, bytes: &[u8]) -> Self {
        self.files.insert(name.to_string(), bytes.to_vec());

        self
    }

    /// The same release without the file `name`.
    pub(crate) fn without_file(mut self, name: &str) -> Self {
        self.files.remove(name);

        self
    }

    /// The same release with `from` in its manifest replaced by `to`.
    pub(crate) fn with_manifest_edit(mut self, from: &str, to: &str) -> Self {
        let manifest = self.files.get(MANIFEST_FILE).expect("a manifest");
        let text = String::from_utf8(manifest.clone()).expect("text");
        assert!(text.contains(from), "the manifest holds {from:?}");

        self.files.insert(
            MANIFEST_FILE.to_string(),
            text.replacen(from, to, 1).into_bytes(),
        );

        self
    }
}

/// The release `release` whose `.wasm` exports the `agents.agent`
/// contract 0.2, which this OTTY does not know.
pub(crate) fn needs_agent_0_2(release: FakeRelease) -> FakeRelease {
    release.with_file("example.wasm", &wasm_with_agent_0_2())
}

/// The template's `.wasm` with the agent contract 0.2 in place of 0.1: the
/// version is part of each name of the contract's interface, so every
/// name is changed, in bytes of the same length.
pub(crate) fn wasm_with_agent_0_2() -> Vec<u8> {
    let old = b"otty:agents-agent/agent@0.1.0";
    let new = b"otty:agents-agent/agent@0.2.0";
    let mut bytes = example_wasm();

    let mut start = 0;
    while let Some(found) = bytes[start..]
        .windows(old.len())
        .position(|window| window == old)
    {
        let at = start + found;
        bytes[at..at + new.len()].copy_from_slice(new);
        start = at + new.len();
    }

    bytes
}

/// The template's `.wasm` built again: the same component in other bytes,
/// with an empty custom section `rebuilt` at its end.
pub(crate) fn rebuilt_wasm() -> Vec<u8> {
    let mut bytes = example_wasm();

    // Section id 0 (custom), 8 bytes long: the name's length, then the name.
    bytes.extend_from_slice(&[0, 8, 7]);
    bytes.extend_from_slice(b"rebuilt");

    bytes
}

/// The smallest test plugin of `otty-plugin-host`: its `.wasm` exports the
/// agent contract only.
pub(crate) fn agent_only_wasm() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../host/tests/fixtures/minimal.wasm"
    ))
    .expect("a test plugin")
}

/// The template's `.wasm`.
pub(crate) fn example_wasm() -> Vec<u8> {
    std::fs::read(Path::new(EXAMPLE_FOLDER).join("example.wasm"))
        .expect("the template's .wasm")
}

/// The releases list body of `releases`: (tag, pre-release, file names).
pub(crate) fn releases_json(releases: &[(&str, bool, &[&str])]) -> Vec<u8> {
    let list: Vec<_> = releases
        .iter()
        .map(|(tag, prerelease, names)| {
            let assets: Vec<_> = names
                .iter()
                .map(|name| {
                    json!({
                        "name": name,
                        "browser_download_url": format!("https://example.test/{tag}/{name}"),
                    })
                })
                .collect();
            json!({ "tag_name": tag, "prerelease": prerelease, "assets": assets })
        })
        .collect();

    serde_json::to_vec(&list).expect("JSON")
}

/// The manifest of the template plugin with the version `version`.
pub(crate) fn template_manifest(version: &str) -> String {
    crate::fixtures::example_manifest(version)
}

/// The parts of `manifest` as installed, all on.
pub(crate) fn template_parts(manifest: &Manifest) -> Vec<InstalledPart> {
    parts_of(manifest)
}

/// The files of the template plugin with the version `version`.
pub(crate) fn example_files(version: &str) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    for name in ["example.wasm", "example-agent.svg", "example-cloud.svg"] {
        let bytes = std::fs::read(Path::new(EXAMPLE_FOLDER).join(name))
            .expect("a file");
        files.insert(name.to_string(), bytes);
    }
    files.insert(
        MANIFEST_FILE.to_string(),
        template_manifest(version).into_bytes(),
    );

    files
}

/// Writes `files` into `folder`.
pub(crate) fn write_folder(folder: &Path, files: &BTreeMap<String, Vec<u8>>) {
    std::fs::create_dir_all(folder).expect("a folder");
    for (name, bytes) in files {
        std::fs::write(folder.join(name), bytes).expect("a file");
    }
}

/// A compiler that accepts every plugin and removes any compiled copy.
pub(crate) fn accepting_compiler() -> MockPluginCompiler {
    let mut compiler = MockPluginCompiler::new();
    compiler.expect_compile().returning(|_, _| Ok(()));
    compiler.expect_remove_compiled().returning(|_| Ok(()));

    compiler
}

/// The places of an installer inside `folder`.
pub(crate) fn places_in(folder: &Path) -> Places {
    Places {
        plugins_folder: folder.join("data/plugins"),
        plugins_file: folder.join("config/plugins.json"),
    }
}

/// An installer inside `folder` that talks to `github` and accepts every
/// plugin.
pub(crate) fn test_installer(folder: &Path, github: &FakeGithub) -> Installer {
    Installer::open(
        places_in(folder),
        Box::new(github.client()),
        Box::new(accepting_compiler()),
    )
    .expect("an installer")
}
