use otty_app_i18n::{self as i18n, Key};

/// Why the settings could not be read or saved, told to the user as a
/// notification.
///
/// The text is built when it is shown, not when the problem is found: the
/// settings applied in the same update may change the language first.
///
/// # Example
///
/// ```
/// use otty_app_settings::SettingsProblem;
///
/// let problem = SettingsProblem::SaveFailed(String::from("disk full"));
///
/// assert!(problem.text().contains("disk full"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsProblem {
    /// The settings file is there but could not be read; carries the error.
    ReadFailed(String),
    /// The settings file is broken, so the defaults are used; carries the
    /// error. The next save writes over the file.
    FileInvalid(String),
    /// The settings could not be written; carries the error.
    SaveFailed(String),
}

impl SettingsProblem {
    /// Return the text that tells the user, in the active language.
    pub fn text(&self) -> String {
        match self {
            Self::ReadFailed(error) => {
                i18n::with_error(Key::TplSettingsReadFailed, error)
            },
            Self::FileInvalid(error) => {
                i18n::with_error(Key::TplSettingsFileInvalid, error)
            },
            Self::SaveFailed(error) => {
                i18n::with_error(Key::TplSettingsSaveFailed, error)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_problem_fills_its_own_template_with_the_error() {
        for (problem, key) in [
            (
                SettingsProblem::ReadFailed(String::from("no access")),
                Key::TplSettingsReadFailed,
            ),
            (
                SettingsProblem::FileInvalid(String::from("expected value")),
                Key::TplSettingsFileInvalid,
            ),
            (
                SettingsProblem::SaveFailed(String::from("disk full")),
                Key::TplSettingsSaveFailed,
            ),
        ] {
            let error = match &problem {
                SettingsProblem::ReadFailed(error)
                | SettingsProblem::FileInvalid(error)
                | SettingsProblem::SaveFailed(error) => error.clone(),
            };

            assert_eq!(problem.text(), i18n::with_error(key, &error));
        }
    }
}
