//! Calls into one part of a plugin: a store per batch of calls, with an
//! empty WASI context, the memory cap, the permissions of that part only,
//! made for the login folders of each call, and a deadline per call.

use std::sync::Arc;
use std::time::{Duration, Instant};

use otty_plugin_manifest::{
    Environment, LoginFolder, PartKind, PartRef, Permissions,
};
use wasmtime::component::{Instance, ResourceTable};
use wasmtime::{Store, StoreContextMut, UpdateDeadline};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::agents::agent::AgentCalls;
use crate::agents::prices::PriceList;
use crate::agents::provider::ProviderCalls;
use crate::call_programs::CallPrograms;
use crate::call_terminals::CallTerminals;
use crate::engine::TICK;
use crate::memory_cap::MemoryCap;
use crate::operations::ZstdCursor;
use crate::outside::Outside;
use crate::plugin::LoadedPlugin;
use crate::plugin_log::PluginLog;

/// What a call does, which sets its deadline (spec "Engine limits").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    /// `detect`, `version`, `launch`, `profiles`: 2 s.
    Quick,
    /// `read-answers` and `read-sessions`: 120 s.
    Reading,
    /// `read-limits` and the provider calls: 90 s.
    Numbers,
}

impl CallKind {
    /// How long a call of this kind may run, host operations included.
    pub fn deadline(self) -> Duration {
        match self {
            CallKind::Quick => Duration::from_secs(2),
            CallKind::Reading => Duration::from_secs(120),
            CallKind::Numbers => Duration::from_secs(90),
        }
    }
}

/// What the store of one batch of calls holds: the part the calls are
/// for, only that part's permissions, the environment, the plugin's log,
/// and the running call's deadline. Host operations get it and check the
/// permissions here, never another part's.
pub struct CallState {
    wasi: WasiCtx,
    table: ResourceTable,
    memory: MemoryCap,
    part: PartRef,
    permissions: Arc<Permissions>,
    env: Environment,
    log: PluginLog,
    outside: Arc<Outside>,
    deadline: Instant,
    programs: CallPrograms,
    terminals: CallTerminals,
    zstd_cursor: ZstdCursor,
    prices: Option<Arc<dyn PriceList>>,
}

impl CallState {
    /// The part being called.
    pub fn part(&self) -> &PartRef {
        &self.part
    }

    /// The permissions of the part being called.
    pub fn permissions(&self) -> &Permissions {
        &self.permissions
    }

    /// The home folder and the variables the app read.
    pub fn env(&self) -> &Environment {
        &self.env
    }

    /// The log of the plugin.
    pub fn log(&self) -> &PluginLog {
        &self.log
    }

    /// The network and the system keyring.
    pub(crate) fn outside(&self) -> &Outside {
        &self.outside
    }

    /// The resources the host gave the plugin in this batch, such as key
    /// handles.
    pub(crate) fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    /// The programs the running call runs and talks to.
    pub(crate) fn programs_mut(&mut self) -> &mut CallPrograms {
        &mut self.programs
    }

    /// The hidden terminals the running call started.
    pub(crate) fn terminals_mut(&mut self) -> &mut CallTerminals {
        &mut self.terminals
    }

    /// Whether the running call has run a program or started a talk or a
    /// hidden terminal, which makes a limit source slow.
    pub(crate) fn has_started_programs(&self) -> bool {
        self.programs.has_started() || self.terminals.has_started()
    }

    /// The zstd file `files.read-lines` read last, kept open for its next
    /// batch.
    pub(crate) fn zstd_cursor(&mut self) -> &mut ZstdCursor {
        &mut self.zstd_cursor
    }

    /// OTTY's price list the running call was given: only a read of
    /// answers gets one.
    pub(crate) fn prices(&self) -> Option<&dyn PriceList> {
        self.prices.as_deref()
    }

    /// Gives the running call OTTY's price list, or takes it away.
    pub(crate) fn set_prices(&mut self, prices: Option<Arc<dyn PriceList>>) {
        self.prices = prices;
    }

