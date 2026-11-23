//! The provider contract `otty:agents-provider@0.1.0` as a trait.

use std::marker::PhantomData;

use crate::GivenKey;
use crate::agents::{
    Account, AccountNumbers, Bill, DayRange, Found, LoginFolder, ReadError,
};

/// One company that sells model use: its accounts and their numbers. A
/// plugin implements it once and serves each provider part its manifest
/// lists; `provider` says which part is asked.
///
/// A provider part owns its accounts (spec
/// 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8): OTTY
/// gives it the profile folders of the agent parts its manifest names in
/// `logins`, and the part reads the logins there itself, with the keys
/// replaced by handles. It gives each account a stable id, which OTTY
/// keeps the user's name and Hide by, so the same account must have the
/// same id after its tokens are renewed. Whether two logins are one
/// account is the part's call.
///
/// It reads an account's numbers any way it can, only in the folders the
/// account was found in ([`Account::found_in`]): from the provider's
/// server with the login there through
/// [`http::request`](crate::http::request), or by asking the agent
/// there, such as with a program or a hidden terminal its manifest
/// declares. `given` holds the keys the user gave for the account after
/// a need of the part asked for them; a part that needs a key it has not
/// got answers [`ReadError::NeedsUser`] with an
/// [`AskKey`](crate::NeedAction::AskKey) need. When no way works, the part
/// answers numbers with nothing in them, not an error.
pub trait Provider {
    /// The accounts whose logins sit in `folders`, and the needs that
    /// belong to no account. Default: none, for a part whose manifest
    /// names no `logins` and so gets no folder.
    fn accounts(_provider: &str, _folders: &[LoginFolder]) -> Found {
        Found {
            accounts: Vec::new(),
            needs: Vec::new(),
        }
    }

    /// The account's numbers right now, read any way the part can. Each
    /// of them is optional.
    fn numbers(
        provider: &str,
        account: &Account,
        given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError>;

    /// The bill of the organization `account` over `days`. Default: not
    /// supported.
    fn read_bill(
        _provider: &str,
        _account: &Account,
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
        Account, AccountNumbers, Bill, DayRange, Found, LoginFolder,
        ReadError,
    };
    use crate::bindings::agents_provider::exports::otty::agents_provider::provider::Guest;
    use crate::bindings::host::otty::host::keys::GivenKey as WitGivenKey;
    use crate::{GivenKey, Key};

    impl<T: Provider> Guest for ProviderExport<T> {
        fn accounts(provider: String, folders: Vec<LoginFolder>) -> Found {
            T::accounts(&provider, &folders)
        }

        fn numbers(
            provider: String,
            account: Account,
            given: Vec<WitGivenKey>,
        ) -> Result<AccountNumbers, ReadError> {
            T::numbers(&provider, &account, &given_keys(given))
        }

        fn read_bill(
            provider: String,
            account: Account,
            given: Vec<WitGivenKey>,
            days: DayRange,
        ) -> Result<Bill, ReadError> {
            T::read_bill(&provider, &account, &given_keys(given), &days)
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
