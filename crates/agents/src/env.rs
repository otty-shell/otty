//! What the app read from the environment for the agents.

use std::path::{Path, PathBuf};

/// The parts of the environment the agents look at.
///
/// The crate reads no environment itself: the app reads it once and passes
/// it in, so tests never see the real home folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEnv {
    home: PathBuf,
    claude_config_dir: Option<PathBuf>,
    codex_home: Option<PathBuf>,
}

impl AgentEnv {
    /// The user's home folder.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The value of `CLAUDE_CONFIG_DIR`, when it is set.
    pub fn claude_config_dir(&self) -> Option<&Path> {
        self.claude_config_dir.as_deref()
    }

    /// The value of `CODEX_HOME`, when it is set.
    pub fn codex_home(&self) -> Option<&Path> {
        self.codex_home.as_deref()
    }

    /// The environment from what the app read.
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    ///
    /// use otty_agents::AgentEnv;
    ///
    /// let env = AgentEnv::new(PathBuf::from("/home/user"), None, None);
    ///
    /// assert_eq!(env.home(), Path::new("/home/user"));
    /// assert_eq!(env.codex_home(), None);
    /// ```
    pub fn new(
        home: PathBuf,
        claude_config_dir: Option<PathBuf>,
        codex_home: Option<PathBuf>,
    ) -> Self {
        Self {
            home,
            claude_config_dir,
            codex_home,
        }
    }
}