    /// Sets the permissions of the next call, for its file operations, its
    /// programs and its hidden terminals.
    fn set_call_permissions(&mut self, permissions: Arc<Permissions>) {
        self.programs.set_permissions(Arc::clone(&permissions));
        self.terminals.set_permissions(Arc::clone(&permissions));
        self.permissions = permissions;
    }

    /// Moves the running call's deadline later by `wait`: a host operation
    /// that waited (HTTP, a program, a terminal) has its own timeout, and
    /// that wait does not count against the call.
    pub fn extend_deadline(&mut self, wait: Duration) {
        self.deadline += wait;
    }
}

impl WasiView for CallState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// A batch of calls into one part of a loaded plugin, such as one refresh
/// of one profile. The calls share one instance, so the plugin keeps no
/// state between batches; after a failed call the next call gets a fresh
/// instance.
pub struct CallBatch<'plugin> {
    plugin: &'plugin LoadedPlugin,
    part: PartRef,
    permissions: Arc<Permissions>,
    starts_programs: bool,
    running: Option<(Store<CallState>, Instance)>,
}

impl<'plugin> CallBatch<'plugin> {
    /// The part the calls are for.
    pub fn part(&self) -> &PartRef {
        &self.part
    }

    /// The plugin the calls go to.
    pub(crate) fn plugin(&self) -> &'plugin LoadedPlugin {
        self.plugin
    }

    /// The calls of the agent contract, for an agent part.
    pub fn agent(&mut self) -> Result<AgentCalls<'_, 'plugin>, CallError> {
        let exports = self.plugin.agent_exports().cloned();
        let (PartKind::Agent, Some(exports)) = (self.part.kind(), exports)
        else {
            return Err(CallError::OtherKind {
                part: self.part.clone(),
                asked: PartKind::Agent,
            });
        };

        Ok(AgentCalls::new(self, exports))
    }

    /// The calls of the provider contract, for a provider part.
    pub fn provider(
        &mut self,
    ) -> Result<ProviderCalls<'_, 'plugin>, CallError> {
        let exports = self.plugin.provider_exports().cloned();
        let (PartKind::Provider, Some(exports)) = (self.part.kind(), exports)
        else {
            return Err(CallError::OtherKind {
                part: self.part.clone(),
                asked: PartKind::Provider,
            });
        };

        Ok(ProviderCalls::new(self, exports))
    }

    pub(crate) fn new(
        plugin: &'plugin LoadedPlugin,
        part: PartRef,
        permissions: Arc<Permissions>,
    ) -> Self {
        Self {
            plugin,
            part,
            permissions,
            starts_programs: true,
            running: None,
        }
    }

    /// The same batch, whose calls start no program and no hidden
    /// terminal, as while Agents mode is away: each start is refused, so
    /// a limit source is not paced and the next read starts it.
    pub fn without_programs(mut self) -> Self {
        self.starts_programs = false;

        self
    }

    /// Runs `call` against the instance with the deadline of `kind`. A
    /// trap, a missed deadline or the memory cap fails this call only.
    /// The programs and hidden terminals the call started are stopped
    /// when it ends.
    pub fn call<T>(
        &mut self,
        kind: CallKind,
        call: impl FnOnce(&mut Store<CallState>, &Instance) -> wasmtime::Result<T>,
    ) -> Result<T, CallError> {
        self.call_in_folders(kind, &[], call)
    }

    /// Runs `call` as [`Self::call`] does, with the login folders
    /// `folders` given to the part for this call only: inside them it also
    /// reads what its logins declare (spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 5). The next call
    /// has its own folders, or none.
    pub(crate) fn call_in_folders<T>(
        &mut self,
        kind: CallKind,
        folders: &[LoginFolder],
        call: impl FnOnce(&mut Store<CallState>, &Instance) -> wasmtime::Result<T>,
    ) -> Result<T, CallError> {
        let permissions = match folders.is_empty() {
            true => Arc::clone(&self.permissions),
            false => Arc::new(self.permissions.in_login_folders(folders)),
        };
        let (store, instance) = self.running()?;

        store.data_mut().set_call_permissions(permissions);
        store.data_mut().deadline = Instant::now() + kind.deadline();
        store.set_epoch_deadline(ticks_for(kind.deadline()));
        store.data_mut().programs.start_call();
        store.data_mut().terminals.start_call();

        let result = call(store, instance);
        store.data_mut().programs.end_call();
        store.data_mut().terminals.end_call();

        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let failure = failure_of(&error, store.data(), kind);
                self.running = None;

                Err(failure)
            },
        }
    }

    /// The store and instance of this batch, made on the first call.
    fn running(
        &mut self,
    ) -> Result<(&mut Store<CallState>, &Instance), CallError> {
        if self.running.is_none() {
            self.running = Some(self.start()?);
        }

        match &mut self.running {
            Some((store, instance)) => Ok((store, instance)),
            None => Err(CallError::Start("no instance".to_string())),
        }
    }

    fn start(&self) -> Result<(Store<CallState>, Instance), CallError> {
        let mut wasi = WasiCtx::builder();
        wasi.stderr(self.plugin.stderr().clone());
        let starters = self.plugin.starters();
        let (programs, terminals) = if self.starts_programs {
            (starters.programs(), starters.terminals())
        } else {
            (None, None)
        };
        let state = CallState {
            wasi: wasi.build(),
            table: ResourceTable::new(),
            memory: MemoryCap::new(),
            part: self.part.clone(),
            permissions: Arc::clone(&self.permissions),
            env: self.plugin.env().clone(),
            log: self.plugin.log().clone(),
            outside: Arc::clone(self.plugin.outside()),
            deadline: Instant::now() + CallKind::Quick.deadline(),
            programs: CallPrograms::new(
                programs,
                Arc::clone(&self.permissions),
                Arc::clone(self.plugin.program_paths()),
            ),
            terminals: CallTerminals::new(
                terminals,
                Arc::clone(&self.permissions),
                Arc::clone(self.plugin.program_paths()),
            ),
            zstd_cursor: ZstdCursor::default(),
            prices: None,
        };

        let mut store = Store::new(self.plugin.engine(), state);
        store.limiter(|state| &mut state.memory);
        store.epoch_deadline_callback(on_deadline);
        store.set_epoch_deadline(ticks_for(CallKind::Quick.deadline()));

        let instance = self
            .plugin
            .instance_pre()
            .instantiate(&mut store)
            .map_err(|error| CallError::Start(error_text(&error)))?;

        Ok((store, instance))
    }
}

