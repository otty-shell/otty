//! Test-only values shared by the tests of this crate: manifests, the
//! built test plugins and temporary folders.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::engine::PluginEngine;
use crate::environment::Environment;
use crate::manifest::Manifest;
use crate::plugin::LoadedPlugin;

/// The manifest of the spec's OpenAI plugin: one agent part and one
/// provider part.
pub(crate) const OPENAI_MANIFEST: &str = r#"
[plugin]
id = "openai"
name = "OpenAI"
version = "0.4.0"
description = "Codex sessions, tokens and limits; ChatGPT and OpenAI account numbers."
repository = "https://github.com/otty-shell/otty-plugin-openai"
license = "Apache-2.0"
authors = ["OTTY team"]
wasm = "openai.wasm"

[api]
agent = "0.1"
provider = "0.1"

[[agent]]
id = "codex"
name = "Codex"
icon = "codex.svg"
program = "codex"
tested_version = "0.169.0"

[agent.permissions]
read = ["~/.codex*/**", "{CODEX_HOME}/**"]
env = ["CODEX_HOME"]

[[agent.permissions.run]]
program = "codex"
args = ["--version"]

[[agent.permissions.run]]
program = "codex"
args = ["-s", "read-only", "-a", "never", "app-server"]

[[agent.permissions.keys]]
files = ["~/.codex*/auth.json", "{CODEX_HOME}/auth.json"]
fields = ["/OPENAI_API_KEY", "/tokens/access_token"]

[[provider]]
id = "openai"
name = "OpenAI"
icon = "openai.svg"
hosts = ["openai.com", "chatgpt.com"]
usage_page = "https://platform.openai.com/usage"
add_by_hand = "admin-key"

[provider.permissions]
http = ["chatgpt.com", "api.openai.com"]
"#;

/// The manifest of the spec's OpenCode plugin: one agent part with a
/// database and a key column.
pub(crate) const OPENCODE_MANIFEST: &str = r#"
[plugin]
id = "opencode"
name = "OpenCode"
version = "0.2.0"
wasm = "opencode.wasm"

[api]
agent = "0.1"

[[agent]]
id = "opencode"
name = "OpenCode"
icon = "opencode.svg"
program = "opencode"

[agent.permissions]
read = ["~/.local/share/opencode/**", "{XDG_DATA_HOME}/opencode/**"]
sqlite = ["~/.local/share/opencode/*.db", "{XDG_DATA_HOME}/opencode/*.db"]
env = ["XDG_DATA_HOME"]

[[agent.permissions.keys]]
sqlite = ["~/.local/share/opencode/*.db", "{XDG_DATA_HOME}/opencode/*.db"]
table = "credential"
column = "value"
fields = ["/key", "/access", "/refresh"]
"#;

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
