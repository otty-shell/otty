//! Values for the tests of the crates that use this one, such as the
//! Plugins mode: the health of a part that is paused or failed. Only with
//! the `fixtures` feature.

/// The health of a plugin part paused after three failed reads in a row,
/// as Plugins mode shows it with Try again.
///
/// ```
/// use otty_plugin_host::fixtures::paused_health;
///
/// assert!(paused_health().is_paused());
/// ```
pub fn paused_health() -> crate::part_health::PartHealth {
    let health = crate::part_health::PartHealth::default();
    for _ in 0..crate::part_health::PAUSE_AFTER {
        health.failed();
    }

    health
}

/// The health of a plugin part whose last call `failure` failed at `at`,
/// as the card of the part in Plugins mode shows it.
///
/// ```
/// use std::time::SystemTime;
///
/// use otty_plugin_host::fixtures::failed_health;
/// use otty_plugin_host::{PluginFailure, PluginFailureKind, PluginPart};
///
/// let part = PluginPart::new("example", "Example", "0.1.0", "", "coder");
/// let failure =
///     PluginFailure::new(part, "numbers", PluginFailureKind::TooSlow);
///
/// let health = failed_health(&failure, SystemTime::UNIX_EPOCH);
///
/// assert_eq!(
///     health.last_failure().map(|last| last.at()),
///     Some(SystemTime::UNIX_EPOCH)
/// );
/// ```
pub fn failed_health(
    failure: &crate::plugin_failure::PluginFailure,
    at: std::time::SystemTime,
) -> crate::part_health::PartHealth {
    let health = crate::part_health::PartHealth::default();
    health.remember_failure(failure, at);

    health
}
