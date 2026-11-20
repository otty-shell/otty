//! `env.var`: an environment variable the part lists, as text.

use otty_plugin_manifest::{Environment, Permissions};

/// The variable `name` as text; none when it is not set, the part does
/// not list it, or it holds a key (read only through a key handle).
pub(super) fn variable(
    permissions: &Permissions,
    env: &Environment,
    name: &str,
) -> Option<String> {
    if !permissions.may_read_var(name) {
        return None;
    }

    env.var(name).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::testing::files_permissions;

    fn env() -> Environment {
        Environment::new(PathBuf::from("/home/user"))
            .with_var("ALPHA_HOME", "/data/alpha")
            .with_var("ALPHA_TOKEN", "sk-1")
            .with_var("BETA_HOME", "/data/beta")
            .with_var("PATH", "/usr/bin")
    }

    #[test]
    fn a_listed_variable_is_read() {
        let alpha = files_permissions("alpha", &env());

        assert_eq!(
            variable(&alpha, &env(), "ALPHA_HOME"),
            Some("/data/alpha".to_string())
        );
    }

    #[test]
    fn an_unlisted_or_unset_variable_reads_as_none() {
        let alpha = files_permissions("alpha", &env());
        let unset = Environment::new(PathBuf::from("/home/user"));

        assert_eq!(variable(&alpha, &env(), "PATH"), None);
        assert_eq!(variable(&alpha, &unset, "ALPHA_HOME"), None);
    }

    #[test]
    fn a_variable_holding_a_key_reads_as_none() {
        let alpha = files_permissions("alpha", &env());

        assert_eq!(variable(&alpha, &env(), "ALPHA_TOKEN"), None);
    }

    #[test]
    fn a_variable_of_another_part_of_the_same_plugin_reads_as_none() {
        let alpha = files_permissions("alpha", &env());
        let beta = files_permissions("beta", &env());

        assert_eq!(variable(&alpha, &env(), "BETA_HOME"), None);
        assert_eq!(
            variable(&beta, &env(), "BETA_HOME"),
            Some("/data/beta".to_string())
        );
    }
}
