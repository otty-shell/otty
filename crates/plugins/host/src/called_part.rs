//! One part of a loaded plugin as OTTY calls it, whatever domain it serves
//! (issue 21): each read of the part is one batch of calls with that
//! part's permissions only, and a read that traps or misses its deadline
//! counts towards pausing the part. Every failure of a call is kept as the
//! part's last one.

use std::sync::Arc;
use std::time::SystemTime;

use otty_plugin_manifest::PartRef;

use crate::calls::{CallBatch, CallError};
use crate::part_health::PartHealth;
use crate::plugin::LoadedPlugin;
use crate::plugin_failure::{PluginFailure, PluginFailureKind};
use crate::plugin_part::PluginPart;

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
pub struct CalledPart {
    plugin: Arc<LoadedPlugin>,
    part: PartRef,
    named: PluginPart,
    health: PartHealth,
}

impl CalledPart {
    /// The plugin.
    pub fn plugin(&self) -> &LoadedPlugin {
        &self.plugin
    }

    /// The part, by its kind and id.
    pub fn part(&self) -> &PartRef {
        &self.part
    }

    /// The plugin and the part, as the reports and a domain's registry
    /// name them.
    pub fn named(&self) -> &PluginPart {
        &self.named
    }

    /// How the part's reads went.
    pub fn health(&self) -> &PartHealth {
        &self.health
    }

    /// The part `part` of `plugin`.
    pub fn new(plugin: Arc<LoadedPlugin>, part: PartRef) -> Self {
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
    pub fn read<T>(
        &self,
        call: &'static str,
        read: impl FnOnce(&mut CallBatch<'_>) -> Result<T, CallError>,
    ) -> Result<T, Box<PluginFailure>> {
        self.read_in(call, Starts::Programs, read)
    }

    /// [`read`](Self::read), whose calls start no program and no hidden
    /// terminal, as while Agents mode is away.
    pub fn read_starting_nothing<T>(
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
    pub fn failure(
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::engine::PluginEngine;
    use crate::testing::{TempDir, load_fixture};

    fn called_part(cache: &TempDir) -> CalledPart {
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = Arc::new(load_fixture(&engine, "minimal"));

        CalledPart::new(plugin, PartRef::agent("test-agent"))
    }

    fn trap(_: &mut CallBatch<'_>) -> Result<(), CallError> {
        Err(CallError::Trap("wasm trap: unreachable".to_string()))
    }

    /// Spec "Engine limits": three failed reads in a row pause the part;
    /// a paused part is not called until Try again.
    #[test]
    fn a_part_pauses_after_three_failed_reads_and_starts_on_try_again() {
        let cache = TempDir::new();
        let part = called_part(&cache);
        let called = Cell::new(false);

        for _ in 0..3 {
            assert!(part.read("sessions", trap).is_err());
        }
        let paused = part.read("sessions", |_| {
            called.set(true);
            Ok(())
        });
        let paused_kind = paused.err().map(|failure| failure.kind());
        part.health().try_again();
        let again = part.read("sessions", |_| Ok(7));

        assert_eq!(paused_kind, Some(PluginFailureKind::Paused));
        assert!(!called.get());
        assert_eq!(again.ok(), Some(7));
        assert!(!part.health().is_paused());
    }

    /// Issue 19: the failure that paused the part stays its last failure;
    /// the paused reads after it do not replace it.
    #[test]
    fn the_last_failure_names_the_part_call_and_kind() {
        let cache = TempDir::new();
        let part = called_part(&cache);

        for _ in 0..3 {
            let _ = part.read("read-answers", trap);
        }
        let _ = part.read("sessions", |_| Ok(()));

        let last = part.health().last_failure().expect("a failure");
        assert_eq!(last.failure().call(), "read-answers");
        assert_eq!(last.failure().kind(), PluginFailureKind::Stopped);
        assert_eq!(last.failure().part().plugin(), "test-plugin");
        assert_eq!(last.failure().part().part(), "test-agent");
    }
}
