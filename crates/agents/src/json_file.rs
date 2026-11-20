//! The rule of the agents for whether a text in their files is filled.

/// The text without the spaces around it, when it is there and not blank.
///
/// This is the one rule of the agents for whether a key, a token or a name
/// in their files is filled.
pub(crate) fn filled(text: Option<&str>) -> Option<&str> {
    let text = text?.trim();

    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filled_text_is_trimmed_and_a_blank_one_is_none() {
        assert_eq!(filled(Some(" sk-made-up ")), Some("sk-made-up"));
        assert_eq!(filled(Some("  ")), None);
        assert_eq!(filled(Some("")), None);
        assert_eq!(filled(None), None);
    }
}
