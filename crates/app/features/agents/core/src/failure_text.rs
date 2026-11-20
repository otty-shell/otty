//! The name of a source and why it failed, in the user's language, such as
//! "Session files: not available: no session files". The dashboard, the
//! statistics dialog and the notices of real failures all write them.
//!
//! `otty-agents` gives the reasons as data; this module writes them. Text
//! that came from outside, such as the system's reason a file could not be
//! read, is put in as it came.

use otty_agents::{
    AdminKeyRef, PluginFailureKind, SourceError, SourceFailure, SourceId,
    Unavailable, Unexpected,
};
use otty_app_i18n::{self as i18n, Key};

/// `error` in the user's language, such as "not available: no session
/// files" or "network error: connection refused".
pub(crate) fn source_error_text(error: &SourceError) -> String {
    match error {
        SourceError::NotAvailable(reason) => i18n::filled(
            Key::TplAgentsFailureNotAvailable,
            &[("reason", &unavailable_text(reason))],
        ),
        SourceError::NotLoggedIn => {
            i18n::t(Key::AgentsFailureNotLoggedIn).to_string()
        },
        SourceError::KeyRefused => {
            i18n::t(Key::AgentsFailureKeyRefused).to_string()
        },
        SourceError::Network(detail) => {
            i18n::filled(Key::TplAgentsFailureNetwork, &[("error", detail)])
        },
        SourceError::UnexpectedAnswer(reason) => i18n::filled(
            Key::TplAgentsFailureUnexpected,
            &[("reason", &unexpected_text(reason))],
        ),
        SourceError::Unreadable { path, detail } => i18n::filled(
            Key::TplAgentsFailureUnreadable,
            &[("path", &path.display().to_string()), ("error", detail)],
        ),
        SourceError::FormatChanged(change) => i18n::filled(
            Key::TplAgentsFailureFormatChanged,
            &[
                ("understood", &change.understood().to_string()),
                ("seen", &change.seen().to_string()),
            ],
        ),
        SourceError::PluginFailed(failure) => i18n::filled(
            Key::TplAgentsFailurePlugin,
            &[
                ("plugin", failure.part().name()),
                ("version", failure.part().version()),
                ("reason", plugin_failure_text(failure.kind())),
            ],
        ),
    }
}

/// Why a plugin part gave no answer, such as "the plugin did not finish
/// in time".
fn plugin_failure_text(kind: PluginFailureKind) -> &'static str {
    let key = match kind {
        PluginFailureKind::Stopped => Key::AgentsPluginFailureStopped,
        PluginFailureKind::TooSlow => Key::AgentsPluginFailureTooSlow,
        PluginFailureKind::Paused => Key::AgentsPluginFailurePaused,
        PluginFailureKind::CouldNotRead(_) => {
            Key::AgentsPluginFailureCouldNotRead
        },
    };

    i18n::t(key)
}

/// Why a source can not be used here, such as "no session files".
pub(crate) fn unavailable_text(reason: &Unavailable) -> String {
    match reason {
        Unavailable::NoStatistics(agent) => i18n::filled(
            Key::TplAgentsNoStatistics,
            &[("agent", agent.as_str())],
        ),
        Unavailable::NoBill => i18n::t(Key::AgentsNoBill).to_string(),
        Unavailable::NoAdminKey => i18n::t(Key::AgentsNoAdminKey).to_string(),
        Unavailable::AdminKeyMissing(AdminKeyRef::File(path)) => i18n::filled(
            Key::TplAgentsAdminKeyFileMissing,
            &[("path", &path.display().to_string())],
        ),
        Unavailable::AdminKeyMissing(AdminKeyRef::EnvVar(name)) => {
            i18n::filled(Key::TplAgentsVariableNotSet, &[("name", name)])
        },
        Unavailable::AdminKeyEmpty(AdminKeyRef::File(path)) => i18n::filled(
            Key::TplAgentsAdminKeyFileEmpty,
            &[("path", &path.display().to_string())],
        ),
        Unavailable::AdminKeyEmpty(AdminKeyRef::EnvVar(name)) => {
            i18n::filled(Key::TplAgentsVariableEmpty, &[("name", name)])
        },
        Unavailable::AdminKeyVariableNotValid => {
            i18n::t(Key::AgentsAdminKeyVariableNotValid).to_string()
        },
        Unavailable::NoAccountSource => {
            i18n::t(Key::AgentsNoAccountSource).to_string()
        },
        Unavailable::KeyGone => i18n::t(Key::AgentsKeyGone).to_string(),
        Unavailable::NoKeptPriceFile => {
            i18n::t(Key::AgentsNoKeptPriceFile).to_string()
        },
        Unavailable::NoSessionFile(session) => i18n::filled(
            Key::TplAgentsNoSessionFile,
            &[("id", session.as_str())],
        ),
        Unavailable::NoSessionFiles => {
            i18n::t(Key::AgentsNoSessionFiles).to_string()
        },
        Unavailable::NotAFolder(path) => i18n::filled(
            Key::TplAgentsNotAFolder,
            &[("path", &path.display().to_string())],
        ),
        Unavailable::ProgramNotFound(program) => {
            i18n::filled(Key::TplAgentsProgramMissing, &[("program", program)])
        },
        Unavailable::ReadStopped => i18n::t(Key::AgentsReadStopped).to_string(),
    }
}

