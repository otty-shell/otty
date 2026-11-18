//! The name of a source and why it failed, in the user's language, such as
//! "Session files: not available: no session files". The dashboard, the
//! statistics dialog and the notices of real failures all write them.
//!
//! `otty-agents` gives the reasons as data; this module writes them. Text
//! that came from outside, such as the system's reason a file could not be
//! read, is put in as it came.

use otty_agents::{
    AdminKeyRef, ClaudeQuestion, ProgramError, SourceError, SourceFailure,
    SourceId, Unavailable, Unexpected,
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
        SourceError::LoginExpired(agent) => i18n::filled(
            Key::TplAgentsFailureLoginExpired,
            &[("reason", &renewed_text(agent))],
        ),
        SourceError::KeyRefused => {
            i18n::t(Key::AgentsFailureKeyRefused).to_string()
        },
        SourceError::Network(detail) => {
            i18n::filled(Key::TplAgentsFailureNetwork, &[("detail", detail)])
        },
        SourceError::UnexpectedAnswer(reason) => i18n::filled(
            Key::TplAgentsFailureUnexpected,
            &[("reason", &unexpected_text(reason))],
        ),
        SourceError::Unreadable { path, detail } => i18n::filled(
            Key::TplAgentsFailureUnreadable,
            &[("path", &path.display().to_string()), ("detail", detail)],
        ),
        SourceError::ProgramStopped(program) => i18n::filled(
            Key::TplAgentsFailureProgramStopped,
            &[("program", program)],
        ),
    }
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
        Unavailable::NoLimitsInSessionFiles(count) => i18n::filled(
            Key::TplAgentsNoLimitsInSessionFiles,
            &[("count", &format!("{count}"))],
        ),
        Unavailable::NotAFolder(path) => i18n::filled(
            Key::TplAgentsNotAFolder,
            &[("path", &path.display().to_string())],
        ),
        Unavailable::NoLoginToken => {
            i18n::t(Key::AgentsNoLoginToken).to_string()
        },
        Unavailable::LoginMayNotReadUsage => {
            i18n::t(Key::AgentsLoginMayNotReadUsage).to_string()
        },
        Unavailable::ApiKeyLogin => i18n::t(Key::AgentsApiKeyLogin).to_string(),
        Unavailable::NotStarted { program, reason } => i18n::filled(
            Key::TplAgentsProgramNotStarted,
            &[
                ("program", program),
                ("reason", &program_error_text(reason)),
            ],
        ),
        Unavailable::NotWritten { program, reason } => i18n::filled(
            Key::TplAgentsProgramNotWritten,
            &[
                ("program", program),
                ("reason", &program_error_text(reason)),
            ],
        ),
        Unavailable::NoAnswerInTime { program, seconds } => i18n::filled(
            Key::TplAgentsProgramNoAnswer,
            &[("program", program), ("count", &format!("{seconds}"))],
        ),
        Unavailable::Refused { program, reason } => {
            let reason = match reason {
                Some(reason) => reason,
                None => i18n::t(Key::AgentsNoReasonGiven),
            };
            i18n::filled(
                Key::TplAgentsProgramRefused,
                &[("program", program), ("reason", reason)],
            )
        },
        Unavailable::ProgramNotFound(program) => {
            i18n::filled(Key::TplAgentsProgramMissing, &[("program", program)])
        },
        Unavailable::ClaudeAsks(question) => question_text(question),
        Unavailable::ClaudeRateLimited => {
            i18n::t(Key::AgentsClaudeRateLimited).to_string()
        },
        Unavailable::ReadStopped => i18n::t(Key::AgentsReadStopped).to_string(),
    }
}

