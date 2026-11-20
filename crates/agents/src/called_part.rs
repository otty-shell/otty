//! One part of a loaded plugin as OTTY calls it: each read of the part is
//! one batch of calls with that part's permissions only, and a read that
//! traps or misses its deadline counts towards pausing the part. Every
//! failure of a call is kept as the part's last one.

use std::sync::Arc;
use std::time::SystemTime;

use otty_plugin_host::{CallBatch, CallError, LoadedPlugin, PartRef};

use crate::part_health::PartHealth;
use crate::plugin_failure::{PluginFailure, PluginFailureKind};
use crate::registry::PluginPart;

/// What the calls of one read may start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Starts {
    /// Programs and hidden terminals the part declares.
    Programs,
    /// Nothing.
    Nothing,
}

/// One part of a loaded plugin: the plugin, the part, what names it in
/// reports, and how its reads went.
pub(crate) struct CalledPart {
    plugin: Arc<LoadedPlugin>,
    part: PartRef,
    named: PluginPart,
    health: PartHealth,
}

impl CalledPart {
    /// The plugin.
    pub(crate) fn plugin(&self) -> &LoadedPlugin {
        &self.plugin
    }

    /// The part, by its kind and id.
    pub(crate) fn part(&self) -> &PartRef {
        &self.part
    }

    /// The plugin and the part, as the registry and the reports name them.
    pub(crate) fn named(&self) -> &PluginPart {
        &self.named
    }

    /// How the part's reads went.
    pub(crate) fn health(&self) -> &PartHealth {
        &self.health
    }

    /// The part `part` of `plugin`.
    pub(crate) fn new(plugin: Arc<LoadedPlugin>, part: PartRef) -> Self {
        let named = PluginPart::of_manifest(plugin.manifest(), part.id());

        Self {
            plugin,
            part,
            named,
            health: PartHealth::default(),
        }
    }

    /// Runs one read of the part, named `call` in reports: `read` makes
    /// its calls in one batch, so the plugin keeps nothing between reads.
    /// A paused part is not called. A trap, a missed deadline or the
    /// memory cap fails the read; three failed reads in a row pause the
    /// part.
    pub(crate) fn read<T>(
        &self,
        call: &'static str,
        read: impl FnOnce(&mut CallBatch<'_>) -> Result<T, CallError>,
    ) -> Result<T, Box<PluginFailure>> {
        self.read_in(call, Starts::Programs, read)
    }

    /// [`read`](Self::read), whose calls start no program and no hidden
    /// terminal, as while Agents mode is away.
    pub(crate) fn read_starting_nothing<T>(
        &self,
        call: &'static str,
        read: impl FnOnce(&mut CallBatch<'_>) -> Result<T, CallError>,
    ) -> Result<T, Box<PluginFailure>> {
        self.read_in(call, Starts::Nothing, read)
    }

    /// One read of the part, whose calls start what `starts` says.
    fn read_in<T>(
        &self,
        call: &'static str,
        starts: Starts,
        read: impl FnOnce(&mut CallBatch<'_>) -> Result<T, CallError>,
    ) -> Result<T, Box<PluginFailure>> {
        if self.health.is_paused() {
            return Err(self.failure(call, PluginFailureKind::Paused));
        }

        let answer = self
            .plugin
            .start_calls(&self.part)
            .map(|batch| match starts {
                Starts::Programs => batch,
                Starts::Nothing => batch.without_programs(),
            })
            .and_then(|mut batch| read(&mut batch));

        match answer {
            Ok(value) => {
                self.health.answered();
                Ok(value)
            },
            Err(error) => {
                log::warn!(
                    "the plugin part {} of {} failed in {call}: {error}",
                    self.part,
                    self.named.plugin()
                );
                self.health.failed();
                Err(self.failure(call, PluginFailureKind::of_call(&error)))
            },
        }
    }

    /// The failure of `call` of this part, of the kind `kind`, kept as
    /// the part's last failure unless the part was only paused: the
    /// failure that paused it stays.
    pub(crate) fn failure(
        &self,
        call: &'static str,
        kind: PluginFailureKind,
    ) -> Box<PluginFailure> {
        let failure = PluginFailure::new(self.named.clone(), call, kind);

        if kind != PluginFailureKind::Paused {
            self.health.remember_failure(&failure, SystemTime::now());
        }
        Box::new(failure)
    }
}
