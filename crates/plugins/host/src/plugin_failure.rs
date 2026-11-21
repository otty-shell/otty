//! Why a plugin part gave no answer, and the report the user copies for
//! the plugin's author (spec 2026-11-19-agent-plugins, "What the UI
//! shows"): it names the plugin, its version and repository, the part, the
//! call and the kind of error, never a value the part read. The kinds are
//! those of any domain; a domain turns what its parts answer they could
//! not read into a short tag of [`PluginFailureKind::CouldNotRead`].

use crate::calls::CallError;
use crate::operations::v0_1::otty::host::types::HostError;
use crate::plugin_part::PluginPart;

/// The kind of error of a call into a plugin part, without its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginFailureKind {
    /// The plugin trapped, used more than its memory, or could not start.
    Stopped,
    /// The call ran past its deadline and was stopped.
    TooSlow,
    /// The part is paused after three failed reads in a row, until Try
    /// again or a restart.
    Paused,
    /// The part answered that it could not read, for the reason `0`, such
    /// as "host error: not found": a host operation refused, or a reason
    /// of the domain's contract.
    CouldNotRead(&'static str),
}

impl PluginFailureKind {
    /// The kind in plain English, as the log and the app write it.
    pub fn text(self) -> &'static str {
        match self {
            PluginFailureKind::Stopped => "the plugin stopped with an error",
            PluginFailureKind::TooSlow => "the plugin did not finish in time",
            PluginFailureKind::Paused => "paused after three failures in a row",
            PluginFailureKind::CouldNotRead(_) => {
                "the plugin could not read what it needs"
            },
        }
    }

    /// The kind of `error`, a call that failed.
    pub fn of_call(error: &CallError) -> Self {
        match error {
            CallError::MissedDeadline(_) => PluginFailureKind::TooSlow,
            _ => PluginFailureKind::Stopped,
        }
    }

    /// The kind of a read the part gave up on because the host operation
    /// failed with `error`; its text, which may name a path, is left out.
    pub fn of_host_error(error: &HostError) -> Self {
        let reason = match error {
            HostError::NotAllowed(_) => "host error: not allowed",
            HostError::NotFound(_) => "host error: not found",
            HostError::TooBig(_) => "host error: too big",
            HostError::TimedOut(_) => "host error: timed out",
            HostError::Failed(_) => "host error: failed",
        };

        PluginFailureKind::CouldNotRead(reason)
    }
}

/// A read of a plugin part that gave no answer: the part, the call that
/// failed, such as `read-answers`, and the kind of error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginFailure {
    part: PluginPart,
    call: &'static str,
    kind: PluginFailureKind,
}

impl PluginFailure {
    /// The plugin part that failed.
    pub fn part(&self) -> &PluginPart {
        &self.part
    }

    /// The call that failed, as the contract names it, such as
    /// `read-answers`.
    pub fn call(&self) -> &'static str {
        self.call
    }

    /// The kind of error.
    pub fn kind(&self) -> PluginFailureKind {
        self.kind
    }

    /// The failure of the call `call` of `part`, of the kind `kind`.
    pub fn new(
        part: PluginPart,
        call: &'static str,
        kind: PluginFailureKind,
    ) -> Self {
        Self { part, call, kind }
    }

    /// The plain text report for the plugin's author: the plugin, its
    /// version and repository, the part, the call and the kind of error.
    /// It is in English and holds no key, token, path or text the part
    /// read.
    ///
    /// ```
    /// use otty_plugin_host::{PluginFailure, PluginFailureKind, PluginPart};
    ///
    /// let part = PluginPart::new(
    ///     "example",
    ///     "Example",
    ///     "0.4.0",
    ///     "https://github.com/otty-shell/otty-plugin-example",
    ///     "coder",
    /// );
    /// let failure =
    ///     PluginFailure::new(part, "read-answers", PluginFailureKind::TooSlow);
    ///
    /// assert!(failure.report().contains("Call: read-answers"));
    /// ```
    pub fn report(&self) -> String {
        let mut lines = vec![
            format!(
                "The plugin {} {} could not read.",
                self.part.name(),
                self.part.version()
            ),
            String::new(),
        ];
        lines.extend(self.part.report_lines());
        lines.push(format!("Call: {}", self.call));
        lines.push(match self.kind {
            PluginFailureKind::CouldNotRead(reason) => {
                format!("Error: {} ({reason})", self.kind.text())
            },
            _ => format!("Error: {}", self.kind.text()),
        });

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn part() -> PluginPart {
        PluginPart::new(
            "example",
            "Example",
            "0.1.0",
            "https://github.com/otty-shell/otty-plugin-example",
            "example-agent",
        )
    }

    /// Spec "What the UI shows": the report names the plugin id, version,
    /// repository, part id, the call and the error kind, and no value.
    #[test]
    fn the_report_names_the_part_the_call_and_the_kind_only() {
        let error = HostError::NotFound(
            "/home/user/.example-agent/secret-session.jsonl".to_string(),
        );
        let failure = PluginFailure::new(
            part(),
            "read-answers",
            PluginFailureKind::of_host_error(&error),
        );

        let report = failure.report();

        for line in [
            "The plugin Example 0.1.0 could not read.",
            "Plugin: example 0.1.0",
            "Repository: https://github.com/otty-shell/otty-plugin-example",
            "Part: example-agent",
            "Call: read-answers",
            "Error: the plugin could not read what it needs (host error: \
             not found)",
        ] {
            assert!(report.contains(line), "{line} in {report}");
        }
        assert!(!report.contains("secret"), "{report}");
    }

    #[test]
    fn a_missed_deadline_is_too_slow_and_any_other_stop_stopped() {
        let late = CallError::MissedDeadline(Duration::from_secs(2));
        let trap = CallError::Trap("wasm trap: unreachable".to_string());

        assert_eq!(
            PluginFailureKind::of_call(&late),
            PluginFailureKind::TooSlow
        );
        assert_eq!(
            PluginFailureKind::of_call(&trap),
            PluginFailureKind::Stopped
        );
        assert_eq!(
            PluginFailureKind::of_call(&CallError::MemoryCap),
            PluginFailureKind::Stopped
        );
    }

    #[test]
    fn a_host_error_is_named_by_its_kind_only() {
        let kinds = [
            HostError::NotAllowed("~/.ssh".to_string()),
            HostError::TooBig("8 MiB".to_string()),
            HostError::TimedOut("10 s".to_string()),
            HostError::Failed("no".to_string()),
        ]
        .map(|error| PluginFailureKind::of_host_error(&error));

        assert_eq!(
            kinds,
            [
                PluginFailureKind::CouldNotRead("host error: not allowed"),
                PluginFailureKind::CouldNotRead("host error: too big"),
                PluginFailureKind::CouldNotRead("host error: timed out"),
                PluginFailureKind::CouldNotRead("host error: failed"),
            ]
        );
    }
}
