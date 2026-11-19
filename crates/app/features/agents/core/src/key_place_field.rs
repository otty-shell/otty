//! The field where the user types where a key is kept: a file or an
//! environment variable (spec Decision 9, spec 2026-11-19, Decision 7).
//! Only the place of the key is typed: a dialog never holds the key
//! itself. The admin key of the profile dialog and the key of the Add
//! account dialog share it.

use std::path::Path;

use otty_agents::AdminKeyRef;

use super::folder_field::{PathProblem, full_path};
use super::profile_dialog::AdminKeyChoice;

/// The place of a key, as typed: a file or a variable, each with its own
/// text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyPlaceField {
    choice: AdminKeyChoice,
    file: String,
    variable: String,
}

impl KeyPlaceField {
    /// Where the key is: a file or a variable.
    pub(crate) fn choice(&self) -> AdminKeyChoice {
        self.choice
    }

    /// The typed path of the key file or the name of the variable,
    /// whichever is picked.
    pub(crate) fn text(&self) -> &str {
        match self.choice {
            AdminKeyChoice::File => &self.file,
            AdminKeyChoice::Variable => &self.variable,
        }
    }

    /// The field with the place `saved`, or empty on the file.
    pub(crate) fn new(saved: Option<&AdminKeyRef>) -> Self {
        let (choice, file, variable) = match saved {
            Some(AdminKeyRef::File(path)) => (
                AdminKeyChoice::File,
                path.display().to_string(),
                String::new(),
            ),
            Some(AdminKeyRef::EnvVar(name)) => {
                (AdminKeyChoice::Variable, String::new(), name.clone())
            },
            None => (AdminKeyChoice::File, String::new(), String::new()),
        };

        Self {
            choice,
            file,
            variable,
        }
    }

    /// Pick where the key is. Each choice keeps its own text.
    pub(crate) fn pick(&mut self, choice: AdminKeyChoice) {
        self.choice = choice;
    }

    /// Take the typed path or name of the picked choice.
    pub(crate) fn set_text(&mut self, text: String) {
        match self.choice {
            AdminKeyChoice::File => self.file = text,
            AdminKeyChoice::Variable => self.variable = text,
        }
    }

    /// Where the key is, with `~` standing for `home`; `None` when nothing
    /// is typed.
    pub(crate) fn key_place(
        &self,
        home: &Path,
    ) -> Result<Option<AdminKeyRef>, KeyPlaceProblem> {
        match self.choice {
            AdminKeyChoice::File => match full_path(&self.file, home) {
                Ok(path) => Ok(Some(AdminKeyRef::File(path))),
                Err(PathProblem::Empty) => Ok(None),
                Err(PathProblem::NotFull) => {
                    Err(KeyPlaceProblem::KeyPathNotFull)
                },
            },
            AdminKeyChoice::Variable => {
                let name = self.variable.trim();
                if name.is_empty() {
                    return Ok(None);
                }
                if !AdminKeyRef::is_variable_name(name) {
                    return Err(KeyPlaceProblem::VariableName);
                }

                Ok(Some(AdminKeyRef::EnvVar(name.to_string())))
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
        field.set_text(String::from("~/.config/anthropic-admin-key"));
        let full = field.key_place(Path::new(HOME));
        field.set_text(String::from("admin-key"));

        assert_eq!(
            full,
            Ok(Some(AdminKeyRef::File(PathBuf::from(
                "/home/user/.config/anthropic-admin-key"
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
        field.pick(AdminKeyChoice::Variable);
        field.set_text(String::from(" ANTHROPIC_ADMIN_KEY "));
        let named = field.key_place(Path::new(HOME));
        field.set_text(String::from("1KEY"));

        assert_eq!(
            named,
            Ok(Some(AdminKeyRef::EnvVar(String::from(
                "ANTHROPIC_ADMIN_KEY"
            ))))
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
        let saved = AdminKeyRef::EnvVar(String::from("ADMIN"));

        let field = KeyPlaceField::new(Some(&saved));

        assert_eq!(field.choice(), AdminKeyChoice::Variable);
        assert_eq!(field.text(), "ADMIN");
    }

    #[test]
    fn each_choice_keeps_its_own_text() {
        let mut field = KeyPlaceField::new(None);
        field.set_text(String::from("~/key"));

        field.pick(AdminKeyChoice::Variable);
        let variable = field.text().to_string();
        field.pick(AdminKeyChoice::File);

        assert_eq!(variable, "");
        assert_eq!(field.text(), "~/key");
    }
}
