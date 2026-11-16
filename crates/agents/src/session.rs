//! Sessions: conversations with an agent that it can continue.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::SourceError;
use crate::profile::Profile;

/// The id an agent gives a session; the agent continues a session by it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A session id from its text.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// One conversation with an agent, as its session list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    id: SessionId,
    folder: PathBuf,
    title: String,
    changed_at: SystemTime,
    branch: Option<String>,
}

impl AgentSession {
    /// The id the agent continues the session by.
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// The project folder the session ran in.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The title shown to the user.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// When the session last changed.
    pub fn changed_at(&self) -> SystemTime {
        self.changed_at
    }

    /// The git branch the session ran on, when known.
    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    /// A session with no known git branch.
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use std::time::SystemTime;
    ///
    /// use otty_agents::{AgentSession, SessionId};
    ///
    /// let session = AgentSession::new(
    ///     SessionId::new("a1"),
    ///     PathBuf::from("/home/user/otty"),
    ///     "Fix the tabs",
    ///     SystemTime::UNIX_EPOCH,
    /// )
    /// .with_branch("main");
    ///
    /// assert_eq!(session.branch(), Some("main"));
    /// ```
    pub fn new(
        id: SessionId,
        folder: PathBuf,
        title: impl Into<String>,
        changed_at: SystemTime,
    ) -> Self {
        Self {
            id,
            folder,
            title: title.into(),
            changed_at,
            branch: None,
        }
    }

    /// The same session with the git branch it ran on.
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }
}

/// One way to list the sessions of a profile, such as "read the session
/// files".
///
/// It is a trait because each agent keeps its sessions its own way, and
/// reading them crosses into files or programs.
#[cfg_attr(test, mockall::automock)]
pub trait SessionSource: Send {
    /// A short name of the source, shown when it answered or failed.
    fn name(&self) -> &'static str;

    /// The sessions of the profile, in any order.
    fn sessions(
        &self,
        profile: &Profile,
    ) -> Result<Vec<AgentSession>, SourceError>;
}
