//! Values for the tests of the crates that use this one, behind the
//! `fixtures` feature: the answer of a walk over made-up sources, the
//! file a price source keeps its copy in, a change of the format of the
//! session files, also one a plugin part saw, the keyring entry of Goose,
//! and a stand-in for Claude Code, which a plugin reads now.

mod stand_in;

use std::path::{Path, PathBuf};

pub use self::stand_in::claude_code_stand_in;
use crate::error::SourceError;
use crate::keyring_entry::KeyringEntry;
use crate::price_source::PriceSource;
use crate::record_format::{
    FormatChange, RecordCounts, RecordFormat, format_change as judged,
};
use crate::registry::PluginPart;
use crate::source_id::SourceId;
use crate::walk::{Answer, SourceFailure, walk};

/// What the walk over sources that gave `answers`, in order, gives: the
/// answer of the first one that answered, with the failures before it, or
/// every failure.
///
/// ```
/// use otty_agents::fixtures::walked;
/// use otty_agents::{SourceError, SourceId};
///
/// let answer = walked([
///     (
///         SourceId::LiteLlmPrices,
///         Err(SourceError::Network(String::from("down"))),
///     ),
///     (SourceId::KeptPrices, Ok(42)),
/// ])
/// .expect("the kept file answers");
///
/// assert_eq!(answer.source(), SourceId::KeptPrices);
/// assert_eq!(answer.earlier_failures().len(), 1);
/// ```
pub fn walked<T>(
    answers: impl IntoIterator<Item = (SourceId, Result<T, SourceError>)>,
) -> Result<Answer<T>, Vec<SourceFailure>> {
    walk(answers.into_iter())
}

/// The file a [`PriceSource`] that keeps its copy in `kept_dir` keeps the
/// last good price file in.
///
/// ```
/// use std::path::Path;
///
/// use otty_agents::fixtures::kept_price_file;
///
/// assert!(
///     kept_price_file(Path::new("/tmp/prices")).starts_with("/tmp/prices")
/// );
/// ```
pub fn kept_price_file(kept_dir: &Path) -> PathBuf {
    PriceSource::new(kept_dir).kept_file()
}

/// A change of the format of made-up session files of an agent tested up
/// to 2.1.0: the newest files hold `seen` records of the kind OTTY reads,
/// `understood` of them understood, and name the agent's `version`.
///
/// # Panics
///
/// When enough records are understood to make no change.
///
/// ```
/// use otty_agents::fixtures::format_change;
///
/// let change = format_change(None, 40, 0);
///
/// assert_eq!((change.seen(), change.understood()), (40, 0));
/// assert_eq!(change.tested_version(), "2.1.0");
/// ```
pub fn format_change(
    version: Option<&str>,
    seen: u64,
    understood: u64,
) -> FormatChange {
    let format = RecordFormat::new("session file", "2.1.0");
    let mut counts = RecordCounts::default();
    for _ in 0..understood {
        counts.add_understood();
    }
    for _ in understood..seen {
        counts.add_not_understood("message.usage", "message", None);
    }
    if let Some(version) = version {
        counts.saw_version(version);
    }

    judged(format, [&counts]).expect("too few records are understood")
}

/// A change of the format of made-up session files read by the agent
/// part `example-agent` of the plugin Example 0.1.0, tested up to 1.4.2:
/// the newest files hold `seen` records, none understood, and name the
/// agent's `version`.
///
/// ```
/// use otty_agents::fixtures::plugin_format_change;
///
/// let change = plugin_format_change("1.5.0", 6);
///
/// assert_eq!(change.plugin().map(|part| part.name()), Some("Example"));
/// ```
pub fn plugin_format_change(version: &str, seen: u64) -> FormatChange {
    let part = PluginPart::new(
        "example",
        "Example",
        "0.1.0",
        "https://github.com/otty-shell/otty-plugin-example",
        "example-agent",
    );
    let format = RecordFormat::of_plugin(part, Some("1.4.2"));
    let counts = RecordCounts::of_plugin(
        seen,
        0,
        Some(version),
        vec!["tokens".to_string()],
        Vec::new(),
    );

    judged(format, [&counts]).expect("no record is understood")
}

/// The keyring entry of Goose, as [`keyring_entries`] lists it when Goose
/// is on the computer.
///
/// [`keyring_entries`]: crate::keyring_entries
///
/// ```
/// let entry = otty_agents::fixtures::goose_keyring_entry();
///
/// assert_eq!(entry.service(), "goose");
/// ```
pub fn goose_keyring_entry() -> KeyringEntry {
    KeyringEntry::goose()
}

/// The health of a plugin part paused after three failed reads in a row,
/// as the Plugins page shows it with Try again.
///
/// ```
/// use otty_agents::fixtures::paused_health;
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
