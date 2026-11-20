//! The agent contract: the bindings of each version the host keeps, the
//! types of the newest version, and the calls into an agent part.
//!
//! A new version goes into its own module next to the old ones, such as
//! `v0_2`; the types below then come from it, and the old version's arm
//! of [`AgentCalls`] converts its types to them with `From`.

pub mod v0_1;

use std::time::Instant;

use wasmtime::Store;
use wasmtime::component::InstancePre;

/// The newest agent contract's types, which the rest of OTTY uses.
pub use crate::agent::v0_1::exports::otty::agent::agent::{
    Answer, Answers, Billing, Command, Connection, Login, LoginRef,
    ModelTokens, Profile, ProfilePlace, ProfileStatus, ProjectStats,
    ProviderRef, RecordCounts, Session, SessionStats, SpendItem, SpendKind,
    SpendPlace, TokenCount,
};
use crate::agent::v0_1::exports::otty::agent::agent::{Guest, GuestIndices};
use crate::calls::{CallBatch, CallError, CallKind, CallState};
use crate::contract::AgentVersion;
/// The host's types the agent contract uses.
pub use crate::operations::v0_1::otty::host::keys::{
    JsonField, KeyPlace, KeyringEntry, SqliteField,
};
/// The host's types the agent contract uses.
pub use crate::operations::v0_1::otty::host::types::{
    Credits, HostError, Identity, LimitWindow, Limits, LoginKind, ReadError,
};
use crate::pacing::PaceKey;

/// The agent contract of one loaded plugin, in the version its `[api]`
/// names.
#[derive(Clone)]
pub(crate) enum AgentExports {
    V0_1(GuestIndices),
}

impl AgentExports {
    /// The exports of `version` of the agent contract in the component of
    /// `instance_pre`.
    pub(crate) fn new(
        version: AgentVersion,
        instance_pre: &InstancePre<CallState>,
    ) -> wasmtime::Result<Self> {
        match version {
            AgentVersion::V0_1 => {
                Ok(Self::V0_1(GuestIndices::new(instance_pre)?))
            },
        }
    }
}

/// The calls of the agent contract into one agent part, within one
/// [`CallBatch`]. Each call has the deadline of its kind; an `Err` is a
/// trap, a missed deadline or the memory cap, while what the part itself
/// answers, such as a [`ReadError`], is in the `Ok` value.
pub struct AgentCalls<'batch, 'plugin> {
    batch: &'batch mut CallBatch<'plugin>,
    exports: AgentExports,
}

impl<'batch, 'plugin> AgentCalls<'batch, 'plugin> {
    pub(crate) fn new(
        batch: &'batch mut CallBatch<'plugin>,
        exports: AgentExports,
    ) -> Self {
        Self { batch, exports }
    }

    /// The agent's config folders on this computer, default first.
    pub fn profiles(&mut self) -> Result<Vec<ProfilePlace>, CallError> {
        let agent = self.batch.part().id().to_string();

        self.call(CallKind::Quick, |guest, store| {
            guest.call_profiles(store, &agent)
        })
    }

