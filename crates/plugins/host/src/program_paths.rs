//! Where OTTY found the programs of a plugin's agent parts (issue 32): a
//! program the user's login shell does not find by its name, but that is
//! at one of the program places its part's manifest names, is started by
//! that full path. The plugin asks for it by its name, or by that path in
//! the command of a found profile, and the permissions of the name apply.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The programs of one plugin that are started by a full path, by name.
#[derive(Debug, Default)]
pub(crate) struct ProgramPaths {
    found: Mutex<BTreeMap<String, PathBuf>>,
}

impl ProgramPaths {
    /// The full path the program `name` is started by, if OTTY found it
    /// at a program place.
    pub(crate) fn path_of(&self, name: &str) -> Option<PathBuf> {
        self.found().get(name).cloned()
    }

    /// The program `name` is started by `path` from now on; `None`: by
    /// its name again.
    pub(crate) fn set(&self, name: &str, path: Option<PathBuf>) {
        let mut found = self.found();

        match path {
            Some(path) => found.insert(name.to_string(), path),
            None => found.remove(name),
        };
    }

    /// The name of the program a plugin asks for as `asked`: the name
    /// whose found path `asked` is, else `asked` as it is.
    pub(crate) fn name_of(&self, asked: &str) -> String {
        self.found()
            .iter()
            .find(|(_, path)| path.as_os_str() == asked)
            .map_or(asked, |(name, _)| name.as_str())
            .to_string()
    }

    /// How the program `name` is started: by the full path it was found
    /// at, else by its name.
    pub(crate) fn to_start(&self, name: &str) -> String {
        self.path_of(name)
            .map_or_else(|| name.to_string(), |path| path.display().to_string())
    }

    fn found(&self) -> MutexGuard<'_, BTreeMap<String, PathBuf>> {
        self.found.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOUND: &str = "/home/user/.opencode/bin/opencode";

    #[test]
    fn a_found_program_is_started_by_its_path_and_known_by_it() {
        let paths = ProgramPaths::default();
        paths.set("opencode", Some(PathBuf::from(FOUND)));

        assert_eq!(paths.to_start("opencode"), FOUND);
        assert_eq!(paths.name_of(FOUND), "opencode");
        assert_eq!(paths.name_of("opencode"), "opencode");
        assert_eq!(paths.to_start("codex"), "codex");
        assert_eq!(paths.name_of("/tmp/opencode"), "/tmp/opencode");
    }

    #[test]
    fn a_program_the_shell_finds_again_is_started_by_its_name() {
        let paths = ProgramPaths::default();
        paths.set("opencode", Some(PathBuf::from(FOUND)));

        paths.set("opencode", None);

        assert_eq!(paths.to_start("opencode"), "opencode");
        assert_eq!(paths.name_of(FOUND), FOUND);
    }
}
