//! Why a source gave no answer.
//!
//! The reasons are data, not sentences: the app writes them in the user's
//! language. The text of `Display` is the English one, for the log. Only
//! text that comes from outside, such as the system's reason a file could
//! not be read or a server's own message, is kept as it came.

use std::path::PathBuf;

use otty_plugin_host::PluginFailure;

use crate::agent::AgentId;
use crate::given_key::KeyRef;
use crate::part_need::PartNeed;
use crate::record_format::FormatChange;
use crate::refused_login::RefusedLogin;
use crate::session::SessionId;

/// Why a source gave no answer. Any of these moves on to the next source
/// (spec Decision 5).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// The source can not be used here, for example its file or program is
    /// missing.
    #[error("not available: {0}")]
    NotAvailable(Unavailable),
    /// The plugin part can not read until the user does what its need
    /// asks, such as log in again or give a key the provider refused
    /// (spec 2026-11-19-agent-plugins, issue 20).
    #[error("{}", .0.title())]
    NeedsUser(Box<PartNeed>),
    /// A provider part refused the login it read with, or saw it expired,
    /// and no agent part said what the user must do about it (issue 20).
    #[error("{} refused the login", .0.provider().name())]
    LoginRefused(Box<RefusedLogin>),
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
    /// The agent changed the format of its files: the newest of them hold
    /// records of the kind the source reads that it does not understand
    /// (spec 2026-11-19, Decision 9).
    #[error(
        "the format of the session files changed: {} of {} records understood",
        .0.understood(),
        .0.seen()
    )]
    FormatChanged(Box<FormatChange>),
    /// A plugin part could not read: it trapped, ran past its deadline,
    /// is paused, or said it could not read (spec
    /// 2026-11-19-agent-plugins, "What the UI shows"). The last good
    /// numbers stay, greyed.
    #[error(
        "{} {} could not read: {}",
        .0.part().name(),
        .0.part().version(),
        .0.kind().text()
    )]
    PluginFailed(Box<PluginFailure>),
}

impl From<Box<PluginFailure>> for SourceError {
    fn from(failure: Box<PluginFailure>) -> Self {
        SourceError::PluginFailed(failure)
    }
}

/// Why a source can not be used here.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unavailable {
    /// The agent reads no statistics, or OTTY does not know it.
    #[error("{} reads no statistics", .0.as_str())]
    NoStatistics(AgentId),
    /// No plugin part reads a bill of the account.
    #[error("the agent has no bill")]
    NoBill,
    /// The profile holds no login the part could read with.
    #[error("no login")]
    NoLogin,
    /// A key the user gave, the key of an added account or one a part
    /// asked for, is not where the user said: its file is missing or its
    /// variable is not set.
    #[error("{}", key_missing_text(.0))]
    KeyMissing(KeyRef),
    /// The file or the variable of a key the user gave is empty.
    #[error("{}", key_empty_text(.0))]
    KeyEmpty(KeyRef),
    /// OTTY reads nothing of the account by itself: its numbers come from
    /// the profiles that use it.
    #[error("the account has no source of its own")]
    NoAccountSource,
    /// The key of the account is no longer where OTTY found it, such as a
    /// key taken out of the agent's file since.
    #[error("the key is no longer where OTTY found it")]
    KeyGone,
    /// The name of the variable of a key the user gave is not a variable
    /// name.
    /// The name is not kept: it may be the key typed in the wrong field.
    #[error("the name of the key variable is not valid")]
    KeyVariableNotValid,
    /// No copy of the price table was kept yet.
    #[error("no kept price file")]
    NoKeptPriceFile,
    /// No session file has the session.
    #[error("no session file of {}", .0.as_str())]
    NoSessionFile(SessionId),
    /// The agent wrote no session file yet.
    #[error("no session files")]
    NoSessionFiles,
    /// A folder the source reads is a file.
    #[error("{} is not a folder", .0.display())]
    NotAFolder(PathBuf),
    /// The program, named here, is not found.
    #[error("{0} is not found")]
    ProgramNotFound(&'static str),
    /// The read stopped before it gave an answer. No source of this crate
    /// gives it: the app gives it for a read whose thread ended before it
    /// sent the answer back.
    #[error("the read stopped")]
    ReadStopped,
}

/// What an answer has that the code does not expect.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unexpected {
    /// The server answered with an HTTP status other than success; the
    /// number is the status.
    #[error("the server answered {0}")]
    ServerStatus(u16),
    /// The answer, or the file read, is JSON but not a JSON object.
    #[error("the answer is not a JSON object")]
    NotJsonObject,
    /// The price table has no price.
    #[error("the price file has no prices")]
    NoPrices,
}

/// The English text of a key that is not where `reference` says, such as
/// "the key file ~/.config/key is missing" or
/// "EXAMPLE_BILL_KEY is not set".
fn key_missing_text(reference: &KeyRef) -> String {
    match reference {
        KeyRef::File(path) => {
            format!("the key file {} is missing", path.display())
        },
        KeyRef::EnvVar(name) => format!("{name} is not set"),
    }
}

/// The English text of a key whose file or variable `reference` is
/// empty, such as "EXAMPLE_BILL_KEY is empty".
fn key_empty_text(reference: &KeyRef) -> String {
    match reference {
        KeyRef::File(path) => {
            format!("the key file {} is empty", path.display())
        },
        KeyRef::EnvVar(name) => format!("{name} is empty"),
    }
}