/// The name a person reads of the source `id`, such as "LiteLLM prices",
/// in the user's language; the name a plugin part gave a named source.
pub(crate) fn source_name(id: SourceId) -> &'static str {
    let key = match id {
        SourceId::Named(name) => return name.as_str(),
        SourceId::SessionFilesAtListPrices => {
            Key::AgentsSourceSessionFilesAtListPrices
        },
        SourceId::CostWrittenByAgent => Key::AgentsSourceCostWrittenByAgent,
        SourceId::LiteLlmPrices => Key::AgentsSourceLitellmPrices,
        SourceId::KeptPrices => Key::AgentsSourceKeptPrices,
        SourceId::Plugin => Key::AgentsSourcePlugin,
    };

    i18n::t(key)
}

/// Each source by its name and why it failed, such as "Plugin: not
/// available: no session files; LiteLLM prices: network error: timed out".
pub(crate) fn failure_reasons(failures: &[SourceFailure]) -> String {
    let reasons: Vec<String> = failures
        .iter()
        .map(|failure| {
            let reason = source_error_text(failure.error());
            named_failure(source_name(failure.source()), &reason)
        })
        .collect();

    reasons.join(i18n::t(Key::AgentsFailureSeparator))
}

/// Why `name`, a source or a profile, failed: "Work: not logged in".
pub(crate) fn named_failure(name: &str, reason: &str) -> String {
    i18n::filled(
        Key::TplAgentsNamedFailure,
        &[("name", name), ("reason", reason)],
    )
}

/// What an answer has that the code does not expect, such as "the answer
/// is not a JSON object".
fn unexpected_text(reason: &Unexpected) -> String {
    let key = match reason {
        Unexpected::ServerStatus(code) => {
            return i18n::filled(
                Key::TplAgentsServerAnswered,
                &[("code", &format!("{code}"))],
            );
        },
        Unexpected::NotJsonObject => Key::AgentsAnswerNotJsonObject,
        Unexpected::NoPrices => Key::AgentsPriceFileNoPrices,
    };

    i18n::t(key).to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::fixtures::format_change;
    use otty_agents::{AgentId, SessionId};
    use otty_app_i18n::Locale;
    use otty_app_testing::alone::is_alone;

    use super::*;

    fn not_available(reason: Unavailable) -> SourceError {
        SourceError::NotAvailable(reason)
    }

    fn unexpected(reason: Unexpected) -> SourceError {
        SourceError::UnexpectedAnswer(reason)
    }

    /// One failure of each kind, with each reason and each program error.
    fn every_failure() -> Vec<SourceError> {
        let path = PathBuf::from("/home/user/.coder/projects");
        let detail = String::from("Permission denied (os error 13)");
        let key_file = AdminKeyRef::File(PathBuf::from("/home/user/admin-key"));
        let key_variable = AdminKeyRef::EnvVar(String::from("ADMIN_KEY"));

        vec![
            SourceError::NotLoggedIn,
            SourceError::KeyRefused,
            SourceError::Network(String::from("connection refused")),
            SourceError::Unreadable {
                path: path.clone(),
                detail: detail.clone(),
            },
            SourceError::FormatChanged(Box::new(format_change(
                Some("2.2.0"),
                40,
                3,
            ))),
            not_available(Unavailable::NoStatistics(AgentId::new("coder"))),
            not_available(Unavailable::NoBill),
            not_available(Unavailable::NoAdminKey),
            not_available(Unavailable::AdminKeyMissing(key_file.clone())),
            not_available(Unavailable::AdminKeyMissing(key_variable.clone())),
            not_available(Unavailable::AdminKeyEmpty(key_file)),
            not_available(Unavailable::AdminKeyEmpty(key_variable)),
            not_available(Unavailable::AdminKeyVariableNotValid),
            not_available(Unavailable::NoKeptPriceFile),
            not_available(Unavailable::NoSessionFile(SessionId::new("a1"))),
            not_available(Unavailable::NoSessionFiles),
            not_available(Unavailable::NotAFolder(path.clone())),
            not_available(Unavailable::ProgramNotFound("coder")),
            not_available(Unavailable::ReadStopped),
            unexpected(Unexpected::ServerStatus(502)),
            unexpected(Unexpected::NotJsonObject),
            unexpected(Unexpected::NoPrices),
            plugin_failed(PluginFailureKind::Stopped),
            plugin_failed(PluginFailureKind::TooSlow),
            plugin_failed(PluginFailureKind::Paused),
            plugin_failed(PluginFailureKind::CouldNotRead("failed")),
        ]
    }

    fn plugin_failed(kind: PluginFailureKind) -> SourceError {
        let part = otty_agents::PluginPart::new(
            "example",
            "Example",
            "0.1.0",
            "https://github.com/otty-shell/otty-plugin-example",
            "example-agent",
        );

        let failure =
            otty_agents::PluginFailure::new(part, "read-answers", kind);

        SourceError::PluginFailed(Box::new(failure))
    }

    /// The app writes every failure in English as `otty-agents` writes it
    /// in its log.
    #[test]
    fn in_english_a_failure_reads_as_the_log_writes_it() {
        for error in every_failure() {
            assert_eq!(source_error_text(&error), error.to_string());
        }
    }

    /// It switches the language of the whole process, so it runs alone in
    /// a copy of the test program.
    #[test]
    fn a_failure_is_written_in_the_language_of_the_app() {
        if !is_alone("a_failure_is_written_in_the_language_of_the_app") {
            return;
        }

        i18n::set_locale(Locale::Ru);

        assert_eq!(
            source_error_text(&not_available(Unavailable::NoSessionFiles)),
            "недоступно: нет файлов сессий"
        );
        assert_eq!(
            source_error_text(&SourceError::Network(String::from(
                "connection refused"
            ))),
            "ошибка сети: connection refused"
        );
        for error in every_failure() {
            assert_ne!(source_error_text(&error), error.to_string());
        }
    }
}
