//! Why a source gave no answer.
//!
//! The reasons are data, not sentences: the app writes them in the user's
//! language. The text of `Display` is the English one, for the log. Only
//! text that comes from outside, such as the system's reason a file could
//! not be read or a server's own message, is kept as it came.

use std::path::PathBuf;

use crate::agent::AgentId;
use crate::program::ProgramError;
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
    LoginExpired(String),
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
    ProgramStopped(String),
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
    /// The file of the admin key is missing.
    #[error("the admin key file {} is missing", .0.display())]
    AdminKeyFileMissing(PathBuf),
    /// The file of the admin key can not be read.
    #[error("the admin key file {} can not be read", .0.display())]
    AdminKeyFileUnreadable(PathBuf),
    /// The file of the admin key is empty.
    #[error("the admin key file {} is empty", .0.display())]
    AdminKeyFileEmpty(PathBuf),
    /// The name of the variable of the admin key is not a variable name.
    /// The name is not kept: it may be the key typed in the wrong field.
    #[error("the name of the admin key variable is not valid")]
    AdminKeyVariableNotValid,
    /// The variable of the admin key, named here, is not set.
    #[error("{0} is not set")]
    AdminKeyVariableNotSet(String),
    /// The variable of the admin key, named here, is empty.
    #[error("{0} is empty")]
    AdminKeyVariableEmpty(String),
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
        program: String,
        /// Why.
        reason: ProgramError,
    },
    /// What the source sends could not be written to the program.
    #[error("can not write to {program}: {reason}")]
    NotWritten {
        /// The program.
        program: String,
        /// Why.
        reason: ProgramError,
    },
    /// The program did not answer in time.
    #[error("{program} did not answer in {seconds} seconds")]
    NoAnswerInTime {
        /// The program.
        program: String,
        /// The time it had, in seconds.
        seconds: u64,
    },
    /// The program answered with an error.
    #[error("{program} refused: {}", .reason.as_deref().unwrap_or("no reason given"))]
    Refused {
        /// The program.
        program: String,
        /// The program's own reason, when it gave one.
        reason: Option<String>,
    },
    /// The program, named here, is not found.
    #[error("{0} is not found")]
    ProgramNotFound(String),
    /// Claude Code asks a question OTTY must not answer for the user.
    #[error("claude asks {0}")]
    ClaudeAsks(ClaudeQuestion),
    /// Claude Code still asks whether to trust its folder after OTTY
    /// answered the question.
    #[error("claude still asks whether to trust its folder")]
    ClaudeStillAsksTrust,
    /// Claude Code says the usage was asked too often.
    #[error("claude says the usage was asked too often")]
    ClaudeRateLimited,
    /// The read stopped before it gave an answer.
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
    /// The answer is JSON, but not a JSON object.
    #[error("the answer is not a JSON object")]
    NotJsonObject,
    /// Claude's usage answer has no usage window and no extra usage.
    #[error("no usage window in the answer")]
    NoUsageWindow,
    /// Codex's answer has no usage window and no credits.
    #[error("no usage window and no credits in the answer")]
    NoUsageWindowNorCredits,
    /// The answer of `codex app-server` has no `rateLimits`.
    #[error("the answer has no rateLimits")]
    NoRateLimits,
    /// The answer is not a cost report of the Admin API.
    #[error("the answer is not a cost report")]
    NotCostReport,
    /// The cost report has more pages than the source reads.
    #[error("the cost report has too many pages")]
    TooManyPages,
    /// The price table is not a JSON object.
    #[error("the price file is not a JSON object")]
    PriceFileNotJsonObject,
    /// The price table has no price.
    #[error("the price file has no prices")]
    NoPrices,
    /// Claude Code's `/usage` panel shows no percent used or left.
    #[error("the usage panel shows no percent used or left")]
    UsagePanelUnreadable,
}
