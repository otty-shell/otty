//! A loaded plugin: its checked manifest, its compiled component ready to
//! instantiate, the permissions of each part and its log.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use otty_plugin_manifest::{Environment, Manifest, PartRef, Permissions};
use wasmtime::Engine;
use wasmtime::component::InstancePre;
use wasmtime_wasi::cli::OutputFile;

use crate::agent::AgentExports;
use crate::cache::LoadKind;
use crate::calls::{CallBatch, CallError, CallState};
use crate::contract::ContractExports;
use crate::engine::{EngineShared, Starters};
use crate::outside::Outside;
use crate::pacing::LimitPacing;
use crate::plugin_log::PluginLog;
use crate::provider::ProviderExports;

/// A plugin loaded by a [`PluginEngine`](crate::PluginEngine), ready to be
/// called part by part.
pub struct LoadedPlugin {
    shared: Arc<EngineShared>,
    manifest: Manifest,
    instance_pre: InstancePre<CallState>,
    exports: ContractExports,
    load_kind: LoadKind,
    env: Environment,
    permissions: HashMap<PartRef, Arc<Permissions>>,
    log: PluginLog,
    stderr: OutputFile,
    starters: Starters,
    pacing: LimitPacing,
}

impl LoadedPlugin {
    /// The plugin's manifest.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Whether the component was compiled now or read from the cache.
    pub fn load_kind(&self) -> LoadKind {
        self.load_kind
    }

    /// The plugin's log: its stderr, at most 64 KiB.
    pub fn log(&self) -> &PluginLog {
        &self.log
    }

    pub(crate) fn engine(&self) -> &Engine {
        self.shared.engine()
    }

    pub(crate) fn outside(&self) -> &Arc<Outside> {
        self.shared.outside()
    }

    pub(crate) fn instance_pre(&self) -> &InstancePre<CallState> {
        &self.instance_pre
    }

    pub(crate) fn stderr(&self) -> &OutputFile {
        &self.stderr
    }

    pub(crate) fn env(&self) -> &Environment {
        &self.env
    }

    pub(crate) fn agent_exports(&self) -> Option<&AgentExports> {
        self.exports.agent()
    }

    pub(crate) fn provider_exports(&self) -> Option<&ProviderExports> {
        self.exports.provider()
    }

    /// How the plugin's programs and hidden terminals are started.
    pub(crate) fn starters(&self) -> &Starters {
        &self.starters
    }

    /// The last slow limit readings of the plugin's parts.
    pub(crate) fn pacing(&self) -> &LimitPacing {
        &self.pacing
    }

    pub(crate) fn new(
        shared: Arc<EngineShared>,
        manifest: Manifest,
        instance_pre: InstancePre<CallState>,
        exports: ContractExports,
        load_kind: LoadKind,
        env: &Environment,
        starters: Starters,
    ) -> io::Result<Self> {
        let permissions = manifest
            .parts()
            .into_iter()
            .filter_map(|part| {
                let declared = manifest.permissions_of(&part)?;
                let permissions = Arc::new(Permissions::new(declared, env));

                Some((part, permissions))
            })
            .collect();

        let log = PluginLog::new();
        let stderr = log.stderr()?;

        Ok(Self {
            shared,
            manifest,
            instance_pre,
            exports,
            load_kind,
            env: env.clone(),
            permissions,
            log,
            stderr,
            starters,
            pacing: LimitPacing::default(),
        })
    }

    /// Starts a batch of calls into the part `part`: they run with that
    /// part's permissions only.
    pub fn start_calls(
        &self,
        part: &PartRef,
    ) -> Result<CallBatch<'_>, CallError> {
        let Some(permissions) = self.permissions.get(part) else {
            return Err(CallError::UnknownPart(part.clone()));
        };

        Ok(CallBatch::new(self, part.clone(), Arc::clone(permissions)))
    }
}
