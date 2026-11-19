//! The version of an agent's program from what it prints for `--version`,
//! when no session file names one yet.

/// The version in `output`, what an agent's program printed for
/// `--version`: its first word that starts with a digit, or with `v` and a
/// digit, and has a dot, without the `v`. `None` when there is none.
///
/// ```
/// use otty_agents::version_in_output;
///
/// assert_eq!(
///     version_in_output("2.1.328 (Claude Code)\n"),
///     Some(String::from("2.1.328"))
/// );
/// assert_eq!(
///     version_in_output("codex-cli 0.169.0"),
///     Some(String::from("0.169.0"))
/// );
/// ```
pub fn version_in_output(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .map(|word| word.strip_prefix('v').unwrap_or(word))
        .map(|word| word.trim_end_matches([',', ';', ')']))
        .find(|word| {
            word.starts_with(|first: char| first.is_ascii_digit())
                && word.contains('.')
        })
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_the_first_word_with_digits_and_a_dot() {
        assert_eq!(
            version_in_output("2.1.328 (Claude Code)"),
            Some(String::from("2.1.328"))
        );
        assert_eq!(
            version_in_output("codex-cli 0.169.0\n"),
            Some(String::from("0.169.0"))
        );
        assert_eq!(
            version_in_output("goose v1.12.0,"),
            Some(String::from("1.12.0"))
        );
    }

    #[test]
    fn output_with_no_version_has_none() {
        assert_eq!(version_in_output(""), None);
        assert_eq!(version_in_output("command not found"), None);
        assert_eq!(version_in_output("build 2026 ok"), None);
    }
}
