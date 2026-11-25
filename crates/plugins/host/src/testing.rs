//! Test-only values shared by the tests of this crate: a manifest, the
//! built test plugins and temporary folders.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use otty_plugin_manifest::{Environment, Manifest, PartRef, Permissions};

use crate::agents::agent::ReadError;
use crate::agents::prices::{MockPriceList, PriceList};
use crate::agents::provider::{
    Account, AccountKind, AccountNumbers, LoginFolder,
};
use crate::calls::{CallBatch, CallKind, CallState};
use crate::engine::PluginEngine;
use crate::https::MockHttpsSend;
use crate::operations::v0_1::otty::host::types::HostError;
use crate::outside::Outside;
use crate::plugin::LoadedPlugin;
use crate::system_keyring::MockKeyringRead;

/// The manifest of a test plugin with one agent part: every test plugin
/// exports the agent contract.
pub(crate) const TEST_PLUGIN_MANIFEST: &str = r#"
[plugin]
id = "test-plugin"
name = "Test plugin"
version = "1.0.0"
wasm = "plugin.wasm"

[[agents.agent]]
id = "test-agent"
name = "Test agent"
icon = "test.svg"

[agents.agent.permissions]
read = ["~/.test-agent/**"]
"#;

/// The manifest of a test plugin whose agent part declares keys in every
/// kind of place, and two provider parts, each with its own host. The
/// example test plugin's `.wasm` loads with it: it exports both contracts
/// and the language receiver.
pub(crate) const KEYS_PLUGIN_MANIFEST: &str = r#"
[plugin]
id = "test-plugin"
name = "Test plugin"
version = "1.0.0"
wasm = "plugin.wasm"

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

/// The built test plugin `name` with the agent contract 0.2 in place of
/// 0.1, written into `folder`: the version is part of each name of the
/// contract's interface, so every name is changed, in bytes of the same
/// length.
pub(crate) fn fixture_with_agent_0_2(name: &str, folder: &Path) -> PathBuf {
    let old = b"otty:agents-agent/agent@0.1.0";
    let new = b"otty:agents-agent/agent@0.2.0";
    let mut bytes = std::fs::read(fixture(name)).expect("a test plugin");

    let mut start = 0;
    while let Some(found) = bytes[start..]
        .windows(old.len())
        .position(|window| window == old)
    {
        let at = start + found;
        bytes[at..at + new.len()].copy_from_slice(new);
        start = at + new.len();
    }

    let path = folder.join(format!("{name}-agent-0.2.wasm"));
    std::fs::write(&path, bytes).expect("write the test plugin");
    path
}

/// The built example test plugin, as an installed plugin folder holds it:
/// `tests/fixtures/example/`, made from `tests/example/` by
/// `scripts/build-test-plugins.sh`.
pub(crate) fn example_copy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/example")
}

/// The `.wasm` of the built example test plugin.
pub(crate) fn example_wasm() -> PathBuf {
    example_copy().join("example.wasm")
}

/// The environment of the example plugin's tests: `tests/example/home/`
/// is the home folder.
pub(crate) fn example_env() -> Environment {
    Environment::new(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/example/home"),
    )
}

/// The built example test plugin loaded with its own manifest by `engine`
/// for `env`.
pub(crate) fn load_example(
    engine: &PluginEngine,
    env: &Environment,
) -> LoadedPlugin {
    let text = std::fs::read_to_string(example_copy().join("otty-plugin.toml"))
        .expect("the example's manifest");
    let manifest = Manifest::parse(&text).expect("a valid manifest");

    engine
        .load(manifest, &example_wasm(), env)
        .expect("the example loads")
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

/// The example test plugin's `.wasm` loaded with [`KEYS_PLUGIN_MANIFEST`] by
/// `engine` for `env`.
pub(crate) fn load_keys_plugin(
    engine: &PluginEngine,
    env: &Environment,
) -> LoadedPlugin {
    let manifest = Manifest::parse(KEYS_PLUGIN_MANIFEST).expect("valid");

    engine
        .load(manifest, &example_wasm(), env)
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
/// names, such as `run`: the test plugin takes the way from the account's
/// name, and puts what the program answered in the plan. The account's id
/// is its first profile, so the pace of its readings is kept per profile.
// A `ReadError` may hold a need, a record the contract can not box.
#[allow(clippy::result_large_err)]
pub(crate) fn read_by_way(
    batch: &mut CallBatch<'_>,
    folder: &str,
    way: &str,
) -> Result<AccountNumbers, ReadError> {
    read_in_profiles(batch, &[("test-agent", folder)], way)
}

/// The numbers the provider part of `batch` of the `programs` test plugin
/// reads, the way `way` names, for an account found in `profiles`, each
/// the id of its agent part and its folder. The test plugin reads in the
/// first profile, or in no folder when there is none.
// A `ReadError` may hold a need, a record the contract can not box.
#[allow(clippy::result_large_err)]
pub(crate) fn read_in_profiles(
    batch: &mut CallBatch<'_>,
    profiles: &[(&str, &str)],
    way: &str,
) -> Result<AccountNumbers, ReadError> {
    let found_in: Vec<LoginFolder> = profiles
        .iter()
        .map(|(agent, folder)| {
            LoginFolder::Profile((agent.to_string(), folder.to_string()))
        })
        .collect();
    let id = profiles
        .first()
        .map(|(agent, folder)| format!("{agent} {folder}"))
        .unwrap_or_default();
    let account = Account {
        id,
        name: way.to_string(),
        kind: AccountKind::Subscription,
        plan: None,
        found_in,
        needs: Vec::new(),
    };

    batch
        .provider()
        .expect("a provider part")
        .numbers(&account, &[], &no_prices())
        .expect("the call")
}

/// A price list with no price, for a read that prices nothing.
pub(crate) fn no_prices() -> Arc<dyn PriceList> {
    let mut prices = MockPriceList::new();
    prices.expect_price_of().returning(|_, _| None);

    Arc::new(prices)
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
    load_programs_plugin_with(engine, PROGRAMS_MANIFEST)
}

/// [`PROGRAMS_MANIFEST`] whose provider part `test-provider` does not list
/// `STAND_IN_HOME` in `env`: it names it only as the `folder_env` of the
/// logins it reads in the profiles of `test-agent`.
pub(crate) fn programs_logins_manifest() -> String {
    edited(
        PROGRAMS_MANIFEST,
        "[agents.provider.permissions]\nenv = [\"STAND_IN_HOME\"]\n",
        "[[agents.provider.logins]]\n\
         agent = \"test-agent\"\n\
         folder_env = \"STAND_IN_HOME\"\n\n",
    )
}

/// The `programs` test plugin loaded with the manifest `text` by
/// `engine`.
pub(crate) fn load_programs_plugin_with(
    engine: &PluginEngine,
    text: &str,
) -> LoadedPlugin {
    let manifest = Manifest::parse(text).expect("valid");

    engine
        .load(manifest, &fixture("programs"), &test_env())
        .expect("the test plugin loads")
}
