//! The provider contract `otty:agents-provider@0.1.0` as a trait.

use std::marker::PhantomData;

use crate::agents::{
    AccountFacts, AccountNumbers, Bill, DayRange, LoginFacts, ReadError,
};
use crate::{GivenKey, Key};

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
///
/// `found` tells what OTTY found about the login, with every profile the
/// account was found in ([`LoginFacts::found_in`]): a part may read the
/// numbers by asking the agent there, such as with a program or a hidden
/// terminal its manifest declares, when the key does not do. A key that
/// is not at its place is a normal case: using its handle fails with
/// [`HostError::NotFound`](crate::HostError::NotFound). When no way
/// works, the part answers numbers with nothing in them, not an error.
pub trait Provider {
    /// Whose account `key` is: account or organization id, email, plan.
    /// `found` is what OTTY found about the login in the agents' files.
    fn account(
        provider: &str,
        key: &Key,
        found: &LoginFacts,
    ) -> Result<AccountFacts, ReadError>;

    /// The account's numbers right now, read any way the part can. Each
    /// of them is optional.
    fn numbers(
        provider: &str,
        found: &LoginFacts,
        account: &AccountFacts,
        key: &Key,
        given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError>;

    /// The bill of an organization over `days`, read with the account's
    /// `key` or a key the user gave. Default: not supported.
    fn read_bill(
        _provider: &str,
        _found: &LoginFacts,
        _account: &AccountFacts,
        _key: &Key,
        _given: &[GivenKey],
        _days: &DayRange,
    ) -> Result<Bill, ReadError> {
        Err(ReadError::NotSupported)
    }
}

/// What [`export!`](crate::export) exports for a plugin type `T` with
/// `agents::provider`: the binding's trait, served by `T`'s
/// [`Provider`].
#[doc(hidden)]
pub struct ProviderExport<T>(PhantomData<T>);

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::{Provider, ProviderExport};
    use crate::agents::{
        AccountFacts, AccountNumbers, Bill, DayRange, LoginFacts, ReadError,
    };
    use crate::bindings::agents_provider::exports::otty::agents_provider::provider::Guest;
    use crate::bindings::host::otty::host::keys::{
        GivenKey as WitGivenKey, Key as WitKey,
    };
    use crate::{GivenKey, Key};

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
            found: LoginFacts,
            account: AccountFacts,
            key: &WitKey,
            given: Vec<WitGivenKey>,
        ) -> Result<AccountNumbers, ReadError> {
            T::numbers(
                &provider,
                &found,
                &account,
                &Key::lent(key),
                &given_keys(given),
            )
        }

        fn read_bill(
            provider: String,
            found: LoginFacts,
            account: AccountFacts,
            key: &WitKey,
            given: Vec<WitGivenKey>,
            days: DayRange,
        ) -> Result<Bill, ReadError> {
            T::read_bill(
                &provider,
                &found,
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
