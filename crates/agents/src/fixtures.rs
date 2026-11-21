//! Values for the tests of the crates that use this one, behind the
//! `fixtures` feature: the answer of a walk over made-up sources, the
//! file a price source keeps its copy in, a change of the format of the
//! session files, also one a plugin part saw, and stand-ins for the agents
//! the official plugins read.

pub(crate) mod stand_in;

use std::path::{Path, PathBuf};

pub use self::stand_in::{claude_code_stand_in, codex_stand_in, not_logged_in};
use crate::command::AgentCommand;
use crate::error::SourceError;
use crate::part_need::{KeyAsk, PartNeed, PartNeedAction};
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

/// The plugin part `part` of the plugin `plugin` 0.1.0, named after the
/// plugin's id with a capital, such as "Openai" of `openai`.
pub fn plugin_part(plugin: &str, part: &str) -> PluginPart {
    let mut name = plugin.to_string();
    if let Some(first) = name.get_mut(0..1) {
        first.make_ascii_uppercase();
    }

    PluginPart::new(
        plugin,
        &name,
        "0.1.0",
        format!("https://github.com/otty-shell/otty-plugin-{plugin}"),
        part,
    )
}

/// The need of a profile of the agent part `part` of `plugin` nobody is
/// logged in with: "Needs login", whose button runs `login`.
///
/// ```
/// use otty_agents::fixtures::login_need;
/// use otty_agents::{AgentCommand, PartNeedAction};
///
/// let need = login_need("openai", "codex", AgentCommand::new("codex"));
///
/// assert_eq!(need.id(), "not-logged-in");
/// assert!(matches!(need.action(), PartNeedAction::Run(_)));
/// ```
pub fn login_need(plugin: &str, part: &str, login: AgentCommand) -> PartNeed {
    PartNeed::new(
        plugin_part(plugin, part),
        "not-logged-in",
        "Needs login",
        "Nobody is logged in with this config folder.",
        PartNeedAction::Run(login),
    )
}

/// The need of the provider part `part` of `plugin` that asks for a key
/// under `slot`, such as the admin key of an organization's bill.
///
/// ```
/// use otty_agents::PartNeedAction;
/// use otty_agents::fixtures::key_need;
///
/// let need = key_need("anthropic", "anthropic", "admin");
///
/// let PartNeedAction::AskKey(ask) = need.action() else {
///     panic!("a key to ask for");
/// };
/// assert_eq!(ask.slot(), "admin");
/// ```
pub fn key_need(plugin: &str, part: &str, slot: &str) -> PartNeed {
    let ask = KeyAsk::new(slot, "Admin key", "Reads the real bill.")
        .with_prefix_and_page(
            Some(String::from("sk-made-up-")),
            Some(String::from("https://example.com/keys")),
        );

    PartNeed::new(
        plugin_part(plugin, part),
        "bill-key",
        "The real bill needs a key",
        "OTTY shows an estimate from the tokens.",
        PartNeedAction::AskKey(ask),
    )
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

/// The health of a plugin part whose last call `failure` failed at `at`,
/// as the card of the part on the Plugins page shows it.
///
/// ```
/// use std::time::SystemTime;
///
/// use otty_agents::fixtures::failed_health;
/// use otty_agents::{PluginFailure, PluginFailureKind, PluginPart};
///
/// let part = PluginPart::new("example", "Example", "0.1.0", "", "coder");
/// let failure =
///     PluginFailure::new(part, "read-limits", PluginFailureKind::TooSlow);
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
