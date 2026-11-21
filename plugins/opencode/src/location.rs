//! Where OpenCode keeps its data: the data folder, which is the folder of
//! a profile, and the database in it, and how its program is pointed at a
//! profile whose folder is not the default one.

use otty_plugin_sdk::agents::{Profile, ProfilePlace};
use otty_plugin_sdk::{Command, env};

/// The variable that moves the data folder of OpenCode, and of other
/// programs.
pub(crate) const DATA_VARIABLE: &str = "XDG_DATA_HOME";

/// The variable that moves the database of OpenCode.
pub(crate) const DATABASE_VARIABLE: &str = "OPENCODE_DB";

/// The program, looked up on `PATH`.
const PROGRAM: &str = "opencode";

/// The data folder of all programs in the home folder when
/// `XDG_DATA_HOME` is not set.
const DEFAULT_DATA_HOME: &str = ".local/share";

/// The name of OpenCode's own folder in the data folder of all programs.
const FOLDER_NAME: &str = "opencode";

/// The database in the data folder.
const DATABASE_FILE: &str = "opencode.db";

/// The value of `OPENCODE_DB` that keeps the database in memory only:
/// there is no file to read then.
const IN_MEMORY: &str = ":memory:";

/// The data folders of OpenCode on this computer: only the default one.
/// OpenCode has no way to name another one but `XDG_DATA_HOME`, which
/// moves the default one.
pub(crate) fn profiles() -> Vec<ProfilePlace> {
    vec![ProfilePlace {
        folder: default_data_folder(),
        is_default: true,
    }]
}

/// The data folder of the default profile: OpenCode's folder in
/// `XDG_DATA_HOME` when it is set, else in `~/.local/share`.
pub(crate) fn default_data_folder() -> String {
    let data_home = filled_var(DATA_VARIABLE).unwrap_or_else(|| {
        format!("{}/{DEFAULT_DATA_HOME}", env::home().trim_end_matches('/'))
    });

    format!("{}/{FOLDER_NAME}", data_home.trim_end_matches('/'))
}

/// The database of `profile`, as OpenCode finds it: the one `OPENCODE_DB`
/// names for the default profile (a relative path is in the data folder),
/// else `opencode.db` in the data folder. `None` when `OPENCODE_DB` keeps
/// the database of the default profile in memory.
pub(crate) fn database_of(profile: &Profile) -> Option<String> {
    let folder = profile.folder.trim_end_matches('/');
    let moved = filled_var(DATABASE_VARIABLE).filter(|_| profile.is_default);

    match moved.as_deref() {
        Some(IN_MEMORY) => None,
        Some(database) if database.starts_with('/') => {
            Some(database.to_string())
        },
        Some(database) => Some(format!("{folder}/{database}")),
        None => Some(format!("{folder}/{DATABASE_FILE}")),
    }
}

/// OpenCode with `args` for `profile`. A profile whose data folder is not
/// the default one points the program at it: with `XDG_DATA_HOME` set to
/// the folder above it when the folder is named `opencode`, such as
/// `~/work/opencode`; else, as OpenCode can not be pointed at a data
/// folder of another name, with `OPENCODE_DB` set to the database in it,
/// so only the sessions are kept there.
pub(crate) fn start_command(profile: &Profile, args: Vec<String>) -> Command {
    let folder = profile.folder.trim_end_matches('/');

    let env = if profile.is_default {
        Vec::new()
    } else {
        match folder.rsplit_once('/') {
            Some((parent, FOLDER_NAME)) if !parent.is_empty() => {
                vec![(DATA_VARIABLE.to_string(), parent.to_string())]
            },
            _ => vec![(
                DATABASE_VARIABLE.to_string(),
                format!("{folder}/{DATABASE_FILE}"),
            )],
        }
    };

    Command {
        program: PROGRAM.to_string(),
        args,
        env,
    }
}

/// The variable `name` when it is set and not blank.
fn filled_var(name: &str) -> Option<String> {
    env::var(name).filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    fn profile(folder: &str, is_default: bool) -> Profile {
        Profile {
            agent: "opencode".to_string(),
            folder: folder.to_string(),
            is_default,
        }
    }

    fn command(args: &[&str], env: &[(&str, &str)]) -> Command {
        Command {
            program: PROGRAM.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            env: env
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        }
    }

    #[test]
    fn the_data_folder_is_in_the_xdg_data_folder() {
        let plain = host();
        let moved = host().with_var(DATA_VARIABLE, "/data");

        let default = as_part(&plain, profiles);
        let elsewhere = as_part(&moved, default_data_folder);

        assert_eq!(
            default,
            [ProfilePlace {
                folder: path(&plain, ".local/share/opencode"),
                is_default: true,
            }]
        );
        assert_eq!(elsewhere, "/data/opencode");
    }

    #[test]
    fn the_database_variable_moves_the_database_of_the_default_profile() {
        let folder = "/home/user/.local/share/opencode";
        let database_with = |value: &str, is_default: bool| {
            let host = host().with_var(DATABASE_VARIABLE, value);

            as_part(&host, || database_of(&profile(folder, is_default)))
        };

        assert_eq!(database_with("/x/o.db", true).as_deref(), Some("/x/o.db"));
        assert_eq!(
            database_with("o.db", true).as_deref(),
            Some("/home/user/.local/share/opencode/o.db")
        );
        assert_eq!(database_with(IN_MEMORY, true), None);
        assert_eq!(
            database_with("/x/o.db", false).as_deref(),
            Some("/home/user/.local/share/opencode/opencode.db"),
            "only the default profile's database is moved"
        );
        assert_eq!(
            as_part(&host(), || database_of(&profile(folder, true))).as_deref(),
            Some("/home/user/.local/share/opencode/opencode.db")
        );
    }

    #[test]
    fn a_profile_in_another_folder_points_the_program_at_it() {
        let default = profile("/home/user/.local/share/opencode", true);
        let own_name = profile("/home/user/work/opencode", false);
        let other_name =
            profile("/home/user/.local/share/opencode-work", false);

        assert_eq!(start_command(&default, Vec::new()), command(&[], &[]));
        assert_eq!(
            start_command(&own_name, vec!["-s".to_string()]),
            command(&["-s"], &[(DATA_VARIABLE, "/home/user/work")])
        );
        assert_eq!(
            start_command(&other_name, Vec::new()),
            command(
                &[],
                &[(
                    DATABASE_VARIABLE,
                    "/home/user/.local/share/opencode-work/opencode.db"
                )]
            )
        );
    }
}
