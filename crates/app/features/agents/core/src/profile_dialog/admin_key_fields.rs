//! The admin key field of the profile dialog (spec Decision 9): where the
//! admin key of the organization an API key profile pays through is kept,
//! and what is wrong with it. The key belongs to the organization, so
//! every profile of it shares it (spec 2026-11-19, Decision 7).

use std::path::Path;

use otty_agents::{Account, AdminKeyRef};

use super::event::AdminKeyChoice;
use crate::key_place_field::{KeyPlaceField, KeyPlaceProblem};

/// The place of the admin key of an organization, as typed. Only the place
/// of the key is typed: the dialog never holds the key itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdminKeyFields {
    organization: Account,
    place: KeyPlaceField,
}

impl AdminKeyFields {
    /// The organization whose admin key is typed.
    pub(crate) fn organization(&self) -> &Account {
        &self.organization
    }

    /// Where the key is: a file or a variable.
    pub(crate) fn key_choice(&self) -> AdminKeyChoice {
        self.place.choice()
    }

    /// The typed path of the key file or the name of the variable,
    /// whichever is picked.
    pub(crate) fn key_text(&self) -> &str {
        self.place.text()
    }

    /// The fields of `organization` with its admin key as `saved`.
    pub(crate) fn new(
        organization: Account,
        saved: Option<&AdminKeyRef>,
    ) -> Self {
        Self {
            organization,
            place: KeyPlaceField::new(saved),
        }
    }

    /// Pick where the key is. Each choice keeps its own text.
    pub(crate) fn pick_key_choice(&mut self, choice: AdminKeyChoice) {
        self.place.pick(choice);
    }

    /// Take the typed path or name of the picked choice.
    pub(crate) fn set_key_text(&mut self, text: String) {
        self.place.set_text(text);
    }

    /// Where the key is, with `~` standing for `home`; `None` when nothing
    /// is typed.
    pub(crate) fn admin_key(
        &self,
        home: &Path,
    ) -> Result<Option<AdminKeyRef>, KeyPlaceProblem> {
        self.place.key_place(home)
    }

    /// Whether the field passes its checks.
    pub(crate) fn is_valid(&self, home: &Path) -> bool {
        self.admin_key(home).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::organization;

    const HOME: &str = "/home/user";

    #[test]
    fn nothing_typed_is_no_admin_key_and_valid() {
        let fields = AdminKeyFields::new(organization(), None);

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
    fn a_wrong_variable_name_is_not_valid() {
        let mut fields = AdminKeyFields::new(organization(), None);
        fields.pick_key_choice(AdminKeyChoice::Variable);
        fields.set_key_text(String::from("1KEY"));

        assert!(!fields.is_valid(Path::new(HOME)));
    }
}
