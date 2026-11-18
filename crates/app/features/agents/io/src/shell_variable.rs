//! The value of an environment variable as the user's login shell sees it.
//!
//! A variable set in `~/.zshrc` is missing from OTTY's own environment
//! when OTTY was started from the desktop. So it is asked of the user's
//! login shell, the same way a program is found (see `program_lookup`).
//! The value may be a secret, such as an admin key: it is never logged.

use otty_agents::AdminKeyRef;
use otty_app_shell::quote_word;

use crate::login_shell::{SHELL_TIMEOUT, ask_login_shell};

/// The line the shell prints right before the value. The start files of an
/// interactive shell may print a greeting first; the value is what follows
/// this line.
const VALUE_MARK: &str = "otty-variable-value-follows";

/// The value of the variable `name` as the user's login shell at `shell`
/// sees it. `None` when `name` is not a variable name (the shell is not
/// started then), the variable is not set, or the shell can not start,
/// fails or takes too long. Only the shell and the name are ever logged,
/// never what the shell printed.
pub(crate) fn shell_variable(shell: &str, name: &str) -> Option<String> {
    if !AdminKeyRef::is_variable_name(name) {
        log::warn!("not reading a variable whose name is not valid");
        return None;
    }

    let script = format!(
        "printf '%s\\n' {}; printenv {}",
        quote_word(VALUE_MARK),
        quote_word(name)
    );
    let printed = ask_login_shell(shell, &script, SHELL_TIMEOUT)?;

    value_after_mark(printed)
}

/// The value the shell printed after [`VALUE_MARK`], without the line end
/// `printenv` adds; `None` when the mark is missing or the text is not
/// UTF-8.
fn value_after_mark(text: Vec<u8>) -> Option<String> {
    let text = String::from_utf8(text).ok()?;
    let (_, value) = text.split_once(&format!("{VALUE_MARK}\n"))?;
    let value = value.strip_suffix('\n').unwrap_or(value);

    Some(value.to_string())
}

#[cfg(all(test, unix))]
mod tests {
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{executable_script, fake_shell, starts};

    /// A made-up login shell whose start files print a greeting and set
    /// `OTTY_TEST_ADMIN_KEY`, as a `~/.zshrc` may; it runs the script with
    /// `/bin/sh`, so the user's own start files are never read.
    fn shell_with_variable(folder: &TempDir) -> String {
        let body = "[ \"$1\" = -l ] && [ \"$2\" = -i ] && [ \"$3\" = -c ] \
                    || exit 2\n\
                    printf 'Welcome back'\n\
                    export OTTY_TEST_ADMIN_KEY=' made-up key '\n\
                    exec /bin/sh -c \"$4\"";

        executable_script(folder.path(), "zsh", body)
            .display()
            .to_string()
    }

    #[test]
    fn a_variable_set_in_the_start_files_of_the_shell_is_read() {
        let folder = TempDir::new("variable-set");
        let shell = shell_with_variable(&folder);

        let value = shell_variable(&shell, "OTTY_TEST_ADMIN_KEY");

        assert_eq!(value.as_deref(), Some(" made-up key "));
    }

    #[test]
    fn an_unset_variable_has_no_value() {
        let folder = TempDir::new("variable-unset");
        let shell = fake_shell(&folder);

        let value = shell_variable(&shell, "OTTY_TEST_NO_SUCH_VARIABLE");

        assert_eq!(value, None);
        assert_eq!(starts(&folder), 1);
    }

    #[test]
    fn a_wrong_name_never_reaches_the_shell() {
        let folder = TempDir::new("variable-wrong-name");
        let shell = fake_shell(&folder);
        let touched = folder.path().join("touched");
        let name = format!("KEY; touch {}", touched.display());

        let value = shell_variable(&shell, &name);

        assert_eq!(value, None);
        assert_eq!(starts(&folder), 0, "the shell is not started");
        assert!(!touched.exists());
    }

    #[test]
    fn a_shell_that_can_not_start_gives_nothing() {
        let value = shell_variable("/no/such/shell", "HOME");

        assert_eq!(value, None);
    }
}
