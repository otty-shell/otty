//! What an agent or provider part answered it could not read, as the
//! short tag a failure report names (issue 21): the `read-error` of the
//! `agents` contracts is the domain's, so the host's failure kinds know
//! only that the part could not read.

use otty_plugin_host::PluginFailureKind;
use otty_plugin_host::agents::agent::ReadError;

/// The kind of failure of a read the part answered with `error`; a host
/// operation that failed is named by the host's kind of its error.
pub(crate) fn read_failure(error: &ReadError) -> PluginFailureKind {
    let reason = match error {
        ReadError::Host(error) => {
            return PluginFailureKind::of_host_error(error);
        },
        ReadError::NotSupported => "the part does not read this",
        ReadError::Unexpected(_) => "unexpected shape",
        ReadError::NeedsUser(_) => "a need OTTY can not show",
        ReadError::LoginRefused => "the login was refused",
        ReadError::Failed(_) => "failed",
    };

    PluginFailureKind::CouldNotRead(reason)
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agents::agent::HostError;
    use otty_plugin_host::{PluginFailure, PluginPart};

    use super::*;

    /// Spec "What the UI shows": the report of a read the part could not
    /// do names the kind of its error and never its text.
    #[test]
    fn a_read_error_is_named_by_its_kind_only() {
        let part = PluginPart::new("example", "Example", "0.1.0", "", "coder");
        let error = ReadError::Host(HostError::NotFound(
            "/home/user/.example-agent/secret-session.jsonl".to_string(),
        ));

        let report =
            PluginFailure::new(part, "read-answers", read_failure(&error))
                .report();

        assert!(
            report.contains(
                "Error: the plugin could not read what it needs (host \
                 error: not found)"
            ),
            "{report}"
        );
        assert!(!report.contains("secret"), "{report}");
        assert_eq!(
            read_failure(&ReadError::Unexpected("message.usage".into())),
            PluginFailureKind::CouldNotRead("unexpected shape")
        );
        assert_eq!(
            read_failure(&ReadError::LoginRefused),
            PluginFailureKind::CouldNotRead("the login was refused")
        );
    }
}
