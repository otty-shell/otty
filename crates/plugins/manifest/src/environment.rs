//! What the app read from the environment, for the paths of permissions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The home folder and the environment variables that permission globs
/// such as `{XDG_DATA_HOME}/opencode/**` may start with.
///
/// The crate reads no environment itself: the app reads it once and
/// passes it in, so tests never see the real home folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    home: PathBuf,
    vars: BTreeMap<String, String>,
}

impl Environment {
    /// The user's home folder, what `~` stands for.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The value of the variable `name`, if it is set.
    pub fn var(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    /// The environment with the home folder `home` and no variable set.
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    ///
    /// use otty_plugin_manifest::Environment;
    ///
    /// let env = Environment::new(PathBuf::from("/home/user"))
    ///     .with_var("CODEX_HOME", "/data/codex");
    ///
    /// assert_eq!(env.home(), Path::new("/home/user"));
    /// assert_eq!(env.var("CODEX_HOME"), Some("/data/codex"));
    /// assert_eq!(env.var("XDG_DATA_HOME"), None);
    /// ```
    pub fn new(home: PathBuf) -> Self {
        Self {
            home,
            vars: BTreeMap::new(),
        }
    }

    /// The same environment with the variable `name` set to `value`.
    pub fn with_var(mut self, name: &str, value: &str) -> Self {
        self.vars.insert(name.to_string(), value.to_string());

        self
    }
}
