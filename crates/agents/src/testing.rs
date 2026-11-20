//! Test-only values and mocks shared by the tests of this crate.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use jiff::tz::TimeZone;
use mockall::mock;
use ruzstd::encoding::{CompressionLevel, compress_to_vec};

use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
};
use crate::account_key::AccountKey;
use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder};
use crate::bill::BillSource;
use crate::claude_code::ClaudeCode;
use crate::codex::Codex;
use crate::command::AgentCommand;
use crate::dollars::Dollars;
use crate::env::AgentEnv;
use crate::hidden_terminal::{MockStartTerminal, StartTerminal};
use crate::https::{HttpsAnswer, MockHttpsGet};
use crate::key_fingerprint::KeyFingerprint;
use crate::limits::LimitSource;
use crate::local_time::LocalNow;
use crate::price_table::TokenPrices;
use crate::profile::{Profile, ProfileStatus};
use crate::program::{MockStartProgram, StartProgram};
use crate::provider::ProviderRef;
use crate::registry::Reader;
use crate::session::{AgentSession, SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::spend::SpendCounter;
use crate::stats::StatsSource;

/// The "now" of the tests: 2026-11-16T10:00:00Z, in seconds since 1970.
pub(crate) const NOW: u64 = 1_794_823_200;

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
        fn account_key(
            &self,
            profile: &Profile,
            account: &Account,
        ) -> Option<AccountKey>;
        fn has_sessions(&self, profile: &Profile) -> bool;
        fn version(&self, profile: &Profile) -> Option<String>;
        fn launch<'a>(
            &self,
            profile: &Profile,
            session: Option<&'a SessionId>,
        ) -> AgentCommand;
        fn login(&self, profile: &Profile) -> AgentCommand;
        fn session_sources(&self) -> Vec<Box<dyn SessionSource>>;
        fn session_source_ids(&self) -> &'static [SourceId];
        fn limit_sources(
            &self,
            programs: Arc<dyn StartProgram>,
            terminals: Arc<dyn StartTerminal>,
        ) -> Vec<Box<dyn LimitSource>>;
        fn limit_source_ids(&self) -> &'static [SourceId];
        fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;
        fn bill_source(&self) -> Option<Box<dyn BillSource>>;
        fn stats_source(&self) -> Option<Box<dyn StatsSource>>;
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

/// A program starter that must never start anything: for the sources of
/// an agent that are only listed, never asked.
pub(crate) fn no_programs() -> Arc<dyn StartProgram> {
    let mut programs = MockStartProgram::new();
    programs.expect_start().never();

    Arc::new(programs)
}

/// A terminal starter that must never start anything: for the sources of
/// an agent that are only listed, never asked.
pub(crate) fn no_terminals() -> Arc<dyn StartTerminal> {
    let mut terminals = MockStartTerminal::new();
    terminals.expect_start().never();

    Arc::new(terminals)
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

/// A request that answers `status` with `body` once.
pub(crate) fn answering(status: u16, body: &str) -> MockHttpsGet {
    let body = body.as_bytes().to_vec();
    let mut client = MockHttpsGet::new();

    client
        .expect_get()
        .times(1)
        .return_once(move |_| Ok(HttpsAnswer { status, body }));

    client
}

/// The entry of a made-up agent with the id `id`, whose program is
/// `program`.
pub(crate) fn mock_entry(id: &str, program: &str) -> AgentEntry {
    AgentEntry::new(
        AgentId::new(id),
        id,
        program,
        "Made-up Labs",
        Reader::BuiltIn,
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

/// A profile of Claude Code that the user added, in `config_dir`.
pub(crate) fn claude_profile(config_dir: &Path) -> Profile {
    let agent = ClaudeCode::default().entry().id().clone();

    Profile::added(agent, "Claude Code · Test", config_dir.to_path_buf())
}

/// A profile of Codex that the user added, in `config_dir`.
pub(crate) fn codex_profile(config_dir: &Path) -> Profile {
    let agent = Codex::default().entry().id().clone();

    Profile::added(agent, "Codex · Test", config_dir.to_path_buf())
}

/// Writes `content` to `path`, creating the folders on the way.
pub(crate) fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create test folders");
    }

    std::fs::write(path, content).expect("write test file");
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

/// Writes `content` to `path` compressed with zstd, the way Codex keeps its
/// older session files, creating the folders on the way.
pub(crate) fn write_compressed(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create test folders");
    }

    let compressed =
        compress_to_vec(content.as_bytes(), CompressionLevel::Fastest);
    std::fs::write(path, compressed).expect("write compressed test file");
}

/// A made-up token in the shape of a JWT whose middle part holds `claims`,
/// such as `{"exp": 1794823200}`. Nothing signs it: only its shape is
/// real.
pub(crate) fn made_up_jwt(claims: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(claims.as_bytes());

    format!("{header}.{payload}.made-up-signature")
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
