//! The key or the login token of an account, read again for one request
//! from where OTTY found it (spec 2026-11-19, Decisions 2 and 5): OTTY
//! keeps only the fingerprint of a key, so the key itself is read from the
//! agent's file, the keyring or the place the user gave right before the
//! request and dropped right after it.

use std::fmt;

use crate::admin_key::AdminKey;

/// The key or the login token of an account, read for one request, with
/// the organization the provider must be told, such as the team of a Kilo
/// login. It can not be copied, and printing it shows no part of the key.
pub struct AccountKey {
    key: AdminKey,
    organization_id: Option<String>,
}

impl AccountKey {
    /// The key, to put into the header of the request.
    pub(crate) fn key(&self) -> &AdminKey {
        &self.key
    }

    /// The organization the request is for, when the login names one.
    pub(crate) fn organization_id(&self) -> Option<&str> {
        self.organization_id.as_deref()
    }

    /// The key `key` of an account with no organization.
    pub(crate) fn new(key: AdminKey) -> Self {
        Self {
            key,
            organization_id: None,
        }
    }

    /// The same key, for the organization `organization_id` when there is
    /// one.
    pub(crate) fn with_organization_id(
        mut self,
        organization_id: Option<String>,
    ) -> Self {
        self.organization_id = organization_id;
        self
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
        let key = AccountKey::new(AdminKey::made_up("sk-made-up-key-0000"))
            .with_organization_id(Some(String::from("made-up-team")));

        let printed = format!("{key:?}");

        assert_eq!(printed, "AccountKey(hidden)");
        assert_eq!(key.organization_id(), Some("made-up-team"));
    }
}
