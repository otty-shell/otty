//! The provider contract `otty:provider@0.1.0` as a trait.

use std::marker::PhantomData;

use crate::{
    AccountFacts, AccountNumbers, Bill, DayRange, Key, LoginFacts, ReadError,
};

/// One company that sells model use: whose account a key is, and its
/// numbers. A plugin implements it once and serves each provider part its
/// manifest lists; `provider` says which part is asked.
///
/// A provider part gets keys only as handles, and only of accounts of its
/// own provider id or hosts. It reads them through
/// [`http::request`](crate::http::request) to the hosts it declares.
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
    ) -> Result<AccountNumbers, ReadError>;

    /// The bill of an organization over `days`, read with an admin key.
    /// Default: not supported.
    fn read_bill(
        _provider: &str,
        _account: &AccountFacts,
        _admin_key: &Key,
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
    use super::{Provider, ProviderExport};
    use crate::bindings::host::otty::host::keys::Key as WitKey;
    use crate::bindings::provider::exports::otty::provider::provider::Guest;
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
        ) -> Result<AccountNumbers, ReadError> {
            T::numbers(&provider, &account, &Key::lent(key))
        }

        fn read_bill(
            provider: String,
            account: AccountFacts,
            admin_key: &WitKey,
            days: DayRange,
        ) -> Result<Bill, ReadError> {
            T::read_bill(&provider, &account, &Key::lent(admin_key), &days)
        }
    }
}
