//! Where a key the user gave OTTY is kept, and reading it for one request:
//! the key of an account the user added (spec 2026-11-19, Decision 7), or
//! a key a plugin part asked for with a need, given under the part's slot
//! (spec 2026-11-19-agent-plugins, issue 20).
//!
//! OTTY never stores the key: `agents.json` holds only the path of the file
//! that holds it, or the name of the environment variable. The key is read
//! right before the part reads the account and dropped right after the
//! request.

use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use crate::error::{SourceError, Unavailable};

/// Where a key the user gave is kept, the key of an added account or one a
/// part asked for: a file or an environment variable. This is all OTTY
/// saves; the key itself is never written.
///
/// ```
/// use otty_agents::KeyRef;
///
/// let reference = KeyRef::EnvVar(String::from("EXAMPLE_KEY"));
///
/// let key = reference
///     .read(|name| (name == "EXAMPLE_KEY").then(|| "made-up".into()));
///
/// assert!(key.is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyRef {
    /// A file that holds the key. Spaces and line ends around it are
    /// dropped.
    File(PathBuf),
    /// The name of an environment variable that holds the key.
    EnvVar(String),
}

impl KeyRef {
    /// Whether `name` can name an environment variable: ASCII letters,
    /// digits and `_`, not starting with a digit. Only such a name is ever
    /// put into a command of the user's shell.
    ///
    /// ```
    /// use otty_agents::KeyRef;
    ///
    /// assert!(KeyRef::is_variable_name("EXAMPLE_KEY"));
    /// assert!(!KeyRef::is_variable_name("KEY; echo"));
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
    ) -> Result<KeySecret, SourceError> {
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
                            Unavailable::KeyVariableNotValid,
                        ));
                    }

                    env_var(name).ok_or_else(|| self.missing())?
                },
            };

        let secret = text.trim();
        if secret.is_empty() {
            return Err(SourceError::NotAvailable(Unavailable::KeyEmpty(
                self.clone(),
            )));
        }

        Ok(KeySecret {
            secret: secret.to_string(),
        })
    }

    /// The failure of a key that is not where this reference says.
    fn missing(&self) -> SourceError {
        SourceError::NotAvailable(Unavailable::KeyMissing(self.clone()))
    }
}

/// A key the user gave for an account after a need of a plugin part asked
/// for it: the part's own name for it, its slot, such as `admin`, and
/// where the user keeps it. The part gets it back as a handle with the
/// slot, never the key.
///
/// ```
/// use otty_agents::{GivenKey, KeyRef};
///
/// let given = GivenKey::new("admin", KeyRef::EnvVar("ORG_KEY".into()));
///
/// assert_eq!(given.slot(), "admin");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GivenKey {
    slot: String,
    key: KeyRef,
}

impl GivenKey {
    /// The part's own name for the key, such as `admin`.
    pub fn slot(&self) -> &str {
        &self.slot
    }

    /// Where the user keeps the key.
    pub fn key(&self) -> &KeyRef {
        &self.key
    }

    /// The key kept at `key`, given under `slot`.
    pub fn new(slot: impl Into<String>, key: KeyRef) -> Self {
        Self {
            slot: slot.into(),
            key,
        }
    }
}

/// A key the user gave, read for one read of a part, with its slot.
#[derive(Debug)]
pub(crate) struct GivenSecret {
    slot: String,
    secret: KeySecret,
}

impl GivenSecret {
    /// The part's own name for the key.
    pub(crate) fn slot(&self) -> &str {
        &self.slot
    }

    /// The key, read for this read only.
    pub(crate) fn secret(&self) -> &KeySecret {
        &self.secret
    }
}

#[cfg(test)]
impl GivenSecret {
    /// The made-up key `secret` given under `slot`, for the tests.
    pub(crate) fn made_up(slot: &str, secret: &str) -> Self {
        Self {
            slot: slot.to_string(),
            secret: KeySecret::made_up(secret),
        }
    }
}

/// A key read for one request: the key of an account, a key the user gave
/// for a part, or the login token of an agent. It can not be copied, and
/// printing it shows no part of the key.
pub struct KeySecret {
    secret: String,
}

impl KeySecret {
    /// The key, to put into the header of the request.
    pub(crate) fn secret(&self) -> &str {
        &self.secret
    }
}

#[cfg(test)]
impl KeySecret {
    /// A made-up key for the tests.
    pub(crate) fn made_up(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
        }
    }
}

impl fmt::Debug for KeySecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeySecret(hidden)")
    }
}

