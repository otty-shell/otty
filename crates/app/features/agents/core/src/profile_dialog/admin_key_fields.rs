//! The admin key field of the profile dialog (spec Decision 9): where the
//! admin key of the organization an API key profile pays through is kept,
//! and what is wrong with it. The key belongs to the organization, so
//! every profile of it shares it (spec 2026-11-19, Decision 7).

use std::path::Path;

use otty_agents::{Account, AdminKeyRef};

use super::event::AdminKeyChoice;
use crate::folder_field::{PathProblem, full_path};

/// The place of the admin key of an organization, as typed. Only the place
/// of the key is typed: the dialog never holds the key itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdminKeyFields {
    organization: Account,
    key_choice: AdminKeyChoice,
    key_file: String,
    key_variable: String,
}

impl AdminKeyFields {
    /// The organization whose admin key is typed.
    pub(crate) fn organization(&self) -> &Account {
        &self.organization
    }

    /// Where the key is: a file or a variable.
    pub(crate) fn key_choice(&self) -> AdminKeyChoice {
        self.key_choice
    }

    /// The typed path of the key file or the name of the variable,
    /// whichever is picked.
    pub(crate) fn key_text(&self) -> &str {
        match self.key_choice {
            AdminKeyChoice::File => &self.key_file,
            AdminKeyChoice::Variable => &self.key_variable,
        }
    }

    /// The fields of `organization` with its admin key as `saved`.
    pub(crate) fn new(
        organization: Account,
        saved: Option<&AdminKeyRef>,
    ) -> Self {
        let (key_choice, key_file, key_variable) = match saved {
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
            organization,
            key_choice,
            key_file,
            key_variable,
        }
    }

    /// Pick where the key is. Each choice keeps its own text.
    pub(crate) fn pick_key_choice(&mut self, choice: AdminKeyChoice) {
        self.key_choice = choice;
    }

    /// Take the typed path or name of the picked choice.
    pub(crate) fn set_key_text(&mut self, text: String) {
        match self.key_choice {
            AdminKeyChoice::File => self.key_file = text,
            AdminKeyChoice::Variable => self.key_variable = text,
        }
    }

    /// Where the key is, with `~` standing for `home`; `None` when nothing
    /// is typed.
    pub(crate) fn admin_key(
        &self,
        home: &Path,
    ) -> Result<Option<AdminKeyRef>, AdminKeyProblem> {
        match self.key_choice {
            AdminKeyChoice::File => match full_path(&self.key_file, home) {
                Ok(path) => Ok(Some(AdminKeyRef::File(path))),
                Err(PathProblem::Empty) => Ok(None),
                Err(PathProblem::NotFull) => {
                    Err(AdminKeyProblem::KeyPathNotFull)
                },
            },
            AdminKeyChoice::Variable => {
                let name = self.key_variable.trim();
                if name.is_empty() {
                    return Ok(None);
                }
                if !AdminKeyRef::is_variable_name(name) {
                    return Err(AdminKeyProblem::VariableName);
                }

                Ok(Some(AdminKeyRef::EnvVar(name.to_string())))
            },
        }
    }

    /// Whether the field passes its checks.
    pub(crate) fn is_valid(&self, home: &Path) -> bool {
        self.admin_key(home).is_ok()
    }
}

/// What is wrong with the admin key field of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdminKeyProblem {
    /// The path of the key file is not a full path.
    KeyPathNotFull,
    /// The variable is not a name of an environment variable.
    VariableName,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::testing::organization;

    const HOME: &str = "/home/user";

    fn fields() -> AdminKeyFields {
        AdminKeyFields::new(organization(), None)
    }

    #[test]
    fn the_key_file_is_a_full_path_with_the_home_folder_as_tilde() {
        let mut fields = fields();
        fields.set_key_text(String::from("~/.config/anthropic-admin-key"));
        let full = fields.admin_key(Path::new(HOME));
        fields.set_key_text(String::from("admin-key"));

        assert_eq!(
            full,
            Ok(Some(AdminKeyRef::File(PathBuf::from(
                "/home/user/.config/anthropic-admin-key"
            ))))
        );
        assert_eq!(
            fields.admin_key(Path::new(HOME)),
            Err(AdminKeyProblem::KeyPathNotFull)
        );
    }

    #[test]
    fn the_key_variable_is_a_variable_name() {
        let mut fields = fields();
        fields.pick_key_choice(AdminKeyChoice::Variable);
        fields.set_key_text(String::from(" ANTHROPIC_ADMIN_KEY "));
        let named = fields.admin_key(Path::new(HOME));
        fields.set_key_text(String::from("1KEY"));

        assert_eq!(
            named,
            Ok(Some(AdminKeyRef::EnvVar(String::from(
                "ANTHROPIC_ADMIN_KEY"
            ))))
        );
        assert_eq!(
            fields.admin_key(Path::new(HOME)),
            Err(AdminKeyProblem::VariableName)
        );
        assert!(!fields.is_valid(Path::new(HOME)));
    }

    #[test]
    fn nothing_typed_is_no_admin_key() {
        let fields = fields();

        assert_eq!(fields.admin_key(Path::new(HOME)), Ok(None));
        assert!(fields.is_valid(Path::new(HOME)));
    }

    #[test]
    fn the_field_starts_with_what_the_organization_saved() {
        let saved = AdminKeyRef::EnvVar(String::from("ADMIN"));

        let fields = AdminKeyFields::new(organization(), Some(&saved));

        assert_eq!(fields.key_choice(), AdminKeyChoice::Variable);
        assert_eq!(fields.key_text(), "ADMIN");
        assert_eq!(fields.organization(), &organization());
    }

    #[test]
    fn each_choice_keeps_its_own_text() {
        let mut fields = fields();
        fields.set_key_text(String::from("~/key"));

        fields.pick_key_choice(AdminKeyChoice::Variable);
        let variable = fields.key_text().to_string();
        fields.pick_key_choice(AdminKeyChoice::File);

        assert_eq!(variable, "");
        assert_eq!(fields.key_text(), "~/key");
    }
}
