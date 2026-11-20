//! `files.read`: the bytes of a small file the part may read.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use otty_plugin_manifest::{Environment, Permissions};

use crate::operations::real_path::reachable_file;
use crate::operations::serve::io_error;
use crate::operations::v0_1::otty::host::types::HostError;

/// The most bytes `files.read` gives (spec "Host operations").
pub(super) const READ_CAP: u32 = 4 * 1024 * 1024;

/// The bytes of the file `path`, at most `max_bytes` and [`READ_CAP`]. A
/// key file is refused: it is read only with its keys replaced.
pub(super) fn read_small_file(
    permissions: &Permissions,
    env: &Environment,
    path: &str,
    max_bytes: u32,
) -> Result<Vec<u8>, HostError> {
    let path = Path::new(path);
    let real = reachable_file(permissions, env, path, Permissions::may_read)?;

    let limit = max_bytes.min(READ_CAP);
    let mut bytes = Vec::new();
    File::open(&real)
        .and_then(|file| {
            file.take(u64::from(limit) + 1).read_to_end(&mut bytes)
        })
        .map_err(|error| io_error(&error, path))?;

    if bytes.len() > limit as usize {
        return Err(HostError::TooBig(format!(
            "{} is more than {limit} bytes",
            path.display()
        )));
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::operations::real_path::not_allowed;
    use crate::testing::{TempDir, debug_err, files_permissions, write_file};

    struct Home {
        dir: TempDir,
        env: Environment,
    }

    impl Home {
        fn path(&self, relative: &str) -> PathBuf {
            self.dir.path().join(relative)
        }

        fn new() -> Self {
            let dir = TempDir::new();
            let env = Environment::new(dir.path().to_path_buf());

            Self { dir, env }
        }

        fn read(
            &self,
            part: &str,
            relative: &str,
            max: u32,
        ) -> Result<Vec<u8>, String> {
            let permissions = files_permissions(part, &self.env);
            let path = self.path(relative).display().to_string();

            debug_err(read_small_file(&permissions, &self.env, &path, max))
        }

        fn refusal(&self, relative: &str) -> Result<Vec<u8>, String> {
            debug_err(Err(not_allowed(&self.path(relative))))
        }
    }

    #[test]
    fn a_file_inside_the_globs_is_read() {
        let home = Home::new();
        write_file(&home.path(".alpha/state.json"), b"{\"v\":1}");

        assert_eq!(
            home.read("alpha", ".alpha/state.json", 1024),
            Ok(b"{\"v\":1}".to_vec())
        );
    }

    #[test]
    fn a_file_outside_the_globs_is_refused() {
        let home = Home::new();
        write_file(&home.path(".ssh/id_ed25519"), b"secret");

        assert_eq!(
            home.read("alpha", ".ssh/id_ed25519", 1024),
            home.refusal(".ssh/id_ed25519")
        );
        assert_eq!(
            home.read("alpha", ".alpha/../.ssh/id_ed25519", 1024),
            home.refusal(".alpha/../.ssh/id_ed25519")
        );
    }

    #[test]
    fn a_key_file_is_refused() {
        let home = Home::new();
        write_file(&home.path(".alpha/auth.json"), b"{\"key\":\"sk-1\"}");

        assert_eq!(
            home.read("alpha", ".alpha/auth.json", 1024),
            home.refusal(".alpha/auth.json")
        );
    }

    #[test]
    fn a_file_of_another_part_of_the_same_plugin_is_refused() {
        let home = Home::new();
        write_file(&home.path(".beta/state.json"), b"{}");

        assert_eq!(
            home.read("alpha", ".beta/state.json", 1024),
            home.refusal(".beta/state.json")
        );
        assert_eq!(
            home.read("beta", ".beta/state.json", 1024),
            Ok(b"{}".to_vec())
        );
    }

    #[test]
    fn a_file_larger_than_asked_is_too_big() {
        let home = Home::new();
        write_file(&home.path(".alpha/big.log"), &[b'a'; 11]);

        let read = home.read("alpha", ".alpha/big.log", 10);

        assert!(
            read.as_ref()
                .is_err_and(|error| error.starts_with("HostError::TooBig(")),
            "{read:?}"
        );
    }
}
