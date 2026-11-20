//! What tells OpenCode and Kilo apart: their names, the folder of their
//! data and the name of their database, and the newest version OTTY was
//! checked against. Everything else they share.

use std::path::{Path, PathBuf};

use crate::agent::{AgentEntry, AgentId, ConfigFolder};
use crate::built_in;
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::profile::Profile;
use crate::record_format::RecordFormat;
use crate::registry::Reader;

/// The variable that moves the data folder of both, and of other programs.
const XDG_DATA_HOME: &str = "XDG_DATA_HOME";

/// The data folder in the home folder when `XDG_DATA_HOME` is not set.
const DEFAULT_DATA_HOME: &str = ".local/share";

/// The value of a database variable that keeps the database in memory
/// only: there is no file to read then.
const IN_MEMORY: &str = ":memory:";

/// OpenCode, or Kilo, a fork of it that keeps the same files under other
/// names.
#[derive(Debug)]
pub(super) struct Fork {
    id: &'static str,
    name: &'static str,
    program: &'static str,
    provider_name: &'static str,
    icon: &'static [u8],
    config_folder: ConfigFolder,
    data_dir_name: &'static str,
    database_file: &'static str,
    format: RecordFormat,
}

impl Fork {
    /// The id the agent is saved under, such as `kilo`.
    pub(super) fn id(&self) -> AgentId {
        AgentId::new(self.id)
    }

    /// Where the agent keeps its data folder.
    pub(super) fn config_folder(&self) -> ConfigFolder {
        self.config_folder
    }

    /// The entry the agent puts in the registry, built in.
    pub(super) fn entry(&self) -> AgentEntry {
        AgentEntry::new(
            self.id(),
            self.name,
            self.program,
            self.provider_name,
            Reader::BuiltIn,
        )
        .with_icon(self.icon)
    }

    /// How the answers in its database are read, and the newest version
    /// of the agent they were checked against.
    pub(super) fn format(&self) -> RecordFormat {
        self.format
    }

    /// The data folder of the default profile: the agent's folder in
    /// `XDG_DATA_HOME` when the app found it set, else in
    /// `~/.local/share`.
    pub(super) fn default_data_dir(&self, env: &AgentEnv) -> PathBuf {
        let data_home = match env.config_dir_of(XDG_DATA_HOME) {
            Some(folder) => folder.to_path_buf(),
            None => env.home().join(DEFAULT_DATA_HOME),
        };

        data_home.join(self.data_dir_name)
    }

    /// The database the database variable names in `env` for the default
    /// profile in `data_dir`, as the agent finds it: a relative path is in
    /// the data folder. `None` when the variable is not set or keeps the
    /// database in memory.
    pub(super) fn moved_database(
        &self,
        env: &AgentEnv,
        data_dir: &Path,
    ) -> Option<PathBuf> {
        let variable = self.config_folder.database_variable()?;
        let database = env.config_dir_of(variable)?;
        if database == Path::new(IN_MEMORY) {
            return None;
        }

        Some(data_dir.join(database))
    }

    /// The database in the data folder `data_dir`.
    pub(super) fn database_in(&self, data_dir: &Path) -> PathBuf {
        data_dir.join(self.database_file)
    }

    /// The agent's program for `profile`. A profile whose data folder is
    /// not the default one points the program at it: with
    /// `XDG_DATA_HOME` set to the folder above it when the folder has the
    /// agent's own name, such as `~/work/kilo`; else, as the agent can not
    /// be pointed at a data folder of another name, with the database
    /// variable set to the database in it, so only the sessions are kept
    /// there.
    pub(super) fn start_command(&self, profile: &Profile) -> AgentCommand {
        let command = AgentCommand::new(self.program);
        if profile.is_in_default_config_dir() {
            return command;
        }

        let folder = profile.config_dir();
        let has_own_name = folder
            .file_name()
            .is_some_and(|name| name == self.data_dir_name);
        if has_own_name && let Some(parent) = folder.parent() {
            return command.with_env(XDG_DATA_HOME, parent.to_string_lossy());
        }

        match self.config_folder.database_variable() {
            Some(variable) => command
                .with_env(variable, self.database_in(folder).to_string_lossy()),
            None => command,
        }
    }
}

