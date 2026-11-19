//! Where the database of a profile is: in its data folder, unless the
//! database variable moves the one of the default profile.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use super::fork::Fork;
use crate::env::AgentEnv;
use crate::profile::Profile;

/// The database of the default profile that the database variable moved,
/// such as `KILO_DB=/data/kilo.db`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MovedDatabase {
    data_dir: PathBuf,
    database: PathBuf,
}

/// Finds the database of a profile of one fork.
///
/// The app tells an agent its environment only when it asks for the
/// folder of the default profile, so the database variable is kept from
/// there for the reads that follow.
#[derive(Debug)]
pub(super) struct DatabaseLocation {
    fork: &'static Fork,
    moved: Mutex<Option<MovedDatabase>>,
}

impl DatabaseLocation {
    /// The fork whose databases these are.
    pub(super) fn fork(&self) -> &'static Fork {
        self.fork
    }

    /// The location of the databases of `fork`, none moved yet.
    pub(super) fn new(fork: &'static Fork) -> Self {
        Self {
            fork,
            moved: Mutex::new(None),
        }
    }

    /// The data folder of the default profile in `env`, keeping the
    /// database its database variable names, if any.
    pub(super) fn default_data_dir(&self, env: &AgentEnv) -> PathBuf {
        let data_dir = self.fork.default_data_dir(env);
        let moved = self.fork.moved_database(env, &data_dir).map(|database| {
            MovedDatabase {
                data_dir: data_dir.clone(),
                database,
            }
        });

        *self.moved.lock().unwrap_or_else(PoisonError::into_inner) = moved;

        data_dir
    }

    /// The database of `profile`: the moved one for the default data
    /// folder, else the one in its data folder.
    pub(super) fn database_of(&self, profile: &Profile) -> PathBuf {
        self.database_in(profile.config_dir())
    }

    /// The database of the data folder `data_dir`.
    fn database_in(&self, data_dir: &Path) -> PathBuf {
        let moved = self.moved.lock().unwrap_or_else(PoisonError::into_inner);

        match moved.as_ref() {
            Some(moved) if moved.data_dir == data_dir => moved.database.clone(),
            _ => self.fork.database_in(data_dir),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::open_code::fork::KILO;

    fn profile(folder: &str) -> Profile {
        Profile::added(AgentId::new("kilo"), "Kilo", PathBuf::from(folder))
    }

    #[test]
    fn the_database_variable_moves_only_the_database_of_the_default_folder() {
        let location = DatabaseLocation::new(&KILO);
        let env = AgentEnv::new(PathBuf::from("/home/user"))
            .with_config_dir("KILO_DB", PathBuf::from("/data/kilo.db"));

        let data_dir = location.default_data_dir(&env);

        assert_eq!(data_dir, PathBuf::from("/home/user/.local/share/kilo"));
        assert_eq!(
            location.database_of(&profile("/home/user/.local/share/kilo")),
            PathBuf::from("/data/kilo.db")
        );
        assert_eq!(
            location.database_of(&profile("/home/user/work/kilo")),
            PathBuf::from("/home/user/work/kilo/kilo.db")
        );
    }

    #[test]
    fn with_no_variable_the_database_is_in_the_data_folder() {
        let location = DatabaseLocation::new(&KILO);

        location.default_data_dir(&AgentEnv::new(PathBuf::from("/home/user")));

        assert_eq!(
            location.database_of(&profile("/home/user/.local/share/kilo")),
            PathBuf::from("/home/user/.local/share/kilo/kilo.db")
        );
    }
}
