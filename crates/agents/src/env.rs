//! What the app read from the environment for the agents.

use std::path::{Path, PathBuf};

/// The parts of the environment the agents look at: the home folder and,
/// for each agent, the variable that points it at a config folder, such as
/// `CLAUDE_CONFIG_DIR` (see [`AgentDescriptor::config_dir_variable`]).
///
/// The crate reads no environment itself: the app reads it once and passes
/// it in, so tests never see the real home folder.
///
/// [`AgentDescriptor::config_dir_variable`]: crate::AgentDescriptor::config_dir_variable
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEnv {
    home: PathBuf,
    config_dirs: Vec<(String, PathBuf)>,
}

impl AgentEnv {
    /// The user's home folder.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The folder the config folder variable `variable` names, such as
    /// `CODEX_HOME`; `None` when it is not set.
    pub fn config_dir_of(&self, variable: &str) -> Option<&Path> {
        self.config_dirs
            .iter()
            .find(|(name, _)| name == variable)
            .map(|(_, folder)| folder.as_path())
    }

    /// The environment with the home folder `home` and no config folder
    /// variable set.
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    ///
    /// use otty_agents::AgentEnv;
    ///
    /// let env = AgentEnv::new(PathBuf::from("/home/user"));
    ///
    /// assert_eq!(env.home(), Path::new("/home/user"));
    /// assert_eq!(env.config_dir_of("CODEX_HOME"), None);
    /// ```
    pub fn new(home: PathBuf) -> Self {
        Self {
            home,
            config_dirs: Vec::new(),
        }
    }

    /// The same environment with the config folder variable `variable`
    /// set to `folder`; a variable set again keeps its last folder.
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    ///
    /// use otty_agents::AgentEnv;
    ///
    /// let env = AgentEnv::new(PathBuf::from("/home/user"))
    ///     .with_config_dir("CODEX_HOME", PathBuf::from("/data/codex"));
    ///
    /// assert_eq!(
    ///     env.config_dir_of("CODEX_HOME"),
    ///     Some(Path::new("/data/codex"))
    /// );
    /// ```
    pub fn with_config_dir(mut self, variable: &str, folder: PathBuf) -> Self {
        self.config_dirs.retain(|(name, _)| name != variable);
        self.config_dirs.push((variable.to_string(), folder));

        self
    }
}
