//! The field where the user types where a key is kept: a file or an
//! environment variable (spec 2026-11-19, Decision 7). Only the place of
//! the key is typed: a dialog never holds the key itself. The key of the
//! Add account dialog and a key a plugin part asks for share it.

use std::fmt;
use std::path::Path;

use otty_agents::KeyRef;
use otty_app_i18n::{self as i18n, Key};

use super::folder_field::{PathProblem, full_path};

/// Where a key the user gives is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPlaceChoice {
    /// A file that holds the key.
    File,
    /// An environment variable that holds the key.
    Variable,
}

/// A place of a key, shown by its word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyPlaceChoiceOption(pub(crate) KeyPlaceChoice);

impl fmt::Display for KeyPlaceChoiceOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self.0 {
            KeyPlaceChoice::File => Key::AgentsKeyPlaceFile,
            KeyPlaceChoice::Variable => Key::AgentsKeyPlaceVariable,
        };

        f.write_str(i18n::t(key))
    }
}

/// The place of a key, as typed: a file or a variable, each with its own
/// text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyPlaceField {
    choice: KeyPlaceChoice,
    file: String,
    variable: String,
}

impl KeyPlaceField {
    /// Where the key is: a file or a variable.
    pub(crate) fn choice(&self) -> KeyPlaceChoice {
        self.choice
    }

    /// The typed path of the key file or the name of the variable,
    /// whichever is picked.
    pub(crate) fn text(&self) -> &str {
        match self.choice {
            KeyPlaceChoice::File => &self.file,
            KeyPlaceChoice::Variable => &self.variable,
        }
    }

    /// The field with the place `saved`, or empty on the file.
    pub(crate) fn new(saved: Option<&KeyRef>) -> Self {
        let (choice, file, variable) = match saved {
            Some(KeyRef::File(path)) => (
                KeyPlaceChoice::File,
                path.display().to_string(),
                String::new(),
            ),
            Some(KeyRef::EnvVar(name)) => {
                (KeyPlaceChoice::Variable, String::new(), name.clone())
            },
            None => (KeyPlaceChoice::File, String::new(), String::new()),
        };

        Self {
            choice,
            file,
            variable,
        }
    }

    /// Pick where the key is. Each choice keeps its own text.
    pub(crate) fn pick(&mut self, choice: KeyPlaceChoice) {
        self.choice = choice;
    }

    /// Take the typed path or name of the picked choice.
    pub(crate) fn set_text(&mut self, text: String) {
        match self.choice {
            KeyPlaceChoice::File => self.file = text,
            KeyPlaceChoice::Variable => self.variable = text,
        }
    }

    /// Where the key is, with `~` standing for `home`; `None` when nothing
    /// is typed.
    pub(crate) fn key_place(
        &self,
        home: &Path,
    ) -> Result<Option<KeyRef>, KeyPlaceProblem> {
        match self.choice {
            KeyPlaceChoice::File => match full_path(&self.file, home) {
                Ok(path) => Ok(Some(KeyRef::File(path))),
                Err(PathProblem::Empty) => Ok(None),
                Err(PathProblem::NotFull) => {
                    Err(KeyPlaceProblem::KeyPathNotFull)
                },
            },
            KeyPlaceChoice::Variable => {
                let name = self.variable.trim();
                if name.is_empty() {
                    return Ok(None);
                }
                if !KeyRef::is_variable_name(name) {
                    return Err(KeyPlaceProblem::VariableName);
                }

                Ok(Some(KeyRef::EnvVar(name.to_string())))
            },
        }
    }
}

/// What is wrong with the typed place of a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyPlaceProblem {
    /// The path of the key file is not a full path.
    KeyPathNotFull,
    /// The variable is not a name of an environment variable.
    VariableName,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const HOME: &str = "/home/user";

    #[test]
    fn the_key_file_is_a_full_path_with_the_home_folder_as_tilde() {
        let mut field = KeyPlaceField::new(None);
        field.set_text(String::from("~/.config/anthropic-org-key"));
        let full = field.key_place(Path::new(HOME));
        field.set_text(String::from("org-key"));

        assert_eq!(
            full,
            Ok(Some(KeyRef::File(PathBuf::from(
                "/home/user/.config/anthropic-org-key"
            ))))
        );
        assert_eq!(
            field.key_place(Path::new(HOME)),
            Err(KeyPlaceProblem::KeyPathNotFull)
        );
    }

    #[test]
    fn the_key_variable_is_a_variable_name() {
        let mut field = KeyPlaceField::new(None);
        field.pick(KeyPlaceChoice::Variable);
        field.set_text(String::from(" ANTHROPIC_BILL_KEY "));
        let named = field.key_place(Path::new(HOME));
        field.set_text(String::from("1KEY"));

        assert_eq!(
            named,
            Ok(Some(KeyRef::EnvVar(String::from("ANTHROPIC_BILL_KEY"))))
        );
        assert_eq!(
            field.key_place(Path::new(HOME)),
            Err(KeyPlaceProblem::VariableName)
        );
    }

    #[test]
    fn nothing_typed_is_no_place() {
        let field = KeyPlaceField::new(None);

        assert_eq!(field.key_place(Path::new(HOME)), Ok(None));
    }

    #[test]
    fn the_field_starts_with_the_saved_place() {
        let saved = KeyRef::EnvVar(String::from("ADMIN"));

        let field = KeyPlaceField::new(Some(&saved));

        assert_eq!(field.choice(), KeyPlaceChoice::Variable);
        assert_eq!(field.text(), "ADMIN");
    }

    #[test]
    fn each_choice_keeps_its_own_text() {
        let mut field = KeyPlaceField::new(None);
        field.set_text(String::from("~/key"));

        field.pick(KeyPlaceChoice::Variable);
        let variable = field.text().to_string();
        field.pick(KeyPlaceChoice::File);

        assert_eq!(variable, "");
        assert_eq!(field.text(), "~/key");
    }
}
