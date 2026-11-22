//! Test-only values and mocks shared by the tests of this crate.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use jiff::tz::TimeZone;
use mockall::mock;
use otty_plugin_host::agents::agent::Login;
use otty_plugin_host::{
    Environment, LoadedPlugin, Manifest, PluginEngine, PluginPart,
};

use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
};
use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder};
use crate::command::AgentCommand;
use crate::dollars::Dollars;
use crate::env::AgentEnv;
use crate::fixtures::stand_in::{default_config_dir, profile_command};
use crate::key_fingerprint::KeyFingerprint;
use crate::local_time::LocalNow;
use crate::part_need::PartNeed;
use crate::price_table::TokenPrices;
use crate::profile::{Profile, ProfileStatus};
use crate::provider::ProviderRef;
use crate::session::{AgentSession, SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::spend::SpendCounter;
use crate::stats::StatsSource;

/// The "now" of the tests: 2026-11-16T10:00:00Z, in seconds since 1970.
pub(crate) const NOW: u64 = 1_794_823_200;

/// The config folder of the made-up agent Coder: `~/.coder`, moved by
/// `CODER_HOME`.
const CODER_FOLDER: ConfigFolder = ConfigFolder::new(".coder", "CODER_HOME");

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

mock! {
    /// A mock agent.
    ///
    /// It is written with `mock!` instead of `automock` on the trait
    /// because `mockall` needs a named lifetime for the reference inside
    /// the `Option` of `launch`, and clippy rejects that lifetime on the
    /// trait itself.
    pub(crate) Agent {}

    impl Agent for Agent {
        fn entry(&self) -> &AgentEntry;
        fn config_folder(&self) -> ConfigFolder;
        fn writes_cost(&self) -> bool;
        fn default_config_dir(&self, env: &AgentEnv) -> PathBuf;
        fn detect(&self, profile: &Profile) -> ProfileStatus;
        fn accounts(&self, profile: &Profile) -> Vec<Account>;
        fn has_sessions(&self, profile: &Profile) -> bool;
        fn version(&self, profile: &Profile) -> Option<String>;
        fn launch<'a>(
            &self,
            profile: &Profile,
            session: Option<&'a SessionId>,
        ) -> AgentCommand;
        fn session_sources(&self) -> Vec<Box<dyn SessionSource>>;
        fn session_source_ids(&self) -> &'static [SourceId];
        fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;
        fn stats_source(&self) -> Option<Box<dyn StatsSource>>;
        fn found_login(
            &self,
            profile: &Profile,
            account: &Account,
        ) -> Option<Login>;
        fn login_refused(
            &self,
            profile: &Profile,
            login: &Login,
        ) -> Option<PartNeed>;
    }
}

/// A fresh folder under the system temp folder, removed on drop.
///
/// Tests write their made-up config folders here and never touch the real
/// ones in the home folder.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn new() -> Self {
        let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let name = format!("otty-agents-{}-{number}", std::process::id());
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

/// A made-up home folder with the files of the template plugin's agent,
/// `~/.example-agent`, and the template plugin (issue 03) loaded for it
/// from its built copy in `otty-plugin-host`.
pub(crate) struct TemplateHome {
    home: TempDir,
    cache: TempDir,
}

impl TemplateHome {
    /// The home folder.
    pub(crate) fn home(&self) -> &Path {
        self.home.path()
    }

    /// The default config folder of the made-up agent.
    pub(crate) fn folder(&self) -> PathBuf {
        self.home().join(".example-agent")
    }

    /// A home whose made-up agent holds a key for `dev@example.com` and
    /// wrote its version 1.4.2, with no session yet.
    pub(crate) fn new() -> Self {
        let home = Self {
            home: TempDir::new(),
            cache: TempDir::new(),
        };

        write_file(
            &home.folder().join("auth.json"),
            r#"{"api_key": "ex-made-up-key-0001", "email": "dev@example.com"}"#,
        );
        write_file(
            &home.folder().join("state.json"),
            r#"{"last_version": "1.4.2"}"#,
        );
        home
    }

    /// The default profile of the made-up agent.
    pub(crate) fn profile(&self) -> Profile {
        let agent = AgentId::new("example-agent");

        Profile::default_for(agent, "Example Agent", self.folder())
    }

    /// Writes the session file `name` of `folder` in the home folder with
    /// `lines`, last changed `changed` seconds after the epoch.
    pub(crate) fn write_session(
        &self,
        folder: &str,
        name: &str,
        lines: &[String],
        changed: u64,
    ) {
        let path = self.home().join(folder).join("sessions").join(name);

        write_file(&path, &format!("{}\n", lines.join("\n")));
        set_file_time(&path, at(changed));
    }