/// Each key of `given`, read from its file or its variable for one read,
/// with its slot; `env_var` gives the value of a variable. A key that is
/// not where the user keeps it fails the read, so the user sees why.
pub(crate) fn read_given_keys(
    given: &[GivenKey],
    env_var: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<GivenSecret>, SourceError> {
    given
        .iter()
        .map(|given| {
            Ok(GivenSecret {
                slot: given.slot.clone(),
                secret: given.key.read(env_var)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    /// A made-up key, never a real one.
    const MADE_UP_KEY: &str = "sk-made-up-for-tests-0001";

    fn no_variables(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn the_key_is_read_from_its_file_without_the_line_end() {
        let dir = TempDir::new();
        let file = dir.path().join("example-key");
        std::fs::write(&file, format!("  {MADE_UP_KEY}\n"))
            .expect("the key file");

        let key = KeyRef::File(file)
            .read(no_variables)
            .expect("the file holds a key");

        assert_eq!(key.secret(), MADE_UP_KEY);
    }

    #[test]
    fn a_missing_key_file_is_not_available_and_named() {
        let dir = TempDir::new();
        let file = dir.path().join("no-such-key");

        let reference = KeyRef::File(file);

        let error = reference.read(no_variables).expect_err("there is no file");

        assert_eq!(
            error,
            SourceError::NotAvailable(Unavailable::KeyMissing(reference))
        );
    }

    #[test]
    fn a_folder_in_place_of_the_key_file_can_not_be_read() {
        let dir = TempDir::new();

        let error = KeyRef::File(dir.path().to_path_buf())
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
        let reference = KeyRef::EnvVar(String::from("OTTY_TEST_KEY"));

        let key = reference
            .read(|name| {
                (name == "OTTY_TEST_KEY").then(|| String::from(MADE_UP_KEY))
            })
            .expect("the variable holds a key");

        assert_eq!(key.secret(), MADE_UP_KEY);
    }

    #[test]
    fn an_unset_variable_or_an_empty_key_is_not_available() {
        let reference = KeyRef::EnvVar(String::from("OTTY_TEST_KEY"));

        let unset = reference.read(no_variables).expect_err("not set");
        let empty = reference
            .read(|_| Some(String::from(" \n")))
            .expect_err("empty");

        assert_eq!(
            unset,
            SourceError::NotAvailable(Unavailable::KeyMissing(
                reference.clone()
            ))
        );
        assert_eq!(
            empty,
            SourceError::NotAvailable(Unavailable::KeyEmpty(reference))
        );
    }

    #[test]
    fn a_variable_name_is_letters_digits_and_underscores() {
        assert!(KeyRef::is_variable_name("EXAMPLE_KEY"));
        assert!(KeyRef::is_variable_name("_key2"));
        assert!(!KeyRef::is_variable_name(""));
        assert!(!KeyRef::is_variable_name("1KEY"));
        assert!(!KeyRef::is_variable_name("KEY NAME"));
        assert!(!KeyRef::is_variable_name("KEY;rm"));
        assert!(!KeyRef::is_variable_name("$(KEY)"));
        assert!(!KeyRef::is_variable_name("КЛЮЧ"));
    }

    #[test]
    fn a_wrong_variable_name_is_not_asked_for_and_not_shown() {
        let reference = KeyRef::EnvVar(String::from(MADE_UP_KEY));

        let error = reference
            .read(|_| panic!("a wrong name is never asked for"))
            .expect_err("not a variable name");

        assert_eq!(
            error,
            SourceError::NotAvailable(Unavailable::KeyVariableNotValid)
        );
    }

    #[test]
    fn given_keys_are_read_with_their_slots() {
        let given = [
            GivenKey::new("admin", KeyRef::EnvVar("OTTY_TEST_ADMIN".into())),
            GivenKey::new("other", KeyRef::EnvVar("OTTY_TEST_OTHER".into())),
        ];
        let variables = |name: &str| match name {
            "OTTY_TEST_ADMIN" => Some(String::from(MADE_UP_KEY)),
            "OTTY_TEST_OTHER" => Some(String::from("sk-made-up-other-0002")),
            _ => None,
        };

        let read = read_given_keys(&given, &variables).expect("both set");
        let missing = read_given_keys(&given, &no_variables);

        let slots: Vec<(&str, &str)> = read
            .iter()
            .map(|given| (given.slot(), given.secret().secret()))
            .collect();
        assert_eq!(
            slots,
            [("admin", MADE_UP_KEY), ("other", "sk-made-up-other-0002")]
        );
        assert!(matches!(
            missing,
            Err(SourceError::NotAvailable(Unavailable::KeyMissing(_)))
        ));
    }

    #[test]
    fn printing_a_key_shows_no_part_of_it() {
        let key = KeySecret::made_up(MADE_UP_KEY);

        let printed = format!("{key:?}");

        assert_eq!(printed, "KeySecret(hidden)");
    }
}
