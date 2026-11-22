//! Test-only values shared by the tests of this crate: a manifest, the
//! built test plugins and temporary folders.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use otty_plugin_manifest::{Environment, Manifest, PartRef, Permissions};

use crate::agents::agent::{Command, Identity, LoginKind, ReadError};
use crate::agents::provider::{
    AccountFacts, AccountNumbers, FoundProfile, LoginFacts,
};
use crate::calls::{CallBatch, CallKind, CallState};
use crate::engine::PluginEngine;
use crate::https::MockHttpsSend;
use crate::keys::{KeySource, KeyText};
use crate::operations::v0_1::otty::host::types::HostError;
use crate::outside::Outside;
use crate::plugin::LoadedPlugin;
use crate::system_keyring::MockKeyringRead;

/// The manifest of a test plugin with one agent part, `[api] agent =
/// "0.1"`, the contract every test plugin exports.
pub(crate) const TEST_PLUGIN_MANIFEST: &str = r#"
[plugin]
id = "test-plugin"
name = "Test plugin"
version = "1.0.0"
wasm = "plugin.wasm"

[api]
agents.agent = "0.1"

[[agents.agent]]
id = "test-agent"
name = "Test agent"
icon = "test.svg"

[agents.agent.permissions]
read = ["~/.test-agent/**"]
"#;

/// The manifest of a test plugin whose agent part declares keys in every
/// kind of place, and two provider parts, each with its own host. The
/// template plugin's `.wasm` loads with it: it exports both contracts and
/// the language receiver.
pub(crate) const KEYS_PLUGIN_MANIFEST: &str = r#"
[plugin]
id = "test-plugin"
name = "Test plugin"
version = "1.0.0"
wasm = "plugin.wasm"

[api]
agents.agent = "0.1"
agents.provider = "0.1"
language = "0.1"

[[agents.agent]]
id = "test-agent"
name = "Test agent"
icon = "test.svg"

[agents.agent.permissions]
read = ["~/.test-agent/**"]
sqlite = ["~/.test-agent/*.db"]

[[agents.agent.permissions.keys]]
files = ["~/.test-agent/auth.json"]
fields = ["/*/key", "/token"]

[[agents.agent.permissions.keys]]
sqlite = ["~/.test-agent/*.db"]
table = "credential"
column = "value"
fields = ["/key"]

[[agents.agent.permissions.keys]]
sqlite = ["~/.test-agent/*.db"]
table = "api_key"
column = "secret"

[[agents.agent.permissions.keys]]
env = ["TEST_AGENT_KEY"]

[[agents.agent.permissions.keyring]]
service = "test-agent"
user = "work"
fields = ["/key"]

[[agents.agent.permissions.keyring]]
service = "test-agent"
user = "plain"

[[agents.provider]]
id = "test-cloud"
name = "Test cloud"
icon = "cloud.svg"
hosts = ["example.com"]

[agents.provider.permissions]
http = ["api.example.com"]

[[agents.provider]]
id = "other-cloud"
name = "Other cloud"
icon = "other.svg"
hosts = ["example.org"]

[agents.provider.permissions]
http = ["api.example.org"]
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

/// The manifest of the host operation tests on files and databases: two
/// agent parts, `alpha` and `beta`, each with its own folder in the home
/// folder and its own variables. `alpha` keeps keys in the column
/// `credential.value` and in the field `/key` of the JSON column
/// `login.data`.
pub(crate) const FILES_MANIFEST: &str = r#"
[plugin]
id = "files-test"
name = "Files test"
version = "1.0.0"
wasm = "plugin.wasm"

[api]
agents.agent = "0.1"

[[agents.agent]]
id = "alpha"
name = "Alpha"
icon = "alpha.svg"

[agents.agent.permissions]
read = ["~/.alpha/**", "{ALPHA_HOME}/**"]
sqlite = ["~/.alpha/*.db"]
env = ["ALPHA_HOME", "ALPHA_TOKEN"]

[[agents.agent.permissions.keys]]
files = ["~/.alpha/auth.json"]
fields = ["/key"]

