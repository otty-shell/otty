//! Where the admin key of an Anthropic organization is kept, and reading it
//! for one request (spec Decision 9).
//!
//! OTTY never stores the key: `agents.json` holds only the path of the file
//! that holds it, or the name of the environment variable. The key is read
//! when the bill is read and dropped right after the request.

use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use crate::error::SourceError;

/// Where the admin key of a profile is kept: a file or an environment
/// variable. This is all OTTY saves; the key itself is never written.
///
/// ```
/// use otty_agents::AdminKeyRef;
///
/// let reference = AdminKeyRef::EnvVar(String::from("ANTHROPIC_ADMIN_KEY"));
///
/// let key = reference
///     .read(|name| (name == "ANTHROPIC_ADMIN_KEY").then(|| "made-up".into()));
///
/// assert!(key.is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminKeyRef {
    /// A file that holds the key. Spaces and line ends around it are
    /// dropped.
    File(PathBuf),
    /// The name of an environment variable that holds the key.
    EnvVar(String),
}

impl AdminKeyRef {
    /// The key, read from its file or its variable. `env_var` gives the
    /// value of a variable: the crate reads no environment itself.
    ///
    /// A missing or unreadable file, an unset variable and an empty key are
    /// "not available". The error names the file or the variable, never
    /// the key.
    pub fn read(
        &self,
        env_var: impl Fn(&str) -> Option<String>,
    ) -> Result<AdminKey, SourceError> {
        let (text, place) = match self {
            Self::File(path) => {
                let place = format!("the admin key file {}", path.display());
                let text = std::fs::read_to_string(path).map_err(|error| {
                    let problem = match error.kind() {
                        ErrorKind::NotFound => "is missing",
                        _ => "can not be read",
                    };
                    SourceError::NotAvailable(format!("{place} {problem}"))
                })?;
                (text, place)
            },
            Self::EnvVar(name) => {
                let text = env_var(name).ok_or_else(|| {
                    SourceError::NotAvailable(format!("{name} is not set"))
                })?;
                (text, name.clone())
            },
        };

        let secret = text.trim();
        if secret.is_empty() {
            return Err(SourceError::NotAvailable(format!("{place} is empty")));
        }

        Ok(AdminKey {
            secret: secret.to_string(),
        })
    }
}

/// An admin key read for one request. It can not be copied, and printing
/// it shows no part of the key.
pub struct AdminKey {
    secret: String,
}

impl AdminKey {
    /// The key, to put into the header of the request.
    pub(crate) fn secret(&self) -> &str {
        &self.secret
    }
}

#[cfg(test)]
impl AdminKey {
    /// A made-up key for the tests.
    pub(crate) fn made_up(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
        }
    }
}

impl fmt::Debug for AdminKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AdminKey(hidden)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    /// A made-up admin key, never a real one.
    const MADE_UP_KEY: &str = "sk-ant-admin01-made-up-for-tests";

    fn no_variables(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn the_key_is_read_from_its_file_without_the_line_end() {
        let dir = TempDir::new();
        let file = dir.path().join("admin-key");
        std::fs::write(&file, format!("  {MADE_UP_KEY}\n"))
            .expect("the key file");

        let key = AdminKeyRef::File(file)
            .read(no_variables)
            .expect("the file holds a key");

        assert_eq!(key.secret(), MADE_UP_KEY);
    }

    #[test]
    fn a_missing_key_file_is_not_available_and_named() {
        let dir = TempDir::new();
        let file = dir.path().join("no-such-key");

        let error = AdminKeyRef::File(file.clone())
            .read(no_variables)
            .expect_err("there is no file");

        assert_eq!(
            error,
            SourceError::NotAvailable(format!(
                "the admin key file {} is missing",
                file.display()
            ))
        );
    }

    #[test]
    fn a_folder_in_place_of_the_key_file_can_not_be_read() {
        let dir = TempDir::new();

        let error = AdminKeyRef::File(dir.path().to_path_buf())
            .read(no_variables)
            .expect_err("a folder holds no key");

        assert_eq!(
            error,
            SourceError::NotAvailable(format!(
                "the admin key file {} can not be read",
                dir.path().display()
            ))
        );
    }

    #[test]
    fn the_key_is_read_from_its_variable() {
        let reference = AdminKeyRef::EnvVar(String::from("OTTY_TEST_KEY"));

        let key = reference
            .read(|name| {
                (name == "OTTY_TEST_KEY").then(|| String::from(MADE_UP_KEY))
            })
            .expect("the variable holds a key");

        assert_eq!(key.secret(), MADE_UP_KEY);
    }

    #[test]
    fn an_unset_variable_or_an_empty_key_is_not_available() {
        let reference = AdminKeyRef::EnvVar(String::from("OTTY_TEST_KEY"));

        let unset = reference.read(no_variables).expect_err("not set");
        let empty = reference
            .read(|_| Some(String::from(" \n")))
            .expect_err("empty");

        assert_eq!(
            unset,
            SourceError::NotAvailable(String::from("OTTY_TEST_KEY is not set"))
        );
        assert_eq!(
            empty,
            SourceError::NotAvailable(String::from("OTTY_TEST_KEY is empty"))
        );
    }

    #[test]
    fn printing_a_key_shows_no_part_of_it() {
        let key = AdminKey::made_up(MADE_UP_KEY);

        let printed = format!("{key:?}");

        assert_eq!(printed, "AdminKey(hidden)");
    }
}