/// Why a call into a plugin failed, in plain words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CallError {
    /// The plugin has no such part.
    #[error("The plugin has no {0}")]
    UnknownPart(PartRef),
    /// The calls of one kind of part were asked of a part of another kind.
    #[error("The {part} has no calls of a {asked} part")]
    OtherKind {
        /// The part of the batch.
        part: PartRef,
        /// The kind whose calls were asked.
        asked: PartKind,
    },
    /// The plugin could not be started.
    #[error("The plugin could not start: {0}")]
    Start(String),
    /// The call ran past its deadline and was stopped.
    #[error("The plugin did not finish within {} s and was stopped", .0.as_secs())]
    MissedDeadline(Duration),
    /// The plugin asked for more memory than the cap.
    #[error("The plugin used more than 64 MiB of memory and was stopped")]
    MemoryCap,
    /// The plugin trapped or a host operation failed it.
    #[error("The plugin stopped with an error: {0}")]
    Trap(String),
}

/// The error the epoch callback stops a call with.
#[derive(Debug, thiserror::Error)]
#[error("the call ran past its deadline")]
struct PastDeadline;

/// The epoch callback: keep running while the call's deadline, maybe
/// extended by host waits, is ahead; stop it once it passed.
fn on_deadline(
    context: StoreContextMut<'_, CallState>,
) -> wasmtime::Result<UpdateDeadline> {
    match ticks_left(context.data().deadline, Instant::now()) {
        Some(ticks) => Ok(UpdateDeadline::Continue(ticks)),
        None => Err(wasmtime::Error::new(PastDeadline)),
    }
}

