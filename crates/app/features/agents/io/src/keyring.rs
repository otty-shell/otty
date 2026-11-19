//! The read of the system keyring: the entries of the known agents the
//! user allowed OTTY to read (spec 2026-11-19, Decision 8). The system may
//! ask the user to unlock the keyring, so the read runs on a thread of its
//! own and the feature asks for it only while Agents mode is on screen.

use iced::Task;
use otty_agents::{KeyringEntry, KeyringError};
use otty_app_agents::AgentsIntent;

use crate::background::on_own_thread;

/// Read `entries` of the system keyring in the background; the keys found
/// there are turned into their fingerprints at once and never kept. The
/// answer comes back as [`AgentsIntent::KeyringLoaded`]: the accounts of
/// the keys, or why the keyring could not be read.
pub fn load_keyring(entries: Vec<KeyringEntry>) -> Task<AgentsIntent> {
    on_own_thread(
        String::from("reading the system keyring"),
        move || otty_agents::read_keyring_accounts(&entries),
        || Err(KeyringError::NotAvailable),
        AgentsIntent::KeyringLoaded,
    )
}
