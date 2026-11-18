//! Commands run by the user's login shell.
//!
//! A program installed by a version manager, such as Codex under `nvm`, is
//! on `PATH` only after the user's shell has read its start files. When OTTY
//! was started from the desktop, its own `PATH` lacks those folders. So a
//! program OTTY starts by name runs through the user's shell, started as a
//! login and interactive shell: a login shell reads `~/.zprofile` or
//! `~/.bash_profile`, an interactive one reads `~/.zshrc` or `~/.bashrc`,
//! where version managers add their folders.

/// Return the arguments that make the user's shell run `script` as a login
/// and interactive shell.
///
/// # Example
///
/// ```
/// use otty_app_shell::login_shell_args;
///
/// assert_eq!(
///     login_shell_args("command -v 'codex'"),
///     ["-l", "-i", "-c", "command -v 'codex'"]
/// );
/// ```
pub fn login_shell_args(script: &str) -> Vec<String> {
    ["-l", "-i", "-c", script].map(String::from).to_vec()
}

/// Return `word` quoted for the shell, so the shell reads it as one word
/// and changes nothing in it.
///
/// # Example
///
/// ```
/// use otty_app_shell::quote_word;
///
/// assert_eq!(quote_word("codex"), "'codex'");
/// assert_eq!(quote_word("it's"), r"'it'\''s'");
/// ```
pub fn quote_word(word: &str) -> String {
    // Inside single quotes the shell changes nothing; a single quote itself
    // ends the quotes, is added escaped, and starts them again.
    let escaped = word.replace('\'', r"'\''");

    format!("'{escaped}'")
}

/// Return the script that replaces the shell with `program` run with
/// `args`, each quoted: `exec 'codex' 'resume' 'a1'`.
///
/// # Example
///
/// ```
/// use otty_app_shell::exec_script;
///
/// let args = [String::from("resume"), String::from("a1")];
///
/// assert_eq!(exec_script("codex", &args), "exec 'codex' 'resume' 'a1'");
/// ```
pub fn exec_script(program: &str, args: &[String]) -> String {
    let mut script = format!("exec {}", quote_word(program));
    for arg in args {
        script.push(' ');
        script.push_str(&quote_word(arg));
    }

    script
}

/// Return the line a user types in a shell to run `program` with `args`
/// and the variables `env`, each value quoted:
/// `CODEX_HOME='/home/user/.codex-work' 'codex' 'resume' 'a1'`.
///
/// # Example
///
/// ```
/// use otty_app_shell::command_line;
///
/// let args = [String::from("--resume"), String::from("a1")];
///
/// assert_eq!(
///     command_line("claude", &args, &[]),
///     "'claude' '--resume' 'a1'"
/// );
/// ```
pub fn command_line(
    program: &str,
    args: &[String],
    env: &[(String, String)],
) -> String {
    let variables = env
        .iter()
        .map(|(name, value)| format!("{name}={}", quote_word(value)));
    let words = std::iter::once(program)
        .chain(args.iter().map(String::as_str))
        .map(quote_word);

    variables.chain(words).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn the_script_runs_in_a_login_and_interactive_shell() {
        assert_eq!(login_shell_args("true"), ["-l", "-i", "-c", "true"]);
    }

    #[test]
    fn a_word_is_quoted_whole_and_its_own_quotes_are_kept() {
        assert_eq!(quote_word("codex"), "'codex'");
        assert_eq!(quote_word("a b"), "'a b'");
        assert_eq!(quote_word("it's"), r"'it'\''s'");
        assert_eq!(quote_word(""), "''");
    }

    #[test]
    fn the_command_line_sets_the_variables_then_quotes_the_program_and_each_argument()
     {
        let args = [String::from("resume"), String::from("a1")];
        let env = [(
            String::from("CODEX_HOME"),
            String::from("/home/user/my codex"),
        )];

        assert_eq!(
            command_line("codex", &args, &env),
            "CODEX_HOME='/home/user/my codex' 'codex' 'resume' 'a1'"
        );
        assert_eq!(command_line("claude", &[], &[]), "'claude'");
    }

    #[test]
    fn the_exec_script_quotes_the_program_and_each_argument() {
        let args = [String::from("resume"), String::from("a1")];

        assert_eq!(exec_script("codex", &args), "exec 'codex' 'resume' 'a1'");
        assert_eq!(exec_script("claude", &[]), "exec 'claude'");
    }

    /// A shell gives the program every argument as it was, with spaces,
    /// quotes and `$` in it.
    #[cfg(unix)]
    #[test]
    fn a_shell_runs_the_exec_script_with_the_arguments_unchanged() {
        let args = ["%s|", "it's", "a b", "$HOME"].map(String::from);
        let script = exec_script("printf", &args);

        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("sh should run");

        assert_eq!(String::from_utf8_lossy(&output.stdout), "it's|a b|$HOME|");
    }
}
