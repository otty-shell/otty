//! Test-only values and mocks shared by the tests of this crate.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use mockall::mock;
use ruzstd::encoding::{CompressionLevel, compress_to_vec};

use crate::agent::{Agent, AgentDescriptor, AgentId};
use crate::claude_code::ClaudeCode;
use crate::codex::Codex;
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::program::{MockStartProgram, StartProgram};
use crate::session::{AgentSession, SessionId, SessionSource};
use crate::spend::SpendCounter;

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
        fn descriptor(&self) -> &AgentDescriptor;
        fn default_config_dir(&self, env: &AgentEnv) -> PathBuf;
        fn looks_like_config_dir(&self, dir: &Path) -> bool;
        fn detect(&self, profile: &Profile) -> ProfileStatus;
        fn launch<'a>(
            &self,
            profile: &Profile,
            session: Option<&'a SessionId>,
        ) -> AgentCommand;
        fn login(&self, profile: &Profile) -> AgentCommand;
        fn session_sources(&self) -> Vec<Box<dyn SessionSource>>;
        fn limit_sources(
            &self,
            programs: Arc<dyn StartProgram>,
        ) -> Vec<Box<dyn LimitSource>>;
        fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;
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
    let agent = ClaudeCode.descriptor().id();

    Profile::added(agent, "Claude Code · Test", config_dir.to_path_buf())
}

/// A profile of Codex that the user added, in `config_dir`.
pub(crate) fn codex_profile(config_dir: &Path) -> Profile {
    let agent = Codex::default().descriptor().id();

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
    let header = base64url(br#"{"alg":"none","typ":"JWT"}"#);
    let payload = base64url(claims.as_bytes());

    format!("{header}.{payload}.made-up-signature")
}

/// `bytes` in base64url without padding, as a JWT writes its parts.
fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut text = String::new();

    for chunk in bytes.chunks(3) {
        let mut group = [0_u8; 3];
        group[..chunk.len()].copy_from_slice(chunk);
        let bits = u32::from(group[0]) << 16
            | u32::from(group[1]) << 8
            | u32::from(group[2]);
        for index in 0..=chunk.len() {
            let value = (bits >> (18 - 6 * index)) & 0x3f;
            text.push(char::from(ALPHABET[value as usize]));
        }
    }

    text
}

/// Sets the time the file at `path` last changed.
pub(crate) fn set_file_time(path: &Path, time: SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(time))
        .expect("set the file time");
}
