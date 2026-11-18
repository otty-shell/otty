//! Whether an agent's program can be started, found the way the agent tab
//! starts it: through the user's login shell.

use std::cell::RefCell;
use std::collections::HashMap;

use otty_app_shell::quote_word;

use crate::login_shell::{SHELL_TIMEOUT, ask_login_shell};

/// Finds programs for one read of the profiles, and remembers what it
/// found, so the user's shell starts once per program.
pub(crate) struct ProgramLookup<'a> {
    shell: &'a str,
    found: RefCell<HashMap<String, bool>>,
}

impl<'a> ProgramLookup<'a> {
    /// A lookup through the user's shell at `shell`, such as `/bin/zsh`.
    pub(crate) fn new(shell: &'a str) -> Self {
        Self {
            shell,
            found: RefCell::new(HashMap::new()),
        }
    }

    /// Whether the program named `program` can be started: the user's
    /// login shell finds it. A shell that can not start, fails or takes
    /// too long finds nothing.
    pub(crate) fn is_found(&self, program: &str) -> bool {
        if let Some(found) = self.found.borrow().get(program) {
            return *found;
        }

        let script = format!("command -v {}", quote_word(program));
        let found =
            ask_login_shell(self.shell, &script, SHELL_TIMEOUT).is_some();

        self.found.borrow_mut().insert(program.to_string(), found);
        found
    }
}

#[cfg(all(test, unix))]
mod tests {
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{fake_shell, starts};

    #[test]
    fn a_name_is_found_by_the_login_shell() {
        let folder = TempDir::new("lookup-name");
        let shell = fake_shell(&folder);
        let lookup = ProgramLookup::new(&shell);

        assert!(lookup.is_found("sh"));
        assert!(!lookup.is_found("otty-no-such-agent-program"));
    }

    #[test]
    fn each_name_is_asked_once_per_read() {
        let folder = TempDir::new("lookup-once");
        let shell = fake_shell(&folder);
        let lookup = ProgramLookup::new(&shell);

        assert!(lookup.is_found("sh"));
        assert!(lookup.is_found("sh"));

        assert_eq!(starts(&folder), 1);
    }

    #[test]
    fn a_shell_that_can_not_start_finds_nothing() {
        let lookup = ProgramLookup::new("/no/such/shell");

        assert!(!lookup.is_found("sh"));
    }
}