/// Kilo, the fork of OpenCode by Kilo Code: `~/.local/share/kilo`,
/// `kilo.db` and `KILO_DB`.
pub(super) const KILO: Fork = Fork {
    id: "kilo",
    name: "Kilo",
    program: "kilo",
    provider_name: "Kilo",
    icon: built_in::KILO_ICON,
    config_folder: ConfigFolder::new(".local/share/kilo", XDG_DATA_HOME)
        .with_database_variable("KILO_DB"),
    data_dir_name: "kilo",
    database_file: "kilo.db",
    format: RecordFormat::new(
        "database kilo.db (tables message and session_message)",
        "7.8.27",
    ),
};

/// OpenCode by Anomaly: `~/.local/share/opencode`, `opencode.db` and
/// `OPENCODE_DB`.
pub(super) const OPENCODE: Fork = Fork {
    id: "opencode",
    name: "OpenCode",
    program: "opencode",
    provider_name: "OpenCode",
    icon: built_in::OPENCODE_ICON,
    config_folder: ConfigFolder::new(".local/share/opencode", XDG_DATA_HOME)
        .with_database_variable("OPENCODE_DB"),
    data_dir_name: "opencode",
    database_file: "opencode.db",
    format: RecordFormat::new(
        "database opencode.db (tables message and session_message)",
        "2.3.2",
    ),
};

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> AgentEnv {
        AgentEnv::new(PathBuf::from("/home/user"))
    }

    fn kilo_profile(folder: &str) -> Profile {
        Profile::added(
            AgentId::new("kilo"),
            "Kilo · Work",
            PathBuf::from(folder),
        )
    }

    #[test]
    fn the_data_folder_is_in_the_xdg_data_folder() {
        let moved =
            env().with_config_dir(XDG_DATA_HOME, PathBuf::from("/data"));

        assert_eq!(
            KILO.default_data_dir(&env()),
            PathBuf::from("/home/user/.local/share/kilo")
        );
        assert_eq!(
            OPENCODE.default_data_dir(&moved),
            PathBuf::from("/data/opencode")
        );
    }

    #[test]
    fn the_database_variable_moves_the_database_of_the_default_profile() {
        let data_dir = Path::new("/home/user/.local/share/kilo");
        let absolute =
            env().with_config_dir("KILO_DB", PathBuf::from("/x/k.db"));
        let relative = env().with_config_dir("KILO_DB", PathBuf::from("k.db"));
        let in_memory =
            env().with_config_dir("KILO_DB", PathBuf::from(IN_MEMORY));
        let other =
            env().with_config_dir("OPENCODE_DB", PathBuf::from("/o.db"));

        assert_eq!(
            KILO.moved_database(&absolute, data_dir),
            Some(PathBuf::from("/x/k.db"))
        );
        assert_eq!(
            KILO.moved_database(&relative, data_dir),
            Some(data_dir.join("k.db"))
        );
        assert_eq!(KILO.moved_database(&in_memory, data_dir), None);
        assert_eq!(KILO.moved_database(&other, data_dir), None);
        assert_eq!(KILO.database_in(data_dir), data_dir.join("kilo.db"));
    }

    #[test]
    fn a_profile_in_another_folder_points_the_program_at_it() {
        let default = Profile::default_for(
            AgentId::new("kilo"),
            "Kilo",
            PathBuf::from("/home/user/.local/share/kilo"),
        );
        let own_name = kilo_profile("/home/user/work/kilo");
        let other_name = kilo_profile("/home/user/.local/share/kilo-work");

        assert_eq!(KILO.start_command(&default), AgentCommand::new("kilo"));
        assert_eq!(
            KILO.start_command(&own_name),
            AgentCommand::new("kilo")
                .with_env(XDG_DATA_HOME, "/home/user/work")
        );
        assert_eq!(
            KILO.start_command(&other_name),
            AgentCommand::new("kilo").with_env(
                "KILO_DB",
                "/home/user/.local/share/kilo-work/kilo.db"
            )
        );
    }
}
