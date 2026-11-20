//! The key or the login token of an account, read again for one request
//! from where OTTY found it (spec 2026-11-19, Decisions 2 and 5): OTTY
//! keeps only the fingerprint of a key, so the key itself is read from the
//! agent's file, the keyring or the place the user gave right before the
//! request and dropped right after it.

use std::fmt;

use crate::admin_key::AdminKey;

/// The key or the login token of an account, read for one request. It can
/// not be copied, and printing it shows no part of the key.
pub struct AccountKey {
    key: AdminKey,
}

impl AccountKey {
    /// The key, to put into the header of the request.
    pub(crate) fn key(&self) -> &AdminKey {
        &self.key
    }

    /// The key `key` of an account.
    pub(crate) fn new(key: AdminKey) -> Self {
        Self { key }
    }
}

impl fmt::Debug for AccountKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccountKey(hidden)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printing_an_account_key_shows_no_part_of_it() {
        let key = AccountKey::new(AdminKey::made_up("sk-made-up-key-0000"));

        let printed = format!("{key:?}");

        assert_eq!(printed, "AccountKey(hidden)");
    }
}