/// The name a person reads of the source `id`, such as "Anthropic usage
/// API (direct request)", in the user's language.
pub(crate) fn source_name(id: SourceId) -> &'static str {
    let key = match id {
        SourceId::SessionFiles => Key::AgentsSourceSessionFiles,
        SourceId::AnthropicUsageApi => Key::AgentsSourceAnthropicUsageApi,
        SourceId::ClaudeUsageCommand => Key::AgentsSourceClaudeUsageCommand,
        SourceId::ChatGptUsageApi => Key::AgentsSourceChatgptUsageApi,
        SourceId::CodexAppServer => Key::AgentsSourceCodexAppServer,
        SourceId::LatestCodexSession => Key::AgentsSourceLatestCodexSession,
        SourceId::AnthropicCostReport => Key::AgentsSourceAnthropicCostReport,
        SourceId::SessionFilesAtListPrices => {
            Key::AgentsSourceSessionFilesAtListPrices
        },
        SourceId::LiteLlmPrices => Key::AgentsSourceLitellmPrices,
        SourceId::KeptPrices => Key::AgentsSourceKeptPrices,
    };

    i18n::t(key)
}

/// Each source by its name and why it failed, such as "Session files: not
/// available: no session files; Anthropic usage API: not logged in".
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
        &[("name", name), ("error", reason)],
    )
}

/// Why an expired login needs no new login: `agent` renews it, such as
/// "Claude Code renews it when it runs".
pub(crate) fn renewed_text(agent: &str) -> String {
    i18n::filled(Key::TplAgentsRenewedWhenRun, &[("agent", agent)])
}

/// What an answer has that the code does not expect, such as "the answer
/// is not JSON".
fn unexpected_text(reason: &Unexpected) -> String {
    let key = match reason {
        Unexpected::ServerStatus(code) => {
            return i18n::filled(
                Key::TplAgentsServerAnswered,
                &[("code", &format!("{code}"))],
            );
        },
        Unexpected::NotJson => Key::AgentsAnswerNotJson,
        Unexpected::NotJsonObject => Key::AgentsAnswerNotJsonObject,
        Unexpected::NoLimits => Key::AgentsAnswerNoLimits,
        Unexpected::NotCostReport => Key::AgentsNotCostReport,
        Unexpected::TooManyPages => Key::AgentsTooManyPages,
        Unexpected::NoPrices => Key::AgentsPriceFileNoPrices,
        Unexpected::UsagePanelUnreadable => Key::AgentsUsagePanelUnreadable,
    };

    i18n::t(key).to_string()
}

/// The question Claude Code asks, such as "claude asks to press Enter".
fn question_text(question: &ClaudeQuestion) -> String {
    let key = match question {
        ClaudeQuestion::TrustOtherFolder(folder) => {
            return i18n::filled(
                Key::TplAgentsClaudeAsksTrust,
                &[("path", folder)],
            );
        },
        ClaudeQuestion::UseApiKey => Key::AgentsClaudeAsksApiKey,
        ClaudeQuestion::ChooseLook => Key::AgentsClaudeAsksLook,
        ClaudeQuestion::PressEnter => Key::AgentsClaudeAsksEnter,
        ClaudeQuestion::TrustOwnFolderAgain => Key::AgentsClaudeAsksTrustAgain,
    };

    i18n::t(key).to_string()
}

