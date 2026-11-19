//! Why a source gave no answer.
//!
//! The reasons are data, not sentences: the app writes them in the user's
//! language. The text of `Display` is the English one, for the log. Only
//! text that comes from outside, such as the system's reason a file could
//! not be read or a server's own message, is kept as it came.

use std::path::PathBuf;

use crate::admin_key::AdminKeyRef;
use crate::agent::AgentId;
use crate::program::ProgramError;
use crate::record_format::FormatChange;
use crate::session::SessionId;

/// Why a source gave no answer. Any of these moves on to the next source
/// (spec Decision 5).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// The source can not be used here, for example its file or program is
    /// missing.
    #[error("not available: {0}")]
    NotAvailable(Unavailable),
    /// The login is missing, or it expired and nothing renews it: the user
    /// has to log in.
    #[error("not logged in")]
    NotLoggedIn,
    /// The login expired, but the agent renews it by itself, so the user
    /// does not have to log in yet. It names the agent that renews it, such
    /// as "Claude Code".
    #[error("login expired: {0} renews it when it runs")]
    LoginExpired(&'static str),
    /// The server refused the key the source was given, such as an admin
    /// key that was revoked or is not an admin key.
    #[error("the key was refused")]
    KeyRefused,
    /// The server could not be reached. The text is the network library's
    /// own reason.
    #[error("network error: {0}")]
    Network(String),
    /// The answer is not what the code expects.
    #[error("unexpected answer: {0}")]
    UnexpectedAnswer(Unexpected),
    /// A folder or file that exists could not be read, such as a folder
    /// without the permission to read it.
    #[error("can not read {}: {detail}", .path.display())]
    Unreadable {
        /// The folder or file.
        path: PathBuf,
        /// The system's own reason.
        detail: String,
    },
    /// The program the source started, such as `codex app-server`, ended
    /// before it answered, such as a program that crashed. It names the
    /// program.
    #[error("stopped unexpectedly: {0} ended before it answered")]
    ProgramStopped(&'static str),
    /// The agent changed the format of its files: the newest of them hold
    /// records of the kind the source reads that it does not understand
    /// (spec 2026-11-19, Decision 9).
    #[error(
        "the format of the session files changed: {} of {} records understood",
        .0.understood(),
        .0.seen()
    )]
    FormatChanged(Box<FormatChange>),
}

/// Why a source can not be used here.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unavailable {
    /// The agent reads no statistics, or OTTY does not know it.
    #[error("{} reads no statistics", .0.as_str())]
    NoStatistics(AgentId),
    /// The agent of the profile has no bill to read.
    #[error("the agent has no bill")]
    NoBill,
    /// The profile names no admin key.
    #[error("no admin key")]
    NoAdminKey,
    /// The admin key is not where the profile says: its file is missing
    /// or its variable is not set.
    #[error("{}", admin_key_missing_text(.0))]
    AdminKeyMissing(AdminKeyRef),
    /// The file or the variable of the admin key is empty.
    #[error("{}", admin_key_empty_text(.0))]
    AdminKeyEmpty(AdminKeyRef),
    /// The name of the variable of the admin key is not a variable name.
    /// The name is not kept: it may be the key typed in the wrong field.
    #[error("the name of the admin key variable is not valid")]
    AdminKeyVariableNotValid,
    /// No copy of the price table was kept yet.
    #[error("no kept price file")]
    NoKeptPriceFile,
    /// No session file has the session.
    #[error("no session file of {}", .0.as_str())]
    NoSessionFile(SessionId),
    /// The agent wrote no session file yet.
    #[error("no session files")]
    NoSessionFiles,
    /// None of the newest session files, as many as the number here, has
    /// the limits.
    #[error("no limits in the {0} newest session files")]
    NoLimitsInSessionFiles(usize),
    /// A folder the source reads is a file.
    #[error("{} is not a folder", .0.display())]
    NotAFolder(PathBuf),
    /// Claude Code's `.credentials.json` has no login token.
    #[error("no login token in .credentials.json")]
    NoLoginToken,
    /// The login token is not allowed to read the usage.
    #[error("the login token may not read the usage")]
    LoginMayNotReadUsage,
    /// The agent is logged in with an API key, which has no usage limits.
    #[error("a login with an API key has no usage limits")]
    ApiKeyLogin,
    /// The program, such as `codex`, could not be started.
    #[error("can not start {program}: {reason}")]
    NotStarted {
        /// The program.
        program: &'static str,
        /// Why.
        reason: ProgramError,
    },
    /// What the source sends could not be written to the program.
    #[error("can not write to {program}: {reason}")]
    NotWritten {
        /// The program.
        program: &'static str,
        /// Why.
        reason: ProgramError,
    },
    /// The program did not answer in time.
    #[error("{program} did not answer in {seconds} seconds")]
    NoAnswerInTime {
        /// The program.
        program: &'static str,
        /// The time it had, in seconds.
        seconds: u64,
    },
    /// The program answered with an error.
    #[error("{program} refused: {}", .reason.as_deref().unwrap_or("no reason given"))]
    Refused {
        /// The program.
        program: &'static str,
        /// The program's own reason, when it gave one.
        reason: Option<String>,
    },
    /// The program, named here, is not found.
    #[error("{0} is not found")]
    ProgramNotFound(&'static str),
    /// Claude Code asks a question OTTY must not answer for the user.
    #[error("claude asks {0}")]
    ClaudeAsks(ClaudeQuestion),
    /// Claude Code says the usage was asked too often.
    #[error("claude says the usage was asked too often")]
    ClaudeRateLimited,
    /// The read stopped before it gave an answer. No source of this crate
    /// gives it: the app gives it for a read whose thread ended before it
    /// sent the answer back.
    #[error("the read stopped")]
    ReadStopped,
}

