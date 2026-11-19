//! The system keyring (spec 2026-11-19, Decision 8): the Secret Service on
//! Linux and the Keychain on macOS, where some agents keep their keys.
//!
//! OTTY only reads it. The read is behind [`KeyringRead`], which has no way
//! to write, change or delete an entry, so the reads of the agents' entries
//! are tested with a mock and never reach the real keyring.
//! [`SystemKeyring`] makes the real read with the `keyring` crate.

use thiserror::Error;

/// Why the system keyring could not be read. It never holds what the
/// keyring holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum KeyringError {
    /// No system keyring answered, such as a Linux desktop with no Secret
    /// Service running.
    #[error("no system keyring answered")]
    NotAvailable,
    /// The keyring stayed locked, or the user refused the read when the
    /// system asked.
    #[error("the system keyring stayed locked or refused the read")]
    NotAllowed,
    /// The entry is not in the shape OTTY knows, or several entries answer
    /// for it.
    #[error("the keyring entry could not be read")]
    Unreadable,
}

/// Reads one entry of the system keyring.
///
/// It is a trait because it crosses into the system keyring: the reads are
/// tested with a mock, and the real keyring is never touched by a test. It
/// only reads: an entry is never written, changed, copied or deleted.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait KeyringRead {
    /// The text kept in the entry of `service` and `user`; `None` when
    /// there is no such entry. The system may ask the user to unlock the
    /// keyring first.
    fn password(
        &self,
        service: &str,
        user: &str,
    ) -> Result<Option<String>, KeyringError>;
}

/// The real system keyring: the Secret Service on Linux, the Keychain on
/// macOS, through the `keyring` crate.
pub(crate) struct SystemKeyring;

impl KeyringRead for SystemKeyring {
    fn password(
        &self,
        service: &str,
        user: &str,
    ) -> Result<Option<String>, KeyringError> {
        let entry = keyring::Entry::new(service, user).map_err(failure)?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(failure(error)),
        }
    }
}

/// What a failure of the `keyring` crate means for the user. Only the
/// failures of the platform are logged: they say what the system answered,
/// while the others may carry the bytes of the entry.
fn failure(error: keyring::Error) -> KeyringError {
    match error {
        keyring::Error::NoStorageAccess(_) => KeyringError::NotAllowed,
        keyring::Error::NoDefaultStore => KeyringError::NotAvailable,
        keyring::Error::PlatformFailure(cause) => {
            log::warn!("the system keyring could not be read: {cause}");
            KeyringError::NotAvailable
        },
        _ => KeyringError::Unreadable,
    }
}