    /// The template plugin, loaded by an engine of its own for this home.
    pub(crate) fn plugin(&self) -> Arc<LoadedPlugin> {
        let crate_folder = Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = std::fs::read_to_string(
            crate_folder.join("../../plugins/template/otty-plugin.toml"),
        )
        .expect("the template's manifest");
        let manifest = Manifest::parse(&manifest).expect("a valid manifest");
        let wasm =
            crate_folder.join("../plugins/host/tests/fixtures/template.wasm");
        let engine = PluginEngine::new(self.cache.path().to_path_buf())
            .expect("the engine");

        let env = Environment::new(self.home().to_path_buf());
        let plugin = engine.load(manifest, &wasm, &env).expect("the template");
        Arc::new(plugin)
    }
}

/// A made-up agent, Coder: its config folder `~/.coder` moved by
/// `CODER_HOME`, its commands, and only whether a folder holds a login
/// file (`auth.json`) or a file in `sessions/`. It reads no file and has no
/// source.
struct Coder {
    entry: AgentEntry,
}

impl Coder {
    /// `coder` of `profile`, with `CODER_HOME` when its folder is not the
    /// default one.
    fn command(&self, profile: &Profile) -> AgentCommand {
        profile_command(profile, self.entry.program(), CODER_FOLDER.variable())
    }
}

impl Agent for Coder {
    fn entry(&self) -> &AgentEntry {
        &self.entry
    }

    fn config_folder(&self) -> ConfigFolder {
        CODER_FOLDER
    }

    fn writes_cost(&self) -> bool {
        false
    }

    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf {
        default_config_dir(&CODER_FOLDER, env)
    }

    fn detect(&self, profile: &Profile) -> ProfileStatus {
        let config_dir = profile.config_dir();

        if !config_dir.is_dir() {
            return ProfileStatus::ConfigDirNotFound;
        }
        if !config_dir.join("auth.json").is_file() {
            return ProfileStatus::NotLoggedIn(None);
        }

        ProfileStatus::Connected {
            account: None,
            plan: None,
            billing: None,
        }
    }

    fn accounts(&self, _profile: &Profile) -> Vec<Account> {
        Vec::new()
    }

    fn has_sessions(&self, profile: &Profile) -> bool {
        has_a_file(&profile.config_dir().join("sessions"))
    }

    fn version(&self, _profile: &Profile) -> Option<String> {
        None
    }

    /// `coder`, `coder resume <id>` to continue a session.
    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand {
        let command = self.command(profile);

        match session {
            Some(id) => command.with_arg("resume").with_arg(id.as_str()),
            None => command,
        }
    }

    fn session_sources(&self) -> Vec<Box<dyn SessionSource>> {
        Vec::new()
    }

    fn session_source_ids(&self) -> &'static [SourceId] {
        &[]
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        None
    }

    fn stats_source(&self) -> Option<Box<dyn StatsSource>> {
        None
    }
}

/// The agents of the tests of this crate: [`Coder`], read by a made-up
/// plugin part.
pub(crate) fn test_agents() -> Vec<Box<dyn Agent>> {
    let entry = AgentEntry::new(
        AgentId::new("coder"),
        "Coder",
        "coder",
        "Example",
        made_up_part("coder"),
    )
    .with_icon(b"<svg/>".as_slice());

    vec![Box::new(Coder { entry })]
}

/// The part `part` of the made-up plugin `example`.
pub(crate) fn made_up_part(part: &str) -> PluginPart {
    PluginPart::new("example", "Example", "0.1.0", "", part)
}

/// The `meta` record of a session of the template's made-up agent in the
/// project `cwd`.
pub(crate) fn meta_line(cwd: &str) -> String {
    format!(
        r#"{{"type":"meta","version":"1.4.2","cwd":"{cwd}","title":"Made up"}}"#
    )
}

/// An `answer` record of the template's made-up agent with the id `id`,
/// written `seconds` after the epoch, with 100 input tokens and `output`
/// output tokens.
pub(crate) fn answer_line(id: &str, seconds: u64, output: u64) -> String {
    let at_ms = seconds * 1000;

    format!(
        r#"{{"type":"answer","id":"{id}","at_ms":{at_ms},"model":"example-large","tokens":{{"input":100,"cached":0,"output":{output}}}}}"#
    )
}

/// `amount` whole dollars.
pub(crate) fn dollars(amount: u64) -> Dollars {
    Dollars::from_micro_dollars(amount * 1_000_000)
}

