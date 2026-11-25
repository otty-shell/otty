//! The language receiver: the bindings of each version the host keeps and
//! the one call into it (issue 20). It belongs to no part of the plugin:
//! OTTY calls it when the user changes the language of its interface.

pub mod v0_1;

use wasmtime::component::InstancePre;

use crate::calls::{CallBatch, CallError, CallKind, CallState};
use crate::contract::LanguageVersion;
use crate::language::v0_1::exports::otty::language::language::GuestIndices;

/// The language receiver of one loaded plugin, in the version its
/// component exports.
#[derive(Clone)]
pub(crate) enum LanguageExports {
    V0_1(GuestIndices),
}

impl LanguageExports {
    /// The exports of `version` of the language receiver in the component
    /// of `instance_pre`.
    pub(crate) fn new(
        version: LanguageVersion,
        instance_pre: &InstancePre<CallState>,
    ) -> wasmtime::Result<Self> {
        match version {
            LanguageVersion::V0_1 => {
                Ok(Self::V0_1(GuestIndices::new(instance_pre)?))
            },
        }
    }

    /// Tells the plugin of `batch` that the language is `tag` now; true
    /// when its texts depend on the language.
    pub(crate) fn language_changed(
        &self,
        batch: &mut CallBatch<'_>,
        tag: &str,
    ) -> Result<bool, CallError> {
        batch.call(CallKind::Quick, |store, instance| match self {
            LanguageExports::V0_1(indices) => {
                let guest = indices.load(&mut *store, instance)?;

                guest.call_language_changed(store, tag)
            },
        })
    }
}
