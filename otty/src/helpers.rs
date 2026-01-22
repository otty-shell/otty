const DEFAULT_MAX_CHAR_COUNT_BEFORE_ELIPSIZE: usize = 20;

pub fn ellipsize(s: &str) -> String {
    let total = s.chars().count();
    if total <= DEFAULT_MAX_CHAR_COUNT_BEFORE_ELIPSIZE {
        return s.to_owned();
    }

    let keep = DEFAULT_MAX_CHAR_COUNT_BEFORE_ELIPSIZE - 2;
    let tail: String = s
        .chars()
        .rev()
        .take(keep)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!("..{}", tail)
}