/// Why an agent's program could not start or be written to, such as "can
/// not run the shell /bin/zsh".
fn program_error_text(reason: &ProgramError) -> String {
    match reason {
        ProgramError::NotNow => i18n::t(Key::AgentsProgramNotNow).to_string(),
        ProgramError::ShellNotFound(shell) => {
            i18n::filled(Key::TplAgentsShellNotFound, &[("shell", shell)])
        },
        ProgramError::ShellNotRun { shell, detail } => i18n::filled(
            Key::TplAgentsShellNotRun,
            &[("shell", shell), ("detail", detail)],
        ),
        ProgramError::FolderNotCreated { folder, detail } => i18n::filled(
            Key::TplAgentsFolderNotCreated,
            &[("path", &folder.display().to_string()), ("detail", detail)],
        ),
        ProgramError::TerminalNotStarted(detail) => i18n::filled(
            Key::TplAgentsTerminalNotStarted,
            &[("detail", detail)],
        ),
        ProgramError::OutputNotRead(detail) => i18n::filled(
            Key::TplAgentsProgramOutputNotRead,
            &[("detail", detail)],
        ),
        ProgramError::NoOutput => {
            i18n::t(Key::AgentsProgramNoOutput).to_string()
        },
        ProgramError::NeedsUnix => {
            i18n::t(Key::AgentsHiddenTerminalNeedsUnix).to_string()
        },
        ProgramError::Ended => i18n::t(Key::AgentsProgramEnded).to_string(),
        ProgramError::NotWritten(detail) => detail.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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

    fn not_started(reason: ProgramError) -> SourceError {
        not_available(Unavailable::NotStarted {
            program: "claude",
            reason,
        })
    }

    /// One failure of each kind, with each reason and each program error.
    fn every_failure() -> Vec<SourceError> {
        let path = PathBuf::from("/home/user/.claude/projects");
        let detail = String::from("Permission denied (os error 13)");
        let key_file = AdminKeyRef::File(PathBuf::from("/home/user/admin-key"));
        let key_variable = AdminKeyRef::EnvVar(String::from("ADMIN_KEY"));
        let program = "codex app-server";

        vec![
            SourceError::NotLoggedIn,
            SourceError::LoginExpired("Claude Code"),
            SourceError::KeyRefused,
            SourceError::Network(String::from("connection refused")),
            SourceError::Unreadable {
                path: path.clone(),
                detail: detail.clone(),
            },
            SourceError::ProgramStopped(program),
            not_available(Unavailable::NoStatistics(AgentId::new("codex"))),
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
            not_available(Unavailable::NoLimitsInSessionFiles(10)),
            not_available(Unavailable::NotAFolder(path.clone())),
            not_available(Unavailable::NoLoginToken),
            not_available(Unavailable::LoginMayNotReadUsage),
            not_available(Unavailable::ApiKeyLogin),
            not_available(Unavailable::NotWritten {
                program,
                reason: ProgramError::NotWritten(String::from("Broken pipe")),
            }),
            not_available(Unavailable::NoAnswerInTime {
                program,
                seconds: 10,
            }),
            not_available(Unavailable::Refused {
                program,
                reason: Some(String::from("Already initialized")),
            }),
            not_available(Unavailable::Refused {
                program,
                reason: None,
            }),
            not_available(Unavailable::ProgramNotFound("claude")),
            not_available(Unavailable::ClaudeAsks(ClaudeQuestion::UseApiKey)),
            not_available(Unavailable::ClaudeAsks(ClaudeQuestion::ChooseLook)),
            not_available(Unavailable::ClaudeAsks(ClaudeQuestion::PressEnter)),
            not_available(Unavailable::ClaudeAsks(
                ClaudeQuestion::TrustOtherFolder(String::from("~/work")),
            )),
            not_available(Unavailable::ClaudeAsks(
                ClaudeQuestion::TrustOwnFolderAgain,
            )),
            not_available(Unavailable::ClaudeRateLimited),
            not_available(Unavailable::ReadStopped),
            not_started(ProgramError::NotNow),
            not_started(ProgramError::ShellNotFound(String::from("/bin/zsh"))),
            not_started(ProgramError::ShellNotRun {
                shell: String::from("/bin/zsh"),
                detail: detail.clone(),
            }),
            not_started(ProgramError::FolderNotCreated {
                folder: path,
                detail: detail.clone(),
            }),
            not_started(ProgramError::TerminalNotStarted(detail.clone())),
            not_started(ProgramError::OutputNotRead(detail)),
            not_started(ProgramError::NoOutput),
            not_started(ProgramError::NeedsUnix),
            not_started(ProgramError::Ended),
            unexpected(Unexpected::ServerStatus(502)),
            unexpected(Unexpected::NotJson),
            unexpected(Unexpected::NotJsonObject),
            unexpected(Unexpected::NoLimits),
            unexpected(Unexpected::NotCostReport),
            unexpected(Unexpected::TooManyPages),
            unexpected(Unexpected::NoPrices),
            unexpected(Unexpected::UsagePanelUnreadable),
        ]
    }

    /// The app writes every failure in English as `otty-agents` writes it
    /// in its log.
    #[test]
    fn in_english_a_failure_reads_as_the_log_writes_it() {
        for error in every_failure() {
            assert_eq!(source_error_text(&error), error.to_string());
        }
    }

    #[test]
    fn a_login_renewed_by_the_agent_names_the_agent() {
        assert_eq!(
            renewed_text("Claude Code"),
            "Claude Code renews it when it runs"
        );
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