[[agents.agent.permissions.keys]]
sqlite = ["~/.alpha/*.db"]
table = "credential"
column = "value"

[[agents.agent.permissions.keys]]
sqlite = ["~/.alpha/*.db"]
table = "login"
column = "data"
fields = ["/key"]

[[agents.agent.permissions.keys]]
env = ["ALPHA_TOKEN"]

[[agents.agent]]
id = "beta"
name = "Beta"
icon = "beta.svg"

[agents.agent.permissions]
read = ["~/.beta/**"]
env = ["BETA_HOME"]
"#;

/// The permissions of the part `id` of [`FILES_MANIFEST`] in `env`.
pub(crate) fn files_permissions(id: &str, env: &Environment) -> Permissions {
    let manifest = Manifest::parse(FILES_MANIFEST).expect("valid");
    let declared = manifest
        .permissions_of(&PartRef::agent(id))
        .expect("a listed part");

    Permissions::new(declared, env)
}

/// `result` with its host error in debug form, to compare: the host
/// errors of the bindings have no `PartialEq`.
pub(crate) fn debug_err<T>(result: Result<T, HostError>) -> Result<T, String> {
    result.map_err(|error| format!("{error:?}"))
}

/// Writes `bytes` to `path`, making its folders.
pub(crate) fn write_file(path: &Path, bytes: &[u8]) {
    let folder = path.parent().expect("a folder");

    std::fs::create_dir_all(folder).expect("create the folder");
    std::fs::write(path, bytes).expect("write the file");
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

/// An engine whose plugins reach `https` and `keyring` in place of the
/// network and the system keyring.
pub(crate) fn engine_with(
    cache: &TempDir,
    https: MockHttpsSend,
    keyring: MockKeyringRead,
) -> PluginEngine {
    let outside = Outside::new(Box::new(https), Box::new(keyring));

    PluginEngine::with_outside(cache.path().to_path_buf(), outside)
        .expect("engine")
}

/// The template plugin's `.wasm` loaded with [`KEYS_PLUGIN_MANIFEST`] by
/// `engine` for `env`.
pub(crate) fn load_keys_plugin(
    engine: &PluginEngine,
    env: &Environment,
) -> LoadedPlugin {
    let manifest = Manifest::parse(KEYS_PLUGIN_MANIFEST).expect("valid");

    engine
        .load(manifest, &fixture("template"), env)
        .expect("the test plugin loads")
}

/// Runs `run` with the call state of one call into `part` of `plugin`,
/// as a host operation of that call gets it.
pub(crate) fn in_call<T>(
    plugin: &LoadedPlugin,
    part: &PartRef,
    run: impl FnOnce(&mut CallState) -> T,
) -> T {
    let mut batch = plugin.start_calls(part).expect("a listed part");

    batch
        .call(CallKind::Quick, |store, _| Ok(run(store.data_mut())))
        .expect("the call")
}

/// The manifest of the `programs` test plugin: the agent part
/// `test-agent` runs, talks to and starts in a hidden terminal the
/// stand-in program `stand-in`, passes `STAND_IN_HOME`, and removes files
/// in `~/.test-agent/probe/`; the agent part `other-agent` runs only
/// `other --version`. The provider parts `test-provider` and
/// `other-provider` declare the same programs, and read the numbers of an
/// account by running them (issue 29).
pub(crate) const PROGRAMS_MANIFEST: &str = r#"
[plugin]
id = "test-plugin"
name = "Test plugin"
version = "1.0.0"
wasm = "plugin.wasm"

[api]
agents.agent = "0.1"
agents.provider = "0.1"

[[agents.agent]]
id = "test-agent"
name = "Test agent"
icon = "test.svg"

[agents.agent.permissions]
write = ["~/.test-agent/probe/**"]
env = ["STAND_IN_HOME", "STAND_IN_KEY"]

[[agents.agent.permissions.run]]
program = "stand-in"
args = ["--version"]

[[agents.agent.permissions.run]]
program = "stand-in"
args = ["serve", "--profile", "*"]

[[agents.agent.permissions.terminal]]
program = "stand-in"
args = ["tui"]

[[agents.agent.permissions.keys]]
env = ["STAND_IN_KEY"]

[[agents.agent]]
id = "other-agent"
name = "Other agent"
icon = "other.svg"

[[agents.agent.permissions.run]]
program = "other"
args = ["--version"]

[[agents.provider]]
id = "test-provider"
name = "Test provider"
icon = "test.svg"
hosts = ["test.example.com"]

[agents.provider.permissions]
env = ["STAND_IN_HOME"]

[[agents.provider.permissions.run]]
program = "stand-in"
args = ["--version"]

[[agents.provider.permissions.run]]
program = "stand-in"
args = ["serve", "--profile", "*"]

[[agents.provider.permissions.terminal]]
program = "stand-in"
args = ["tui"]

[[agents.provider]]
id = "other-provider"
name = "Other provider"
icon = "other.svg"
hosts = ["other.example.com"]

[[agents.provider.permissions.run]]
program = "other"
args = ["--version"]
"#;

/// The permissions of `part` of [`PROGRAMS_MANIFEST`] for `env`.
pub(crate) fn programs_permissions(
    part: &str,
    env: &Environment,
) -> Arc<Permissions> {
    let manifest = Manifest::parse(PROGRAMS_MANIFEST).expect("valid");
    let declared = manifest
        .permissions_of(&PartRef::agent(part))
        .expect("a listed part");

    Arc::new(Permissions::new(declared, env))
}

/// `texts` as owned strings, such as the arguments of a program.
pub(crate) fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_string()).collect()
}

