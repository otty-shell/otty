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
use crate::keys::{KeyHandle, KeySource};
use crate::operations::v0_1::otty::host::types::ReadError;
/// The newest provider contract's types, which the rest of OTTY uses.
pub use crate::provider::v0_1::exports::otty::provider::provider::{
    AccountFacts, AccountNumbers, Bill, DayCost, DayRange, ExtraUsage,
    KeyLimit, LoginFacts, Money,
};
use crate::provider::v0_1::exports::otty::provider::provider::{
    GivenKey, Guest, GuestIndices,
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
/// [`CallBatch`]. The key of each call is given by its source, a place or
/// a key the user gave by hand: the part gets a handle to it for that
/// call only, never the key. The keys the user gave for the account after
/// a need of the part asked for them go with their slot, each also as a
/// handle. Which keys a part may get is the caller's choice (only
/// accounts of the part's own provider id or hosts).
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

    /// Whose account the key of `key` is.
    pub fn account(
        &mut self,
        key: &KeySource,
        found: &LoginFacts,
    ) -> Result<Result<AccountFacts, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_keys(key, &[], |guest, store, key, _| {
            guest.call_account(store, &provider, key, found)
        })
    }

    /// The numbers of `account` right now, read with the key of `key` and
    /// the keys of `given`, each with its slot.
    pub fn numbers(
        &mut self,
        account: &AccountFacts,
        key: &KeySource,
        given: &[(String, KeySource)],
    ) -> Result<Result<AccountNumbers, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_keys(key, given, |guest, store, key, given| {
            guest.call_numbers(store, &provider, account, key, &given)
        })
    }

    /// The bill of `account` over `days`, read with the key of `key` and
    /// the keys of `given`, each with its slot.
    pub fn read_bill(
        &mut self,
        account: &AccountFacts,
        key: &KeySource,
        given: &[(String, KeySource)],
        days: &DayRange,
    ) -> Result<Result<Bill, ReadError>, CallError> {
        let provider = self.batch.part().id().to_string();

        self.call_with_keys(key, given, |guest, store, key, given| {
            guest.call_read_bill(store, &provider, account, key, &given, days)
        })
    }

    /// Runs `call` with the exports of the plugin's version of the
    /// contract, a handle to the key of `source`, lent to the plugin for
    /// this call only, and a handle to each key of `given` with its slot,
    /// given to the plugin for this call.
    fn call_with_keys<T>(
        &mut self,
        source: &KeySource,
        given: &[(String, KeySource)],
        call: impl FnOnce(
            &Guest,
            &mut Store<CallState>,
            Resource<KeyHandle>,
            Vec<GivenKey>,
        ) -> wasmtime::Result<T>,
    ) -> Result<T, CallError> {
        let exports = self.exports.clone();
        let handle = KeyHandle::of(source.clone());

        self.batch
            .call(CallKind::Numbers, |store, instance| match &exports {
                ProviderExports::V0_1(indices) => {
                    let guest = indices.load(&mut *store, instance)?;
                    let owned = store.data_mut().table().push(handle)?;
                    let lent = Resource::new_borrow(owned.rep());
                    let mut given_keys = Vec::new();
                    for (slot, source) in given {
                        let key = KeyHandle::of(source.clone());
                        given_keys.push(GivenKey {
                            slot: slot.clone(),
                            key: store.data_mut().table().push(key)?,
                        });
                    }

                    let result = call(&guest, store, lent, given_keys);
                    store.data_mut().table().delete(owned)?;

                    result
                },
            })
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::{Environment, PartRef};

    use super::*;
    use crate::https::MockHttpsSend;
    use crate::keys::KeyText;
    use crate::operations::v0_1::otty::host::http::Response;
    use crate::operations::v0_1::otty::host::keys::KeyPlace;
    use crate::operations::v0_1::otty::host::types::{Identity, NeedAction};
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{TempDir, engine_with, load_template, template_env};

    const KEY: &str = "ex-made-up-key-0001";

    /// The money left the template's provider part reads with the key of
    /// `key`, the plugin's variables from `env`; the one request it sends
    /// must carry [`KEY`].
    fn money_left_with(key: &KeySource, env: &Environment) -> f64 {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https
            .expect_send()
            .withf(|sent| {
                sent.url == "https://api.example.com/v1/balance"
                    && sent.headers.iter().any(|(name, value)| {
                        name == "authorization"
                            && *value == format!("Bearer {KEY}")
                    })
            })
            .times(1)
            .returning(|_| {
                Ok(Response {
                    status: 200,
                    headers: Vec::new(),
                    body: br#"{"balance": 12.5, "currency": "USD"}"#.to_vec(),
                })
            });
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_template(&engine, env);
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

        let numbers = batch.provider().expect("a provider part").numbers(
            &account,
            key,
            &[],
        );

        let Ok(Ok(numbers)) = numbers else {
            panic!("the part's answer: {numbers:?}");
        };
        let left = numbers.left.expect("the money left");
        assert_eq!(left.currency, "USD");
        left.amount
    }

    #[test]
    fn the_template_provider_reads_the_balance_with_its_key_handle() {
        let env = template_env().with_var("EXAMPLE_CLOUD_KEY", KEY);
        let key =
            KeySource::Place(KeyPlace::EnvVar("EXAMPLE_CLOUD_KEY".to_string()));

        assert_eq!(money_left_with(&key, &env), 12.5);
    }

    /// Issue 20: a key the user gave for a slot a need asked for reaches
    /// the next `read-bill` as a handle with that slot, and the host fills
    /// it into the part's request.
    #[test]
    fn a_given_key_reaches_the_bill_with_its_slot() {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https
            .expect_send()
            .withf(|sent| {
                sent.url.starts_with("https://api.example.com/v1/bill?")
                    && sent.headers.iter().any(|(name, value)| {
                        name == "authorization"
                            && value == "Bearer ex-admin-made-up-0001"
                    })
            })
            .times(1)
            .returning(|_| {
                Ok(Response {
                    status: 200,
                    headers: Vec::new(),
                    body: br#"{"days": [{"day": "2026-11-02", "usd": 4.0}]}"#
                        .to_vec(),
                })
            });
        let engine = engine_with(&cache, https, MockKeyringRead::new());
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
        let key = KeySource::Given(KeyText::new(KEY).expect("a key"));
        let admin = KeyText::new("ex-admin-made-up-0001").expect("a key");
        let days = DayRange {
            first: "2026-11-01".to_string(),
            last: "2026-11-02".to_string(),
        };

        let bill = batch.provider().expect("a provider part").read_bill(
            &account,
            &key,
            &[("admin".to_string(), KeySource::Given(admin))],
            &days,
        );

        let Ok(Ok(bill)) = bill else {
            panic!("the bill: {bill:?}");
        };
        assert_eq!(bill.days[0].cost.amount, 4.0);
    }

    /// With no key under its slot, the part asks the user for one.
    #[test]
    fn a_bill_with_no_given_key_is_a_need_of_the_part() {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https.expect_send().never();
        let engine = engine_with(&cache, https, MockKeyringRead::new());
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
        let key = KeySource::Given(KeyText::new(KEY).expect("a key"));
        let days = DayRange {
            first: "2026-11-01".to_string(),
            last: "2026-11-02".to_string(),
        };

        let bill = batch.provider().expect("a provider part").read_bill(
            &account,
            &key,
            &[],
            &days,
        );

        let Ok(Err(ReadError::NeedsUser(need))) = bill else {
            panic!("a need: {bill:?}");
        };
        assert!(
            matches!(&need.action, NeedAction::AskKey(ask) if ask.slot == "admin"),
            "{need:?}"
        );
    }

    /// Spec 2026-11-19-agent-plugins, Decision 5: a key the user added by
    /// hand reaches the part only as a handle; the host fills it into the
    /// request.
    #[test]
    fn a_key_given_by_hand_reaches_the_part_as_a_handle() {
        let given = KeyText::new(KEY).expect("a filled key");
        let key = KeySource::Given(given);

        assert_eq!(money_left_with(&key, &template_env()), 12.5);
    }
}
