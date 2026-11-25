//! Where a path really leads. A part reaches a file through a symlink
//! only when the real file lies inside the part's globs too, so a symlink
//! in an agent's folder can not hand a plugin `~/.ssh`.
//!
//! The real file is checked as if it were written from the home folder or
//! the variable folder it lies in, so a home folder that is itself a
//! symlink, such as `/home` linked to `/var/home`, keeps working.

use std::fs;
use std::path::{Path, PathBuf};

use otty_plugin_manifest::{Environment, Permissions};

use crate::operations::serve::io_error;
use crate::operations::v0_1::otty::host::types::HostError;

/// The home folder and the folders of the variables that are not where
/// they are written, each with the folder it really is.
pub(super) struct RealFolders {
    /// Pairs of the folder as written and the real folder.
    folders: Vec<(PathBuf, PathBuf)>,
}

impl RealFolders {
    /// The folders of `env` that lead elsewhere through a symlink.
    pub(super) fn new(env: &Environment) -> Self {
        let written = std::iter::once(env.home().to_path_buf())
            .chain(env.vars().map(|(_, value)| PathBuf::from(value)))
            .filter(|folder| folder.is_absolute());

        let folders = written
            .filter_map(|folder| {
                let real = fs::canonicalize(&folder).ok()?;

                (real != folder).then_some((folder, real))
            })
            .collect();

        Self { folders }
    }

    /// Whether `allows` lets the part reach the real file `real`, as it
    /// is or written from the home folder or variable folder it lies in.
    pub(super) fn allows(
        &self,
        permissions: &Permissions,
        real: &Path,
        allows: fn(&Permissions, &Path) -> bool,
    ) -> bool {
        if allows(permissions, real) {
            return true;
        }

        self.folders.iter().any(|(written, real_folder)| {
            real.strip_prefix(real_folder)
                .is_ok_and(|rest| allows(permissions, &written.join(rest)))
        })
    }
}

/// The real file behind `path`, every symlink resolved, when `allows`
/// lets the part reach both `path` as written and that real file.
pub(super) fn reachable_file(
    permissions: &Permissions,
    env: &Environment,
    path: &Path,
    allows: fn(&Permissions, &Path) -> bool,
) -> Result<PathBuf, HostError> {
    if !allows(permissions, path) {
        return Err(not_allowed(path));
    }

    let real =
        fs::canonicalize(path).map_err(|error| io_error(&error, path))?;
    if real == path {
        return Ok(real);
    }

    if !RealFolders::new(env).allows(permissions, &real, allows) {
        return Err(not_allowed(path));
    }

    Ok(real)
}

/// The refusal of `path`, which the called part did not declare.
pub(super) fn not_allowed(path: &Path) -> HostError {
    HostError::NotAllowed(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;
    use crate::testing::{TempDir, debug_err, files_permissions, write_file};

    fn may_read(permissions: &Permissions, path: &Path) -> bool {
        permissions.may_read(path)
    }

    fn reach(env: &Environment, path: &Path) -> Result<PathBuf, String> {
        let permissions = files_permissions("alpha", env);

        debug_err(reachable_file(&permissions, env, path, may_read))
    }

    fn refusal(path: &Path) -> Result<PathBuf, String> {
        debug_err(Err(not_allowed(path)))
    }

    #[test]
    fn a_file_inside_the_globs_is_reachable() {
        let home = TempDir::new();
        let env = Environment::new(home.path().to_path_buf());
        let file = home.path().join(".alpha/sessions/a.jsonl");
        write_file(&file, b"{}\n");

        assert_eq!(reach(&env, &file), Ok(file));
    }

    #[test]
    fn a_symlink_leaving_the_globs_is_refused() {
        let home = TempDir::new();
        let env = Environment::new(home.path().to_path_buf());
        let secret = home.path().join(".ssh/id_ed25519");
        let link = home.path().join(".alpha/key");
        write_file(&secret, b"secret");
        std::fs::create_dir_all(home.path().join(".alpha")).expect("folder");
        symlink(&secret, &link).expect("symlink");

        assert_eq!(reach(&env, &link), refusal(&link));
    }

    #[test]
    fn a_folder_symlink_leaving_the_globs_is_refused() {
        let home = TempDir::new();
        let env = Environment::new(home.path().to_path_buf());
        write_file(&home.path().join(".ssh/id_ed25519"), b"secret");
        std::fs::create_dir_all(home.path().join(".alpha")).expect("folder");
        symlink(home.path().join(".ssh"), home.path().join(".alpha/ssh"))
            .expect("symlink");
        let through = home.path().join(".alpha/ssh/id_ed25519");

        assert_eq!(reach(&env, &through), refusal(&through));
    }

    #[test]
    fn a_symlink_staying_inside_the_globs_is_reachable() {
        let home = TempDir::new();
        let env = Environment::new(home.path().to_path_buf());
        let file = home.path().join(".alpha/data/a.jsonl");
        let link = home.path().join(".alpha/latest.jsonl");
        write_file(&file, b"{}\n");
        symlink(&file, &link).expect("symlink");

        assert_eq!(reach(&env, &link), Ok(file));
    }

    #[test]
    fn a_home_folder_that_is_a_symlink_keeps_its_globs() {
        let root = TempDir::new();
        let real_home = root.path().join("var/home/user");
        let home = root.path().join("home-user");
        let file = real_home.join(".alpha/a.jsonl");
        write_file(&file, b"{}\n");
        symlink(&real_home, &home).expect("symlink");
        let env = Environment::new(home.clone());

        assert_eq!(reach(&env, &home.join(".alpha/a.jsonl")), Ok(file));
    }

    #[test]
    fn a_variable_folder_that_is_a_symlink_keeps_its_globs() {
        let root = TempDir::new();
        let real_folder = root.path().join("disk/alpha");
        let folder = root.path().join("alpha-link");
        let file = real_folder.join("a.jsonl");
        write_file(&file, b"{}\n");
        symlink(&real_folder, &folder).expect("symlink");
        let env = Environment::new(root.path().join("home"))
            .with_var("ALPHA_HOME", &folder.display().to_string());

        assert_eq!(reach(&env, &folder.join("a.jsonl")), Ok(file));
    }

    #[test]
    fn a_path_outside_the_globs_or_with_dot_dot_is_refused() {
        let home = TempDir::new();
        let env = Environment::new(home.path().to_path_buf());
        let outside = home.path().join("notes.txt");
        let up = home.path().join(".alpha/../notes.txt");
        write_file(&outside, b"notes");

        assert_eq!(reach(&env, &outside), refusal(&outside));
        assert_eq!(reach(&env, &up), refusal(&up));
    }

    #[test]
    fn a_missing_file_is_not_found() {
        let home = TempDir::new();
        let env = Environment::new(home.path().to_path_buf());
        let missing = home.path().join(".alpha/missing.jsonl");

        assert_eq!(
            reach(&env, &missing),
            debug_err(Err(HostError::NotFound(missing.display().to_string())))
        );
    }
}