/// Asserts that `actual` prints as `expected`: the host's WIT types, such
/// as `HostError`, have no `PartialEq`.
#[track_caller]
pub(crate) fn assert_same<T: std::fmt::Debug>(actual: T, expected: T) {
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
}

/// The numbers the provider part of `batch` of the `programs` test plugin
/// reads for an account found in the profile in `folder`, the way `way`
/// names, such as `run`: the test plugin takes the way from the account
/// id, and puts what the program answered in the plan.
// A `ReadError` may hold a need, a record the contract can not box.
#[allow(clippy::result_large_err)]
pub(crate) fn read_by_way(
    batch: &mut CallBatch<'_>,
    folder: &str,
    way: &str,
) -> Result<AccountNumbers, ReadError> {
    let identity = Identity {
        account_id: Some(way.to_string()),
        organization_id: None,
        email: None,
    };
    let found = LoginFacts {
        kind: LoginKind::Subscription,
        identity: identity.clone(),
        plan: None,
        found_in: vec![FoundProfile {
            plugin: "test-plugin".to_string(),
            agent: "test-agent".to_string(),
            folder: folder.to_string(),
            launch: Command {
                program: "stand-in".to_string(),
                args: Vec::new(),
                env: Vec::new(),
            },
        }],
    };
    let account = AccountFacts {
        identity,
        plan: None,
    };
    let key = KeySource::Given(KeyText::new("made-up-key").expect("a key"));

    batch
        .provider()
        .expect("a provider part")
        .numbers(&found, &account, &key, &[])
        .expect("the call")
}

/// What the program answered, as [`read_by_way`] gave it: the plan, or
/// the error of the part.
// A `ReadError` may hold a need, a record the contract can not box.
#[allow(clippy::result_large_err)]
pub(crate) fn plan_of(
    numbers: Result<AccountNumbers, ReadError>,
) -> Result<String, String> {
    numbers
        .map(|numbers| numbers.plan.unwrap_or_default())
        .map_err(|error| format!("{error:?}"))
}

/// The `programs` test plugin loaded with [`PROGRAMS_MANIFEST`] by
/// `engine`.
pub(crate) fn load_programs_plugin(engine: &PluginEngine) -> LoadedPlugin {
    let manifest = Manifest::parse(PROGRAMS_MANIFEST).expect("valid");

    engine
        .load(manifest, &fixture("programs"), &test_env())
        .expect("the test plugin loads")
}
