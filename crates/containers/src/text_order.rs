//! The order of names and other text shown to the user (spec Decision 19).

use std::cmp::Ordering;

/// Compare two texts as a person reads them: A to Z, whatever the case. Texts
/// that differ only in case keep a fixed order, so nothing jumps between
/// rebuilds. Nothing is allocated: tables are long and rebuilt often.
///
/// ```
/// use std::cmp::Ordering;
///
/// use otty_containers::compare_text;
///
/// assert_eq!(compare_text("apple", "Banana"), Ordering::Less);
/// assert_eq!(compare_text("Web", "web"), Ordering::Less);
/// ```
pub fn compare_text(a: &str, b: &str) -> Ordering {
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
        .then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_compares_ignoring_case() {
        assert_eq!(compare_text("apple", "Banana"), Ordering::Less);
        assert_eq!(compare_text("Banana", "apple"), Ordering::Greater);
        assert_eq!(compare_text("Ёлка", "ёж"), Ordering::Greater);
    }

    #[test]
    fn text_that_differs_only_in_case_keeps_a_fixed_order() {
        assert_eq!(compare_text("Web", "web"), Ordering::Less);
        assert_eq!(compare_text("web", "Web"), Ordering::Greater);
        assert_eq!(compare_text("web", "web"), Ordering::Equal);
    }
}
