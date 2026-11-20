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
use crate::keys::{KeyText, permission_place, place_text, read_key};
use crate::operations::list_files;
use crate::operations::v0_1::otty::host::files::FileInfo;
use crate::operations::v0_1::otty::host::keys::KeyPlace;
use crate::operations::v0_1::otty::host::types::HostError;
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

    /// The home folder and the variables the plugin's parts see.
    pub fn env(&self) -> &Environment {
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

    /// The files whose paths match `glob` that the part `part` may read,
    /// as `files.list` gives them to the part: OTTY walks the spend places
    /// a part names with it (spec "Host operations").
    pub fn list_files(
        &self,
        part: &PartRef,
        glob: &str,
    ) -> Result<Vec<FileInfo>, HostError> {
        let Some(permissions) = self.permissions.get(part) else {
            return Err(HostError::NotAllowed(part.to_string()));
        };

        list_files(permissions, &self.env, glob)
    }

    /// The key at `place`, which the part `part` declares, read now for
    /// OTTY's own use: the fingerprint accounts are joined by, or a reader
    /// built into OTTY that still sends the key itself. It is never given
    /// to a plugin, and dropped once used.
    pub fn read_key(
        &self,
        part: &PartRef,
        place: &KeyPlace,
    ) -> Result<KeyText, HostError> {
        let may_open = self.permissions.get(part).is_some_and(|permissions| {
            permissions.may_open_key(&permission_place(place))
        });
        if !may_open {
            return Err(HostError::NotAllowed(place_text(place)));
        }

        read_key(place, &self.env, self.outside())
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::PartRef;

    use super::*;
    use crate::engine::PluginEngine;
    use crate::operations::v0_1::otty::host::keys::JsonField;
    use crate::testing::{TempDir, load_template, template_env};

    const AGENT: &str = "example-agent";
    const PROVIDER: &str = "example-cloud";

    fn auth_place() -> KeyPlace {
        let file = template_env().home().join(".example-agent/auth.json");

        KeyPlace::JsonField(JsonField {
            file: file.display().to_string(),
            pointer: "/api_key".to_string(),
        })
    }

    #[test]
    fn the_files_of_a_glob_are_listed_with_the_parts_permissions() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let glob = template_env()
            .home()
            .join(".example-agent/sessions/*.jsonl")
            .display()
            .to_string();

        let mut for_agent: Vec<String> = plugin
            .list_files(&PartRef::agent(AGENT), &glob)
            .expect("listed")
            .into_iter()
            .map(|file| file.path)
            .collect();
        let for_provider = plugin
            .list_files(&PartRef::provider(PROVIDER), &glob)
            .expect("listed");

        for_agent.sort();
        let folder = template_env().home().join(".example-agent/sessions");
        assert_eq!(
            for_agent,
            [
                folder.join("s1.jsonl").display().to_string(),
                folder.join("s2.jsonl").display().to_string(),
            ]
        );
        assert!(for_provider.is_empty(), "{for_provider:?}");
    }

    #[test]
    fn a_key_is_read_for_otty_only_at_a_place_the_part_declares() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());

        let for_agent = plugin.read_key(&PartRef::agent(AGENT), &auth_place());
        let for_provider =
            plugin.read_key(&PartRef::provider(PROVIDER), &auth_place());

        let key = for_agent.expect("the agent part declares the place");
        assert_eq!(key.as_str(), "ex-made-up-key-0001");
        assert_eq!(key.fingerprint().len(), 64);
        assert_eq!(format!("{key:?}"), "KeyText(hidden)");
        assert!(matches!(for_provider, Err(HostError::NotAllowed(_))));
    }
}