    /// Whether `profile` is logged in.
    pub fn detect(
        &mut self,
        profile: &Profile,
    ) -> Result<ProfileStatus, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_detect(store, profile)
        })
    }

    /// The agent's version written in the profile's newest files.
    pub fn version(
        &mut self,
        profile: &Profile,
    ) -> Result<Option<String>, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_version(store, profile)
        })
    }

    /// The logins and keys the profile holds.
    pub fn logins(
        &mut self,
        profile: &Profile,
    ) -> Result<Vec<Login>, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_logins(store, profile)
        })
    }

    /// The sessions of the profile, newest first.
    pub fn sessions(
        &mut self,
        profile: &Profile,
    ) -> Result<Result<Vec<Session>, ReadError>, CallError> {
        self.call(CallKind::Reading, |guest, store| {
            guest.call_sessions(store, profile)
        })
    }

    /// Where the profile's answers are.
    pub fn spend_places(
        &mut self,
        profile: &Profile,
    ) -> Result<Vec<SpendPlace>, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_spend_places(store, profile)
        })
    }

    /// Every answer in one file or database of a spend place.
    pub fn read_answers(
        &mut self,
        profile: &Profile,
        item: &SpendItem,
    ) -> Result<Result<Answers, ReadError>, CallError> {
        self.call(CallKind::Reading, |guest, store| {
            guest.call_read_answers(store, profile, item)
        })
    }

    /// The command that starts a new session, or continues `session`.
    pub fn launch(
        &mut self,
        profile: &Profile,
        session: Option<&str>,
    ) -> Result<Command, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_launch(store, profile, session)
        })
    }

    /// The command that logs the profile in.
    pub fn login_command(
        &mut self,
        profile: &Profile,
    ) -> Result<Command, CallError> {
        self.call(CallKind::Quick, |guest, store| {
            guest.call_login_command(store, profile)
        })
    }

    /// The limits the agent itself shows for `login`, from wherever the
    /// part reads them. A reading that ran a program or a hidden terminal
    /// is slow: it runs at most once in the five minutes of the limits
    /// read per profile, and the answer of the last reading is given
    /// within that time.
    pub fn read_limits(
        &mut self,
        profile: &Profile,
        login: &LoginRef,
    ) -> Result<Result<Limits, ReadError>, CallError> {
        let plugin = self.batch.plugin();
        let key = PaceKey::new(self.batch.part(), &profile.folder);
        let started_at = Instant::now();
        if let Some(answer) = plugin.pacing().recent(&key, started_at) {
            return Ok(answer);
        }

        let (answer, is_slow) =
            self.call(CallKind::Numbers, |guest, store| {
                let answer =
                    guest.call_read_limits(&mut *store, profile, login)?;

                Ok((answer, store.data().has_started_programs()))
            })?;

        if is_slow {
            plugin.pacing().remember(key, started_at, answer.clone());
        }

        Ok(answer)
    }

    /// What the files of one session tell.
    pub fn read_session_stats(
        &mut self,
        profile: &Profile,
        session: &str,
    ) -> Result<Result<SessionStats, ReadError>, CallError> {
        self.call(CallKind::Reading, |guest, store| {
            guest.call_read_session_stats(store, profile, session)
        })
    }

    /// What the files of one project folder tell.
    pub fn read_project_stats(
        &mut self,
        profile: &Profile,
        folder: &str,
    ) -> Result<Result<ProjectStats, ReadError>, CallError> {
        self.call(CallKind::Reading, |guest, store| {
            guest.call_read_project_stats(store, profile, folder)
        })
    }

    /// Runs `call` with the exports of the plugin's version of the
    /// contract.
    fn call<T>(
        &mut self,
        kind: CallKind,
        call: impl FnOnce(&Guest, &mut Store<CallState>) -> wasmtime::Result<T>,
    ) -> Result<T, CallError> {
        let exports = self.exports.clone();

        self.batch.call(kind, |store, instance| match &exports {
            AgentExports::V0_1(indices) => {
                let guest = indices.load(&mut *store, instance)?;

                call(&guest, store)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use otty_plugin_manifest::PartRef;

    use super::*;
    use crate::engine::PluginEngine;
    use crate::programs::{MockStartProgram, RunOutput};
    use crate::testing::{
        TempDir, load_fixture, load_programs_plugin, load_template,
        template_env,
    };

    const AGENT: &str = "example-agent";

    fn default_profile() -> Profile {
        let folder = template_env().home().join(".example-agent");

        Profile {
            agent: AGENT.to_string(),
            folder: folder.display().to_string(),
            is_default: true,
        }
    }

    #[test]
    fn the_template_gives_its_version_from_its_fixtures() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");

        let version = batch
            .agent()
            .expect("an agent part")
            .version(&default_profile());

        assert_eq!(version, Ok(Some("1.4.2".to_string())));
    }

    #[test]
    fn the_template_finds_its_profiles_in_the_home_folder() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");

        let places = batch
            .agent()
            .expect("an agent part")
            .profiles()
            .expect("the profiles");

        let listed: Vec<(&str, bool)> = places
            .iter()
            .map(|place| (place.folder.as_str(), place.is_default))
            .collect();
        let work = template_env().home().join(".example-agent-work");
        assert_eq!(
            listed,
            [
                (default_profile().folder.as_str(), true),
                (work.display().to_string().as_str(), false)
            ]
        );
    }

    #[test]
    fn a_variable_the_part_lists_reaches_it() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let env = template_env().with_var("EXAMPLE_AGENT_HOME", "/data/agent");
        let plugin = load_template(&engine, &env);
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");

        let places = batch
            .agent()
            .expect("an agent part")
            .profiles()
            .expect("the profiles");

        assert_eq!(places[0].folder, "/data/agent");
        assert!(places[0].is_default);
    }

    #[test]
    fn what_the_part_logs_reaches_the_plugin_log() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");
        let work = Profile {
            folder: template_env()
                .home()
                .join(".example-agent-work")
                .display()
                .to_string(),
            is_default: false,
            ..default_profile()
        };

        let version = batch.agent().expect("an agent part").version(&work);

        assert_eq!(version, Ok(None));
        assert!(
            plugin
                .log()
                .text()
                .contains("[warn] state.json is not in the shape"),
            "{}",
            plugin.log().text()
        );
    }

    #[test]
    fn the_template_reads_its_sessions_through_the_host() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::agent(AGENT))
            .expect("a listed part");

        let sessions = batch
            .agent()
            .expect("an agent part")
            .sessions(&default_profile())
            .expect("the call ran")
            .expect("the sessions");

        let found: Vec<(&str, &str)> = sessions
            .iter()
            .map(|session| (session.id.as_str(), session.title.as_str()))
            .collect();
        assert_eq!(
            found,
            [
                ("s2", "Write the release notes"),
                ("s1", "Fix the cart total")
            ]
        );
    }

    #[test]
    fn a_provider_part_has_no_agent_calls() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider("example-cloud"))
            .expect("a listed part");

        let error = batch.agent().err().expect("not an agent part");

        assert_eq!(
            error,
            CallError::OtherKind {
                part: PartRef::provider("example-cloud"),
                asked: otty_plugin_manifest::PartKind::Agent,
            }
        );
    }

    #[test]
    fn pacing_skips_a_second_probe_of_a_profile_within_five_minutes() {
        let cache = TempDir::new();
        let mut starter = MockStartProgram::new();
        starter.expect_run().times(2).returning(|command, _, _| {
            let folder = command.env()[0].1.clone();
            Ok(RunOutput::new(Some(0), folder.into_bytes(), Vec::new()))
        });
        let engine = PluginEngine::new(cache.path().to_path_buf())
            .expect("engine")
            .with_programs(Arc::new(starter));
        let plugin = load_programs_plugin(&engine);
        let work = Profile {
            agent: "test-agent".to_string(),
            folder: "/home/test-user/.test-agent-work".to_string(),
            is_default: false,
        };
        let home = Profile {
            folder: "/home/test-user/.test-agent".to_string(),
            is_default: true,
            ..work.clone()
        };
        let quick = Profile {
            folder: "/home/test-user/.test-agent-quick".to_string(),
            ..work.clone()
        };
        // The test plugin reads the limits the way its login names.
        let read = |profile: &Profile, way: &str| {
            let mut batch = plugin
                .start_calls(&PartRef::agent("test-agent"))
                .expect("a listed part");
            let limits = batch
                .agent()
                .expect("an agent part")
                .read_limits(profile, &LoginRef::EnvVar(way.to_string()))
                .expect("the call");

            limits.ok().and_then(|limits| limits.plan)
        };

        let first = read(&work, "run");
        let second = read(&work, "run");
        let other_profile = read(&home, "run");
        let no_program = [read(&quick, "none"), read(&quick, "none")];

        assert_eq!(first.as_deref(), Some("/home/test-user/.test-agent-work"));
        assert_eq!(second, first);
        assert_eq!(
            other_profile.as_deref(),
            Some("/home/test-user/.test-agent")
        );
        assert_eq!(
            no_program,
            [
                Some("no program".to_string()),
                Some("no program".to_string())
            ]
        );
    }

    #[test]
    fn a_plugin_with_only_the_agent_contract_has_no_provider_calls() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "minimal");
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");

        assert!(batch.provider().is_err());
        assert!(batch.agent().is_ok());
    }
}
