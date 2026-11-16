//! The title of a session as the session list shows it.

/// The longest title, in characters, before it is cut.
const MAX_TITLE_CHARS: usize = 100;

/// The first line of `text` that is not blank, trimmed and cut to
/// [`MAX_TITLE_CHARS`] characters. `None` when `text` is blank.
pub(crate) fn title_line(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;

    if line.chars().count() <= MAX_TITLE_CHARS {
        return Some(line.to_owned());
    }

    let cut: String = line.chars().take(MAX_TITLE_CHARS).collect();
    Some(format!("{cut}..."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_is_the_first_line_that_is_not_blank() {
        let title = title_line("\n\n   Rename the panel   \nand its tests");

        assert_eq!(title.as_deref(), Some("Rename the panel"));
    }

    #[test]
    fn a_long_title_is_cut_to_a_readable_length() {
        let title = title_line(&"a".repeat(300));

        assert_eq!(title, Some(format!("{}...", "a".repeat(100))));
    }

    #[test]
    fn blank_text_has_no_title() {
        assert_eq!(title_line(" \n\t\n"), None);
    }
}
