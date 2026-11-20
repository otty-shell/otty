//! Test-only values shared by the tests of this crate: a manifest, the
//! built test plugins and temporary folders.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use otty_plugin_manifest::{Environment, Manifest};

use crate::engine::PluginEngine;
use crate::plugin::LoadedPlugin;

/// The manifest of a test plugin with one agent part, `[api] agent =
/// "0.1"`, the contract every test plugin exports.
pub(crate) const TEST_PLUGIN_MANIFEST: &str = r#"
[plugin]
id = "test-plugin"
name = "Test plugin"
version = "1.0.0"
wasm = "plugin.wasm"

[api]
agent = "0.1"

[[agent]]
id = "test-agent"
name = "Test agent"
icon = "test.svg"

[agent.permissions]
read = ["~/.test-agent/**"]
"#;

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

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
        let name = format!("otty-plugin-host-{}-{number}", std::process::id());
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

/// `text` with the first `from` replaced by `to`; `from` must be there.
pub(crate) fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the text holds {from:?}");

    text.replacen(from, to, 1)
}

/// The built test plugin `name`, such as `spin`, from `tests/fixtures/`.
pub(crate) fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{name}.wasm"))
}

/// The folder of the template plugin, `plugins/template/`.
pub(crate) fn template_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../plugins/template")
        .canonicalize()
        .expect("the template plugin")
}

/// The environment of the template's tests: its `fixtures/` is the home
/// folder.
pub(crate) fn template_env() -> Environment {
    Environment::new(template_folder().join("fixtures"))
}

/// The template plugin, built into `tests/fixtures/template.wasm`, loaded
/// with its own manifest by `engine` for `env`.
pub(crate) fn load_template(
    engine: &PluginEngine,
    env: &Environment,
) -> LoadedPlugin {
    let text =
        std::fs::read_to_string(template_folder().join("otty-plugin.toml"))
            .expect("the template's manifest");
    let manifest = Manifest::parse(&text).expect("a valid manifest");

    engine
        .load(manifest, &fixture("template"), env)
        .expect("the template loads")
}

/// A test environment whose home folder does not exist.
pub(crate) fn test_env() -> Environment {
    Environment::new(PathBuf::from("/home/test-user"))
}

/// The test plugin `name` loaded with [`TEST_PLUGIN_MANIFEST`] by
/// `engine`.
pub(crate) fn load_fixture(engine: &PluginEngine, name: &str) -> LoadedPlugin {
    let manifest = Manifest::parse(TEST_PLUGIN_MANIFEST).expect("valid");

    engine
        .load(manifest, &fixture(name), &test_env())
        .expect("the test plugin loads")
}
