//! Test-only values shared by the tests of this crate: a made-up plugin
//! written to a temporary folder for the test host.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The manifest of the made-up plugin: an agent part that reads, writes,
/// queries and keeps keys under `~/.made-up/` (and reads under the folder
/// `MADE_UP_HOME` names) and runs, talks to and
/// starts in a hidden terminal the program `made-up`, and a provider part
/// that calls `api.example.com`.
pub(crate) const MANIFEST: &str = r#"
[plugin]
id = "made-up"
name = "Made up"
version = "0.1.0"
wasm = "made-up.wasm"

[api]
agents.agent = "0.1"
agents.provider = "0.1"

[[agents.agent]]
id = "made-up-agent"
name = "Made-up agent"
icon = "agent.svg"

[agents.agent.permissions]
read = ["~/.made-up/**", "{MADE_UP_HOME}/**"]
write = ["~/.made-up/probe/*.jsonl"]
sqlite = ["~/.made-up/*.db"]
env = ["MADE_UP_HOME", "MADE_UP_KEY"]

[[agents.agent.permissions.run]]
program = "made-up"
args = ["--version"]

[[agents.agent.permissions.run]]
program = "made-up"
args = ["app-server", "--profile", "*"]

[[agents.agent.permissions.terminal]]
program = "made-up"
args = ["--tui"]

[[agents.agent.permissions.keys]]
files = ["~/.made-up/auth.json"]
fields = ["/*/key", "/token"]

[[agents.agent.permissions.keys]]
sqlite = ["~/.made-up/*.db"]
table = "credential"
column = "value"
fields = ["/key"]

[[agents.agent.permissions.keys]]
env = ["MADE_UP_KEY"]

[[agents.agent.permissions.keyring]]
service = "made-up"
user = "work"
fields = ["/key"]

[[agents.agent.permissions.keyring]]
service = "made-up"
user = "plain"

[[agents.agent.permissions.keyring]]
service = "made-up"
user = "home"
fields = ["/key"]

[[agents.provider]]
id = "made-up-cloud"
name = "Made-up cloud"
icon = "cloud.svg"
hosts = ["example.com"]

[agents.provider.permissions]
http = ["api.example.com"]
"#;

static NEXT_FOLDER: AtomicUsize = AtomicUsize::new(0);

/// The made-up plugin in a folder of its own, removed when dropped: its
/// manifest and the files of its `fixtures/`.
pub(crate) struct MadeUpPlugin {
    folder: PathBuf,
}

impl MadeUpPlugin {
    /// The plugin's folder.
    pub(crate) fn folder(&self) -> &Path {
        &self.folder
    }

    /// The plugin with no fixture yet.
    pub(crate) fn new() -> Self {
        let number = NEXT_FOLDER.fetch_add(1, Ordering::Relaxed);
        let name = format!("otty-plugin-sdk-{}-{number}", std::process::id());
        let folder = std::env::temp_dir().join(name);

        std::fs::create_dir_all(folder.join("fixtures")).expect("the folder");
        std::fs::write(folder.join("otty-plugin.toml"), MANIFEST)
            .expect("the manifest");

        Self { folder }
    }

    /// Writes the fixture `relative` of the home folder.
    pub(crate) fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.folder.join("fixtures").join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the fixture's folder");
        }

        std::fs::write(&path, bytes).expect("the fixture");

        path.canonicalize().expect("a written fixture")
    }
}

impl Drop for MadeUpPlugin {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}