/// A question Claude Code asks that OTTY must not answer for the user.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClaudeQuestion {
    /// Whether to use the API key it found.
    #[error("whether to use an API key")]
    UseApiKey,
    /// To choose its look, on its first run.
    #[error("to choose its look")]
    ChooseLook,
    /// To press Enter.
    #[error("to press Enter")]
    PressEnter,
    /// Whether to trust a folder that is not the probe's own, named here
    /// as the screen shows it.
    #[error("whether to trust another folder: {0}")]
    TrustOtherFolder(String),
    /// Whether to trust its own folder, again after OTTY answered that
    /// question.
    #[error("again whether to trust its folder")]
    TrustOwnFolderAgain,
}

/// What an answer has that the code does not expect.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unexpected {
    /// The server answered with an HTTP status other than success; the
    /// number is the status.
    #[error("the server answered {0}")]
    ServerStatus(u16),
    /// The answer is not JSON.
    #[error("the answer is not JSON")]
    NotJson,
    /// The answer, or the file read, is JSON but not a JSON object.
    #[error("the answer is not a JSON object")]
    NotJsonObject,
    /// The answer has no limits: no usage window, no extra usage and no
    /// credits.
    #[error("the answer has no limits")]
    NoLimits,
    /// The answer is not a cost report of the Admin API.
    #[error("the answer is not a cost report")]
    NotCostReport,
    /// The cost report has more pages than the source reads.
    #[error("the cost report has too many pages")]
    TooManyPages,
    /// The price table has no price.
    #[error("the price file has no prices")]
    NoPrices,
    /// Claude Code's `/usage` panel shows no percent used or left.
    #[error("the usage panel shows no percent used or left")]
    UsagePanelUnreadable,
}

/// The English text of an admin key that is not where `reference` says,
/// such as "the admin key file ~/.config/key is missing" or
/// "ANTHROPIC_ADMIN_KEY is not set".
fn admin_key_missing_text(reference: &AdminKeyRef) -> String {
    match reference {
        AdminKeyRef::File(path) => {
            format!("the admin key file {} is missing", path.display())
        },
        AdminKeyRef::EnvVar(name) => format!("{name} is not set"),
    }
}

/// The English text of an admin key whose file or variable `reference`
/// is empty, such as "ANTHROPIC_ADMIN_KEY is empty".
fn admin_key_empty_text(reference: &AdminKeyRef) -> String {
    match reference {
        AdminKeyRef::File(path) => {
            format!("the admin key file {} is empty", path.display())
        },
        AdminKeyRef::EnvVar(name) => format!("{name} is empty"),
    }
}
