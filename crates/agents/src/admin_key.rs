//! Where the admin key of an Anthropic organization is kept, and reading it
//! for one request (spec Decision 9).
//!
//! OTTY never stores the key: `agents.json` holds only the path of the file
//! that holds it, or the name of the environment variable. The key is read
//! when the bill is read and dropped right after the request.

use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use crate::error::{SourceError, Unavailable};

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
    /// Whether `name` can name an environment variable: ASCII letters,
    /// digits and `_`, not starting with a digit. Only such a name is ever
    /// put into a command of the user's shell.
    ///
    /// ```
    /// use otty_agents::AdminKeyRef;
    ///
    /// assert!(AdminKeyRef::is_variable_name("ANTHROPIC_ADMIN_KEY"));
    /// assert!(!AdminKeyRef::is_variable_name("KEY; echo"));
    /// ```
    pub fn is_variable_name(name: &str) -> bool {
        let mut bytes = name.bytes();
        let starts_well = bytes
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_');

        starts_well
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    }

    /// The key, read from its file or its variable. `env_var` gives the
    /// value of a variable: the crate reads no environment itself.
    ///
    /// A missing file, an unset variable and an empty key are "not
    /// available", and a file that can not be read is "unreadable". A
    /// variable whose name is not a variable name is "not available" too
    /// and is never asked for. The error names the file or the variable,
    /// never the key, and a wrong name is not shown, since it may be the
    /// key typed in the wrong field.
    pub fn read(
        &self,
        env_var: impl Fn(&str) -> Option<String>,
    ) -> Result<AdminKey, SourceError> {
        let text =
            match self {
                Self::File(path) => std::fs::read_to_string(path).map_err(
                    |error| match error.kind() {
                        ErrorKind::NotFound => self.missing(),
                        _ => SourceError::Unreadable {
                            path: path.clone(),
                            detail: error.to_string(),
                        },
                    },
                )?,
                Self::EnvVar(name) => {
                    if !Self::is_variable_name(name) {
                        return Err(SourceError::NotAvailable(
                            Unavailable::AdminKeyVariableNotValid,
                        ));
                    }

                    env_var(name).ok_or_else(|| self.missing())?
                },
            };

        let secret = text.trim();
        if secret.is_empty() {
            return Err(SourceError::NotAvailable(Unavailable::AdminKeyEmpty(
                self.clone(),
            )));
        }

        Ok(AdminKey {
            secret: secret.to_string(),
        })
    }

    /// The failure of a key that is not where this reference says.
    fn missing(&self) -> SourceError {
        SourceError::NotAvailable(Unavailable::AdminKeyMissing(self.clone()))
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

        let reference = AdminKeyRef::File(file);

        let error = reference.read(no_variables).expect_err("there is no file");

        assert_eq!(
            error,
            SourceError::NotAvailable(Unavailable::AdminKeyMissing(reference))
        );
    }

    #[test]
    fn a_folder_in_place_of_the_key_file_can_not_be_read() {
        let dir = TempDir::new();

        let error = AdminKeyRef::File(dir.path().to_path_buf())
            .read(no_variables)
            .expect_err("a folder holds no key");

        assert!(
            matches!(
                &error,
                SourceError::Unreadable { path, .. } if path == dir.path()
            ),
            "got {error:?}"
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
            SourceError::NotAvailable(Unavailable::AdminKeyMissing(
                reference.clone()
            ))
        );
        assert_eq!(
            empty,
            SourceError::NotAvailable(Unavailable::AdminKeyEmpty(reference))
        );
    }

    #[test]
    fn a_variable_name_is_letters_digits_and_underscores() {
        assert!(AdminKeyRef::is_variable_name("ANTHROPIC_ADMIN_KEY"));
        assert!(AdminKeyRef::is_variable_name("_key2"));
        assert!(!AdminKeyRef::is_variable_name(""));
        assert!(!AdminKeyRef::is_variable_name("1KEY"));
        assert!(!AdminKeyRef::is_variable_name("KEY NAME"));
        assert!(!AdminKeyRef::is_variable_name("KEY;rm"));
        assert!(!AdminKeyRef::is_variable_name("$(KEY)"));
        assert!(!AdminKeyRef::is_variable_name("КЛЮЧ"));
    }

    #[test]
    fn a_wrong_variable_name_is_not_asked_for_and_not_shown() {
        let reference = AdminKeyRef::EnvVar(String::from(MADE_UP_KEY));

        let error = reference
            .read(|_| panic!("a wrong name is never asked for"))
            .expect_err("not a variable name");

        assert_eq!(
            error,
            SourceError::NotAvailable(Unavailable::AdminKeyVariableNotValid)
        );
    }

    #[test]
    fn printing_a_key_shows_no_part_of_it() {
        let key = AdminKey::made_up(MADE_UP_KEY);

        let printed = format!("{key:?}");

        assert_eq!(printed, "AdminKey(hidden)");
    }
}