/// Prices per million tokens of input, cached input, input written to the
/// cache and output, each in micro-dollars: `1_250_000` is $1.25.
pub(crate) fn token_prices(
    input: u64,
    cached: u64,
    cache_written: u64,
    output: u64,
) -> TokenPrices {
    TokenPrices {
        input: Dollars::from_micro_dollars(input),
        cached: Dollars::from_micro_dollars(cached),
        cache_written: Dollars::from_micro_dollars(cache_written),
        output: Dollars::from_micro_dollars(output),
    }
}

/// The entry of a made-up agent with the id `id`, whose program is
/// `program`.
pub(crate) fn mock_entry(id: &str, program: &str) -> AgentEntry {
    AgentEntry::new(
        AgentId::new(id),
        id,
        program,
        "Made-up Labs",
        made_up_part(id),
    )
}

/// A made-up default profile of a made-up agent.
pub(crate) fn profile(name: &str) -> Profile {
    let agent = AgentId::new("test-agent");
    let config_dir = PathBuf::from(format!("/home/user/.config-{name}"));

    Profile::default_for(agent, name, config_dir)
}

/// A made-up session in `folder` that last changed `seconds` after the
/// epoch.
pub(crate) fn session(id: &str, folder: &str, seconds: u64) -> AgentSession {
    AgentSession::new(
        SessionId::new(id),
        PathBuf::from(folder),
        format!("Session {id}"),
        at(seconds),
    )
}

/// Writes `content` to `path`, creating the folders on the way.
pub(crate) fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create test folders");
    }

    std::fs::write(path, content).expect("write test file");
}

/// Writes a made-up program to `path` that only ends, which its owner
/// may run, creating the folders on the way.
#[cfg(unix)]
pub(crate) fn write_program(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    write_file(path, "#!/bin/sh\nexit 0\n");
    let permissions = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("make it a program");
}

/// The time `seconds` after the epoch.
pub(crate) fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

/// The "now" of the tests, [`NOW`], in UTC.
pub(crate) fn utc_now() -> LocalNow {
    LocalNow::new(at(NOW), TimeZone::UTC)
}

/// A time zone `hours` east of UTC, with no summer time.
pub(crate) fn hours_east(hours: i8) -> TimeZone {
    TimeZone::fixed(jiff::tz::offset(hours))
}

/// Berlin: UTC+1 in winter and UTC+2 in summer, from the last Sunday of
/// March to the last Sunday of October. It is made from its POSIX rule, so
/// no test depends on the time zone files of the machine.
pub(crate) fn berlin() -> TimeZone {
    TimeZone::posix("CET-1CEST,M3.5.0,M10.5.0/3").expect("the rule of Berlin")
}

/// Sets the time the file at `path` last changed.
pub(crate) fn set_file_time(path: &Path, time: SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(time))
        .expect("set the file time");
}

/// A made-up Anthropic Console organization with the id `id`, paid per
/// token, found in `/home/user/.claude`.
pub(crate) fn organization(id: &str) -> Account {
    Account::new(
        ProviderRef::of_id("anthropic"),
        AccountIdentity::OrganizationId(id.to_string()),
        AccountLogin::Email(String::from("dev@example.com")),
        AccountKind::Money,
        FoundIn::ProfileFolder(PathBuf::from("/home/user/.claude")),
    )
}

/// A made-up ChatGPT login with the account id `id`, found in
/// `/home/user/.codex`.
pub(crate) fn chatgpt_login(id: &str) -> Account {
    Account::new(
        ProviderRef::of_id("openai"),
        AccountIdentity::AccountId(id.to_string()),
        AccountLogin::Email(String::from("you@example.com")),
        AccountKind::Limits,
        FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
    )
}

/// The account of the made-up `key` at `provider`, found in
/// `/home/user/.claude`.
pub(crate) fn key_account(provider: ProviderRef, key: &str) -> Account {
    Account::new(
        provider,
        AccountIdentity::KeyFingerprint(KeyFingerprint::of(key)),
        AccountLogin::Key(None),
        AccountKind::Money,
        FoundIn::ProfileFolder(PathBuf::from("/home/user/.claude")),
    )
}

/// `account` read by the made-up plugin `example` of the tests, the one
/// [`made_up_part`] and `PluginProvider::made_up` belong to (issue 29).
pub(crate) fn read_by_example(account: Account) -> Account {
    account.with_read_by(Some(String::from("example")))
}

/// What a plugin part tells of whom a login is for when its files say
/// nothing.
pub(crate) fn no_identity() -> otty_plugin_host::agents::agent::Identity {
    otty_plugin_host::agents::agent::Identity {
        account_id: None,
        organization_id: None,
        email: None,
    }
}

/// Whether the folder `folder` holds a file, at any depth.
fn has_a_file(folder: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return false;
    };

    entries.flatten().any(|entry| {
        let path = entry.path();
        path.is_file() || has_a_file(&path)
    })
}
