//! What the host operations reach outside OTTY besides files: the network
//! and the system keyring, whether the user allowed the keyring, and the
//! language OTTY's interface shows now. The engine holds the real ones;
//! tests give mocks.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::https::{HttpsSend, UreqClient};
use crate::operations::v0_1::otty::host::types::HostError;
use crate::system_keyring::{KeyringError, KeyringRead, SystemKeyring};

/// The HTTPS client, the system keyring and the language of every plugin
/// call.
pub(crate) struct Outside {
    https: Box<dyn HttpsSend>,
    keyring: Box<dyn KeyringRead>,
    keyring_allowed: AtomicBool,
    language: Mutex<String>,
}

impl Outside {
    /// The HTTPS client.
    pub(crate) fn https(&self) -> &dyn HttpsSend {
        self.https.as_ref()
    }

    /// The language OTTY's interface shows now, as a tag such as `ru`.
    pub(crate) fn language(&self) -> String {
        match self.language.lock() {
            Ok(language) => language.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// `https` and `keyring`, the keyring not allowed yet, the interface
    /// in English.
    pub(crate) fn new(
        https: Box<dyn HttpsSend>,
        keyring: Box<dyn KeyringRead>,
    ) -> Self {
        Self {
            https,
            keyring,
            keyring_allowed: AtomicBool::new(false),
            language: Mutex::new(String::from("en")),
        }
    }

    /// The interface shows the language `tag` from now on: the next
    /// `env.language` of every plugin gives it.
    pub(crate) fn set_language(&self, tag: &str) {
        let mut language = match self.language.lock() {
            Ok(language) => language,
            Err(poisoned) => poisoned.into_inner(),
        };

        *language = tag.to_string();
    }

    /// The network with `ureq` and the system keyring.
    pub(crate) fn real() -> Self {
        Self::new(Box::new(UreqClient::new()), Box::new(SystemKeyring))
    }

    /// Lets plugins read the keyring entries they declare, or stops them,
    /// as the user chose in Agents mode.
    pub(crate) fn set_keyring_allowed(&self, allowed: bool) {
        self.keyring_allowed.store(allowed, Ordering::Relaxed);
    }

    /// The text of the keyring entry of `service` and `user`; `None` when
    /// there is no such entry. Refused until the user allowed OTTY to read
    /// the keyring.
    pub(crate) fn keyring_entry(
        &self,
        service: &str,
        user: &str,
    ) -> Result<Option<String>, HostError> {
        if !self.keyring_allowed.load(Ordering::Relaxed) {
            return Err(HostError::NotAllowed(
                "the system keyring: the user has not allowed OTTY to read it"
                    .to_string(),
            ));
        }

        self.keyring
            .password(service, user)
            .map_err(|error| match error {
                KeyringError::NotAllowed => {
                    HostError::NotAllowed(error.to_string())
                },
                _ => HostError::Failed(error.to_string()),
            })
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;

    use super::*;
    use crate::https::MockHttpsSend;
    use crate::system_keyring::MockKeyringRead;

    fn outside(keyring: MockKeyringRead) -> Outside {
        Outside::new(Box::new(MockHttpsSend::new()), Box::new(keyring))
    }

    #[test]
    fn the_keyring_is_not_read_before_the_user_allows_it() {
        let mut keyring = MockKeyringRead::new();
        keyring.expect_password().never();
        let outside = outside(keyring);

        let entry = outside.keyring_entry("made-up", "work");

        assert!(matches!(entry, Err(HostError::NotAllowed(_))));
    }

    #[test]
    fn an_allowed_keyring_gives_the_entry_or_none() {
        let mut keyring = MockKeyringRead::new();
        keyring
            .expect_password()
            .with(eq("made-up"), eq("work"))
            .returning(|_, _| Ok(Some("ex-made-up-key".to_string())));
        keyring
            .expect_password()
            .with(eq("made-up"), eq("home"))
            .returning(|_, _| Ok(None));
        let outside = outside(keyring);
        outside.set_keyring_allowed(true);

        let work = outside.keyring_entry("made-up", "work");
        let home = outside.keyring_entry("made-up", "home");

        assert_eq!(work.expect("an entry").as_deref(), Some("ex-made-up-key"));
        assert_eq!(home.expect("no entry"), None);
    }

    #[test]
    fn a_locked_keyring_is_a_refusal_and_a_missing_one_a_failure() {
        let mut keyring = MockKeyringRead::new();
        keyring
            .expect_password()
            .with(eq("made-up"), eq("locked"))
            .returning(|_, _| Err(KeyringError::NotAllowed));
        keyring
            .expect_password()
            .with(eq("made-up"), eq("none"))
            .returning(|_, _| Err(KeyringError::NotAvailable));
        let outside = outside(keyring);
        outside.set_keyring_allowed(true);

        let locked = outside.keyring_entry("made-up", "locked");
        let none = outside.keyring_entry("made-up", "none");

        assert!(matches!(locked, Err(HostError::NotAllowed(_))));
        assert!(matches!(none, Err(HostError::Failed(_))));
    }
}