/// How many epoch ticks are left until `deadline` at `now`; `None` once it
/// passed.
fn ticks_left(deadline: Instant, now: Instant) -> Option<u64> {
    let left = deadline.checked_duration_since(now)?;
    if left.is_zero() {
        return None;
    }

    Some(ticks_for(left))
}

/// `duration` in epoch ticks, rounded up, at least one.
fn ticks_for(duration: Duration) -> u64 {
    let ticks = duration.as_nanos().div_ceil(TICK.as_nanos());

    u64::try_from(ticks).unwrap_or(u64::MAX).max(1)
}

fn failure_of(
    error: &wasmtime::Error,
    state: &CallState,
    kind: CallKind,
) -> CallError {
    if error.is::<PastDeadline>()
        || error.downcast_ref::<wasmtime::Trap>()
            == Some(&wasmtime::Trap::Interrupt)
    {
        return CallError::MissedDeadline(kind.deadline());
    }
    if state.memory.hit() {
        return CallError::MemoryCap;
    }

    CallError::Trap(error_text(error))
}

fn error_text(error: &wasmtime::Error) -> String {
    let causes: Vec<String> = error.chain().map(ToString::to_string).collect();

    causes.join(": ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::agent::Profile;
    use crate::engine::PluginEngine;
    use crate::testing::{TempDir, load_fixture};

    /// A profile of the test plugin's agent part.
    fn profile() -> Profile {
        Profile {
            agent: "test-agent".to_string(),
            folder: "/home/test-user/.test-agent".to_string(),
            is_default: true,
        }
    }

    /// Calls `func` of the exported interface `interface`, which no
    /// contract binds, such as the test plugins' `misbehave`.
    fn call_export<R>(
        batch: &mut CallBatch<'_>,
        kind: CallKind,
        interface: &str,
        func: &str,
        args: impl wasmtime::component::ComponentNamedList
        + wasmtime::component::Lower
        + Send
        + Sync,
    ) -> Result<R, CallError>
    where
        R: wasmtime::component::ComponentType
            + wasmtime::component::Lift
            + Send
            + Sync,
    {
        batch.call(kind, |store, instance| {
            let interface = instance
                .get_export_index(&mut *store, None, interface)
                .ok_or_else(|| {
                    wasmtime::format_err!("no export {interface}")
                })?;
            let func = instance
                .get_export_index(&mut *store, Some(&interface), func)
                .ok_or_else(|| wasmtime::format_err!("no function {func}"))?;
            let typed =
                instance.get_typed_func::<_, (R,)>(&mut *store, &func)?;

            let (result,) = typed.call(&mut *store, args)?;

            Ok(result)
        })
    }

    #[test]
    fn a_call_reaches_the_plugin_and_its_stderr_reaches_the_log() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "minimal");
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");

        let version = batch
            .agent()
            .expect("an agent part")
            .version(&profile())
            .expect("the version");

        assert_eq!(version.as_deref(), Some("1.0.0"));
        let logged = (0..100).any(|_| {
            let found = plugin
                .log()
                .text()
                .contains("the test plugin was asked its version");
            if !found {
                std::thread::sleep(Duration::from_millis(10));
            }
            found
        });
        assert!(logged, "the plugin's stderr is in its log");
    }

    #[test]
    fn a_spinning_plugin_stops_near_its_deadline() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "misbehave");
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");
        let started = Instant::now();

        let result: Result<u64, CallError> = call_export(
            &mut batch,
            CallKind::Quick,
            "otty:test/misbehave@0.1.0",
            "spin",
            (),
        );

        let took = started.elapsed();
        assert_eq!(
            result,
            Err(CallError::MissedDeadline(Duration::from_secs(2)))
        );
        assert!(took >= Duration::from_secs(2), "{took:?}");
        assert!(took < Duration::from_secs(3), "{took:?}");
    }

    #[test]
    fn a_host_wait_extends_the_deadline_by_its_own_length() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "misbehave");
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");
        let started = Instant::now();

        let result = batch.call(CallKind::Quick, |store, instance| {
            // What a host operation does after waiting 1.5 s for an answer.
            std::thread::sleep(Duration::from_millis(1500));
            store
                .data_mut()
                .extend_deadline(Duration::from_millis(1500));

            let interface = instance
                .get_export_index(
                    &mut *store,
                    None,
                    "otty:test/misbehave@0.1.0",
                )
                .ok_or_else(|| wasmtime::format_err!("no misbehave"))?;
            let spin = instance
                .get_export_index(&mut *store, Some(&interface), "spin")
                .ok_or_else(|| wasmtime::format_err!("no spin"))?;
            let typed =
                instance.get_typed_func::<(), (u64,)>(&mut *store, &spin)?;

            typed.call(&mut *store, ())
        });

        let took = started.elapsed();
        assert!(matches!(result, Err(CallError::MissedDeadline(_))));
        assert!(took >= Duration::from_millis(3500), "{took:?}");
        assert!(took < Duration::from_millis(4500), "{took:?}");
    }

    #[test]
    fn the_next_call_after_a_failure_gets_a_fresh_instance() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "misbehave");
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");

        let grabbed: Result<u64, CallError> = call_export(
            &mut batch,
            CallKind::Quick,
            "otty:test/misbehave@0.1.0",
            "hold",
            (100_u32,),
        );
        let version = batch.agent().expect("an agent part").version(&profile());

        assert_eq!(grabbed, Err(CallError::MemoryCap));
        assert_eq!(version, Ok(Some("1.0.0".to_string())));
    }

    #[test]
    fn a_memory_grabber_traps_at_the_cap() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "misbehave");
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");

        let held: Result<u64, CallError> = call_export(
            &mut batch,
            CallKind::Quick,
            "otty:test/misbehave@0.1.0",
            "hold",
            (32_u32,),
        );
        let grabbed: Result<u64, CallError> = call_export(
            &mut batch,
            CallKind::Quick,
            "otty:test/misbehave@0.1.0",
            "hold",
            (64_u32,),
        );

        assert_eq!(held, Ok(32 * 1024 * 1024));
        assert_eq!(grabbed, Err(CallError::MemoryCap));
    }

    #[test]
    fn a_call_names_its_part_and_carries_its_permissions_only() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "minimal");
        let mut batch = plugin
            .start_calls(&PartRef::agent("test-agent"))
            .expect("a listed part");

        let (part, reads_own, reads_other) = batch
            .call(CallKind::Quick, |store, _| {
                let state = store.data();
                Ok((
                    state.part().clone(),
                    state.permissions().may_read(std::path::Path::new(
                        "/home/test-user/.test-agent/a",
                    )),
                    state.permissions().may_read(std::path::Path::new(
                        "/home/test-user/.codex/a",
                    )),
                ))
            })
            .expect("the state");

        assert_eq!(part, PartRef::agent("test-agent"));
        assert!(reads_own);
        assert!(!reads_other);
    }

    #[test]
    fn a_part_the_plugin_does_not_list_can_not_be_called() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_fixture(&engine, "minimal");

        let error = plugin
            .start_calls(&PartRef::provider("test-agent"))
            .err()
            .expect("not listed");

        assert_eq!(
            error,
            CallError::UnknownPart(PartRef::provider("test-agent"))
        );
    }

    #[test]
    fn a_wait_of_a_host_operation_moves_the_deadline() {
        let now = Instant::now();
        let deadline = now + Duration::from_millis(5);

        assert_eq!(ticks_left(deadline, now), Some(1));
        assert_eq!(
            ticks_left(deadline + Duration::from_secs(1), now),
            Some(101)
        );
        assert_eq!(ticks_left(now, now), None);
        assert_eq!(ticks_left(now, now + Duration::from_millis(1)), None);
    }

    #[test]
    fn deadlines_follow_the_kind_of_call() {
        assert_eq!(CallKind::Quick.deadline(), Duration::from_secs(2));
        assert_eq!(CallKind::Reading.deadline(), Duration::from_secs(120));
        assert_eq!(CallKind::Numbers.deadline(), Duration::from_secs(90));
        assert_eq!(ticks_for(CallKind::Quick.deadline()), 200);
        assert_eq!(ticks_for(Duration::ZERO), 1);
    }
}
