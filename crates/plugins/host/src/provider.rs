//! The provider contract: the bindings of each version the host keeps,
//! the types of the newest version, and the calls into a provider part.
//!
//! A new version goes into its own module next to the old ones, such as
//! `v0_2`; the types below then come from it, and the old version's arm
//! of [`ProviderCalls`] converts its types to them with `From`.

pub mod v0_1;

use wasmtime::Store;
use wasmtime::component::{InstancePre, Resource};

use crate::calls::{CallBatch, CallError, CallKind, CallState};
use crate::contract::ProviderVersion;
use crate::keys::KeyHandle;
use crate::operations::v0_1::otty::host::keys::KeyPlace;
use crate::operations::v0_1::otty::host::types::ReadError;
/// The newest provider contract's types, which the rest of OTTY uses.
pub use crate::provider::v0_1::exports::otty::provider::provider::{
    AccountFacts, AccountNumbers, Bill, DayCost, DayRange, KeyLimit,
    LoginFacts, Money,
};
use crate::provider::v0_1::exports::otty::provider::provider::{
    Guest, GuestIndices,
};

/// The provider contract of one loaded plugin, in the version its `[api]`
/// names.
#[derive(Clone)]
pub(crate) enum ProviderExports {
    V0_1(GuestIndices),
}

impl ProviderExports {
    /// The exports of `version` of the provider contract in the component
    /// of `instance_pre`.
    pub(crate) fn new(
        version: ProviderVersion,
        instance_pre: &InstancePre<CallState>,
    ) -> wasmtime::Result<Self> {
        match version {
            ProviderVersion::V0_1 => {
                Ok(Self::V0_1(GuestIndices::new(instance_pre)?))
            },
        }
    }
}

/// The calls of the provider contract into one provider part, within one
/// [`CallBatch`]. The key of each call is given by its place: the part
/// gets a handle to it for that call only, never the key. Which places a
/// part may get is the caller's choice (only accounts of the part's own
/// provider id or hosts).
pub struct ProviderCalls<'batch, 'plugin> {
    batch: &'batch mut CallBatch<'plugin>,
    exports: ProviderExports,
}

impl<'batch, 'plugin> ProviderCalls<'batch, 'plugin> {
    pub(crate) fn new(
        batch: &'batch mut CallBatch<'plugin>,
        exports: ProviderExports,
    ) -> Self {
        Self { batch, exports }
    }

    /// Whose account the key at `key` is.
    pub fn account(
        &mut self,
        key: &KeyPlace,
        found: &LoginFacts,
    ) -> Result<Result<AccountFacts, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_key(key, |guest, store, key| {
            guest.call_account(store, &provider, key, found)
        })
    }

    /// The numbers of `account` right now, read with the key at `key`.
    pub fn numbers(
        &mut self,
        account: &AccountFacts,
        key: &KeyPlace,
    ) -> Result<Result<AccountNumbers, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_key(key, |guest, store, key| {
            guest.call_numbers(store, &provider, account, key)
        })
    }

    /// The bill of `account` over `days`, read with the admin key at
    /// `admin_key`.
    pub fn read_bill(
        &mut self,
        account: &AccountFacts,
        admin_key: &KeyPlace,
        days: &DayRange,
    ) -> Result<Result<Bill, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_key(admin_key, |guest, store, key| {
            guest.call_read_bill(store, &provider, account, key, days)
        })
    }

    /// Runs `call` with the exports of the plugin's version of the
    /// contract and a handle to the key at `place`, lent to the plugin for
    /// this call only.
    fn call_with_key<T>(
        &mut self,
        place: &KeyPlace,
        call: impl FnOnce(
            &Guest,
            &mut Store<CallState>,
            Resource<KeyHandle>,
        ) -> wasmtime::Result<T>,
    ) -> Result<T, CallError> {
        let exports = self.exports.clone();
        let handle = KeyHandle::new(place.clone());

        self.batch
            .call(CallKind::Numbers, |store, instance| match &exports {
                ProviderExports::V0_1(indices) => {
                    let guest = indices.load(&mut *store, instance)?;
                    let owned = store.data_mut().table().push(handle)?;
                    let lent = Resource::new_borrow(owned.rep());

                    let result = call(&guest, store, lent);
                    store.data_mut().table().delete(owned)?;

                    result
                },
            })
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::PartRef;

    use super::*;
    use crate::engine::PluginEngine;
    use crate::operations::v0_1::otty::host::types::{HostError, Identity};
    use crate::testing::{TempDir, load_template, template_env};

    #[test]
    fn the_template_provider_gets_a_key_handle_and_asks_the_host() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let plugin = load_template(&engine, &template_env());
        let mut batch = plugin
            .start_calls(&PartRef::provider("example-cloud"))
            .expect("a listed part");
        let account = AccountFacts {
            identity: Identity {
                account_id: None,
                organization_id: None,
                email: None,
            },
            plan: None,
        };
        let key = KeyPlace::EnvVar("EXAMPLE_CLOUD_KEY".to_string());

        let numbers = batch
            .provider()
            .expect("a provider part")
            .numbers(&account, &key);

        // Until issue 05 brings `http.send`, the host refuses it, and the
        // part's answer says so: the call, the key handle and the answer
        // made the round trip.
        let Ok(Err(ReadError::Host(HostError::Failed(reason)))) = numbers
        else {
            panic!("the part's own answer: {numbers:?}");
        };
        assert_eq!(reason, "http.send is not in this OTTY yet");
    }
}
