//! The provider contract `otty:provider@0.1.0` as a trait.

use std::marker::PhantomData;

use crate::{
    AccountFacts, AccountNumbers, Bill, DayRange, Key, LoginFacts, ReadError,
};

/// A key the user gave for an account, under the `slot` a need of the
/// part asked for it with [`NeedAction::AskKey`](crate::NeedAction).
pub struct GivenKey {
    slot: String,
    key: Key,
}

impl GivenKey {
    /// The part's own name for the key, such as `admin`.
    pub fn slot(&self) -> &str {
        &self.slot
    }

    /// The handle to the key.
    pub fn key(&self) -> &Key {
        &self.key
    }

    /// The key a provider call is given under `slot`.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn of_call(slot: String, key: Key) -> Self {
        Self { slot, key }
    }

    /// The key `key` given under `slot`, for the tests of a plugin; the
    /// handle comes from [`TestHost::key`](crate::test_host::TestHost::key).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(slot: &str, key: Key) -> Self {
        Self {
            slot: slot.to_string(),
            key,
        }
    }
}

/// The key of `given` under `slot`, when the user gave one, such as the
/// admin key a part asked for under `admin`.
pub fn given_key<'a>(given: &'a [GivenKey], slot: &str) -> Option<&'a Key> {
    given
        .iter()
        .find(|given| given.slot() == slot)
        .map(GivenKey::key)
}

/// One company that sells model use: whose account a key is, and its
/// numbers. A plugin implements it once and serves each provider part its
/// manifest lists; `provider` says which part is asked.
///
/// A provider part gets keys only as handles, and only of accounts of its
/// own provider id or hosts. It reads them through
/// [`http::request`](crate::http::request) to the hosts it declares.
/// `given` holds the keys the user gave for the account after a need of
/// the part asked for them; a part that needs a key it has not got
/// answers [`ReadError::NeedsUser`] with an
/// [`AskKey`](crate::NeedAction::AskKey) need.
pub trait Provider {
    /// Whose account `key` is: account or organization id, email, plan.
    /// `found` is what OTTY found about the login in the agents' files.
    fn account(
        provider: &str,
        key: &Key,
        found: &LoginFacts,
    ) -> Result<AccountFacts, ReadError>;

    /// The account's numbers right now. Each of them is optional.
    fn numbers(
        provider: &str,
        account: &AccountFacts,
        key: &Key,
        given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError>;

    /// The bill of an organization over `days`, read with the account's
    /// `key` or a key the user gave. Default: not supported.
    fn read_bill(
        _provider: &str,
        _account: &AccountFacts,
        _key: &Key,
        _given: &[GivenKey],
        _days: &DayRange,
    ) -> Result<Bill, ReadError> {
        Err(ReadError::NotSupported)
    }
}

/// What [`export!`](crate::export) exports for a plugin type `T` with
/// `provider`: the binding's trait, served by `T`'s [`Provider`].
#[doc(hidden)]
pub struct ProviderExport<T>(PhantomData<T>);

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::{GivenKey, Provider, ProviderExport};
    use crate::bindings::host::otty::host::keys::Key as WitKey;
    use crate::bindings::provider::exports::otty::provider::provider::{
        GivenKey as WitGivenKey, Guest,
    };
    use crate::{
        AccountFacts, AccountNumbers, Bill, DayRange, Key, LoginFacts,
        ReadError,
    };

    impl<T: Provider> Guest for ProviderExport<T> {
        fn account(
            provider: String,
            key: &WitKey,
            found: LoginFacts,
        ) -> Result<AccountFacts, ReadError> {
            T::account(&provider, &Key::lent(key), &found)
        }

        fn numbers(
            provider: String,
            account: AccountFacts,
            key: &WitKey,
            given: Vec<WitGivenKey>,
        ) -> Result<AccountNumbers, ReadError> {
            T::numbers(&provider, &account, &Key::lent(key), &given_keys(given))
        }

        fn read_bill(
            provider: String,
            account: AccountFacts,
            key: &WitKey,
            given: Vec<WitGivenKey>,
            days: DayRange,
        ) -> Result<Bill, ReadError> {
            T::read_bill(
                &provider,
                &account,
                &Key::lent(key),
                &given_keys(given),
                &days,
            )
        }
    }

    /// The keys OTTY gives a call, each under its slot.
    fn given_keys(given: Vec<WitGivenKey>) -> Vec<GivenKey> {
        given
            .into_iter()
            .map(|given| {
                GivenKey::of_call(given.slot, Key::from_wasm(given.key))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_host::TestHost;
    use crate::testing::MadeUpPlugin;

    #[test]
    fn a_given_key_is_found_by_its_slot_only() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        let given = [GivenKey::new("admin", host.key("made-up-given-key"))];

        assert!(given_key(&given, "admin").is_some());
        assert!(given_key(&given, "other").is_none());
        assert!(given_key(&[], "admin").is_none());
    }
}
