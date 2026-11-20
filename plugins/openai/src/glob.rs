//! Globs over the paths of a profile: a config folder can be any folder
//! `CODEX_HOME` names, so its path goes into a glob escaped.

/// The characters a glob reads as a pattern.
const PATTERN_CHARACTERS: [char; 7] = ['*', '?', '[', ']', '{', '}', '\\'];

/// `path` with every character a glob reads as a pattern escaped by a
/// backslash, so that the glob matches the path itself.
pub(crate) fn escaped(path: &str) -> String {
    let mut text = String::with_capacity(path.len());

    for character in path.chars() {
        if PATTERN_CHARACTERS.contains(&character) {
            text.push('\\');
        }
        text.push(character);
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_path_stays_as_it_is() {
        assert_eq!(escaped("/home/dev/.codex-work"), "/home/dev/.codex-work");
    }

    #[test]
    fn pattern_characters_are_escaped() {
        assert_eq!(
            escaped("/data/[codex]*{x}?\\"),
            "/data/\\[codex\\]\\*\\{x\\}\\?\\\\"
        );
    }
}
