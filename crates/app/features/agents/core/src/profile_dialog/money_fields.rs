//! The fields of an API key profile in the profile dialog (spec Decision
//! 9): where its admin key is kept and its monthly budget, and what is
//! wrong with them.

use std::path::Path;

use otty_agents::{AdminKeyRef, Dollars, Profile};

use super::event::AdminKeyChoice;
use crate::folder_field::{PathProblem, full_path};

/// The admin key and the budget of an API key profile, as typed. Only the
/// place of the key is typed: the dialog never holds the key itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyFields {
    has_admin_key: bool,
    key_choice: AdminKeyChoice,
    key_file: String,
    key_variable: String,
    budget: String,
}

impl MoneyFields {
    /// Whether the admin key field shows: the profile's agent has a bill
    /// OTTY can read.
    pub(crate) fn has_admin_key(&self) -> bool {
        self.has_admin_key
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

    /// The budget as typed.
    pub(crate) fn budget_text(&self) -> &str {
        &self.budget
    }

    /// The fields of `profile` as it is saved; the admin key field only
    /// when its agent `has_admin_key`.
    pub(crate) fn new(profile: &Profile, has_admin_key: bool) -> Self {
        let (key_choice, key_file, key_variable) = match profile.admin_key() {
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
            has_admin_key,
            key_choice,
            key_file,
            key_variable,
            budget: profile
                .monthly_budget()
                .map(budget_text)
                .unwrap_or_default(),
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

    /// Take the typed budget.
    pub(crate) fn set_budget(&mut self, text: String) {
        self.budget = text;
    }

    /// Where the key is, with `~` standing for `home`; `None` when nothing
    /// is typed or the agent has no bill.
    pub(crate) fn admin_key(
        &self,
        home: &Path,
    ) -> Result<Option<AdminKeyRef>, MoneyProblem> {
        if !self.has_admin_key {
            return Ok(None);
        }

        match self.key_choice {
            AdminKeyChoice::File => match full_path(&self.key_file, home) {
                Ok(path) => Ok(Some(AdminKeyRef::File(path))),
                Err(PathProblem::Empty) => Ok(None),
                Err(PathProblem::NotFull) => Err(MoneyProblem::KeyPathNotFull),
            },
            AdminKeyChoice::Variable => {
                let name = self.key_variable.trim();
                if name.is_empty() {
                    return Ok(None);
                }
                if !AdminKeyRef::is_variable_name(name) {
                    return Err(MoneyProblem::VariableName);
                }

                Ok(Some(AdminKeyRef::EnvVar(name.to_string())))
            },
        }
    }

    /// The budget typed in dollars; `None` when nothing is typed.
    pub(crate) fn budget(&self) -> Result<Option<Dollars>, MoneyProblem> {
        parse_budget(&self.budget)
    }

    /// Whether both fields pass their checks.
    pub(crate) fn is_valid(&self, home: &Path) -> bool {
        self.admin_key(home).is_ok() && self.budget().is_ok()
    }
}

/// What is wrong with the money fields of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoneyProblem {
    /// The path of the key file is not a full path.
    KeyPathNotFull,
    /// The variable is not a name of an environment variable.
    VariableName,
    /// The budget is not an amount of dollars.
    Budget,
}

/// A budget written for its field: whole dollars as they are, such as
/// "300", else with cents, such as "49.50".
fn budget_text(budget: Dollars) -> String {
    let all_cents = budget.cents();
    let dollars = all_cents / 100;
    let cents = all_cents % 100;

    if cents == 0 {
        format!("{dollars}")
    } else {
        format!("{dollars}.{cents:02}")
    }
}

/// The budget typed as `text`: dollars with up to two digits of cents, a
/// `$` before them allowed; `None` when nothing is typed.
fn parse_budget(text: &str) -> Result<Option<Dollars>, MoneyProblem> {
    let text = text.trim();
    let text = text.strip_prefix('$').unwrap_or(text).trim();
    if text.is_empty() {
        return Ok(None);
    }

    let (whole, cents) = text.split_once('.').unwrap_or((text, "0"));
    let is_amount = !whole.is_empty()
        && (1..=2).contains(&cents.len())
        && whole
            .bytes()
            .chain(cents.bytes())
            .all(|byte| byte.is_ascii_digit());
    if !is_amount {
        return Err(MoneyProblem::Budget);
    }

    let whole: u64 = whole.parse().map_err(|_| MoneyProblem::Budget)?;
    let cents: u64 = format!("{cents:0<2}")
        .parse()
        .map_err(|_| MoneyProblem::Budget)?;
    let budget = whole
        .checked_mul(100)
        .and_then(|total| total.checked_add(cents))
        .and_then(Dollars::from_cents)
        .ok_or(MoneyProblem::Budget)?;

    Ok(Some(budget))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::testing::{claude, dollars};

    const HOME: &str = "/home/user";

    fn fields() -> MoneyFields {
        MoneyFields::new(&claude(), true)
    }

    fn budget(text: &str) -> Result<Option<Dollars>, MoneyProblem> {
        parse_budget(text)
    }

    #[test]
    fn a_budget_is_dollars_with_up_to_two_digits_of_cents() {
        assert_eq!(budget("300"), Ok(Some(dollars(300))));
        assert_eq!(
            budget(" $49.5 "),
            Ok(Some(Dollars::from_micro_dollars(49_500_000)))
        );
        assert_eq!(
            budget("0.99"),
            Ok(Some(Dollars::from_micro_dollars(990_000)))
        );
        assert_eq!(budget(""), Ok(None));
        for wrong in ["-5", "12.345", "1,5", "abc", ".5", "5.", "1e3"] {
            assert_eq!(budget(wrong), Err(MoneyProblem::Budget), "{wrong}");
        }
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
            Err(MoneyProblem::KeyPathNotFull)
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
            Err(MoneyProblem::VariableName)
        );
        assert!(!fields.is_valid(Path::new(HOME)));
    }

    #[test]
    fn nothing_typed_is_no_admin_key_and_no_budget() {
        let fields = fields();

        assert_eq!(fields.admin_key(Path::new(HOME)), Ok(None));
        assert_eq!(fields.budget(), Ok(None));
        assert!(fields.is_valid(Path::new(HOME)));
    }

    #[test]
    fn the_fields_start_with_what_the_profile_saved() {
        let profile = claude()
            .with_admin_key(Some(AdminKeyRef::EnvVar(String::from("ADMIN"))))
            .with_monthly_budget(Some(Dollars::from_micro_dollars(49_500_000)));

        let fields = MoneyFields::new(&profile, true);

        assert_eq!(fields.key_choice(), AdminKeyChoice::Variable);
        assert_eq!(fields.key_text(), "ADMIN");
        assert_eq!(fields.budget_text(), "49.50");
        assert_eq!(budget_text(dollars(300)), "300");
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

    #[test]
    fn an_agent_with_no_bill_has_no_admin_key() {
        let mut fields = MoneyFields::new(&claude(), false);
        fields.set_key_text(String::from("relative"));

        assert!(!fields.has_admin_key());
        assert_eq!(fields.admin_key(Path::new(HOME)), Ok(None));
    }
}
