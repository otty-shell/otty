//! The name of the shell new terminal tabs start.
//!
//! The session itself (`ShellSession` and its launch options) is built from
//! the terminal widget's settings types, which come with iced, so it lives
//! in `otty-app-shell`.

use std::path::Path;

/// Return the shell's name from its path, such as `zsh` for `/bin/zsh`.
///
/// # Examples
///
/// ```
/// use otty_app_domain::shell_session::shell_name;
///
/// assert_eq!(shell_name("/bin/zsh"), "zsh");
/// ```
pub fn shell_name(shell_path: &str) -> String {
    Path::new(shell_path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| shell_path.to_string())
}

#[cfg(test)]
mod tests {
    use super::shell_name;

    #[test]
    fn given_shell_path_when_named_then_name_is_the_file_name() {
        assert_eq!(shell_name("/bin/sh"), "sh");
        assert_eq!(shell_name("/usr/bin/bash"), "bash");
    }

    #[test]
    fn given_bare_program_when_named_then_name_is_the_program() {
        assert_eq!(shell_name("fish"), "fish");
    }

    #[test]
    fn given_path_without_file_name_when_named_then_name_is_the_path() {
        assert_eq!(shell_name("/"), "/");
    }
}
