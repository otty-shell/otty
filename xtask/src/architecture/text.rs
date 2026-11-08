//! Small helpers for reading Rust code as text: words, comments, braces and
//! statements that span several lines.

use super::source::Line;

/// Whether `ch` can be part of a Rust identifier.
pub(crate) fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// The byte offset of the first `word` in `text` that is a whole word, not
/// part of a longer identifier.
pub(crate) fn find_word(text: &str, word: &str) -> Option<usize> {
    let mut from = 0;

    while let Some(found) = text[from..].find(word) {
        let start = from + found;
        let end = start + word.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if !before.is_some_and(is_ident_char)
            && !after.is_some_and(is_ident_char)
        {
            return Some(start);
        }
        from = end;
    }

    None
}

/// Whether `text` contains `word` as a whole word.
pub(crate) fn contains_word(text: &str, word: &str) -> bool {
    find_word(text, word).is_some()
}

/// Whether the byte offset `at` lies inside a string literal of `text`.
pub(crate) fn is_in_string(text: &str, at: usize) -> bool {
    let mut in_string = false;
    let mut escaped = false;

    for ch in text[..at].chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            _ => {},
        }
    }

    in_string
}

/// `line` without a trailing `// ...` comment that is outside strings.
pub(crate) fn strip_trailing_comment(line: &str) -> &str {
    let mut from = 0;

    while let Some(found) = line[from..].find("//") {
        let at = from + found;
        if !is_in_string(line, at) {
            return line[..at].trim_end();
        }
        from = at + 2;
    }

    line.trim_end()
}

/// The number of leading spaces of `line`.
pub(crate) fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Opening minus closing braces of `text`, outside strings.
pub(crate) fn brace_balance(text: &str) -> i64 {
    let mut balance = 0;

    for (at, ch) in text.char_indices() {
        if (ch == '{' || ch == '}') && !is_in_string(text, at) {
            balance += if ch == '{' { 1 } else { -1 };
        }
    }

    balance
}

/// Join the statement that starts at `lines[start]` up to its closing `;`,
/// and return it with the index of the next line.
pub(crate) fn join_statement(lines: &[Line], start: usize) -> (String, usize) {
    let mut joined = String::new();
    let mut index = start;

    while index < lines.len() {
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(lines[index].text().trim());
        index += 1;

        if joined.ends_with(';') && brace_balance(&joined) <= 0 {
            break;
        }
    }

    (joined, index)
}

/// The index of the line that closes the block opened by the `{` that ends
/// `lines[start]`, also when that line starts with `} else`.
pub(crate) fn block_end(lines: &[Line], start: usize) -> usize {
    let mut balance = 1;

    for (index, line) in lines.iter().enumerate().skip(start + 1) {
        let text = line.text().trim_start();
        if balance == 1 && text.starts_with('}') {
            return index;
        }

        balance += brace_balance(text);
        if balance <= 0 {
            return index;
        }
    }

    lines.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(texts: &[&str]) -> Vec<Line> {
        texts
            .iter()
            .enumerate()
            .map(|(index, text)| Line::new(index + 1, text))
            .collect()
    }

    #[test]
    fn a_word_is_found_only_as_a_whole_identifier() {
        assert_eq!(find_word("&app.widgets", "app"), Some(1));
        assert_eq!(find_word("AppEvent::Tabs", "app"), None);
        assert_eq!(find_word("snap_app(x)", "app"), None);
        assert!(contains_word("route(app, event)", "app"));
    }

    #[test]
    fn a_trailing_comment_is_cut_but_not_inside_a_string() {
        assert_eq!(strip_trailing_comment("let x = 1; // note"), "let x = 1;");
        assert_eq!(
            strip_trailing_comment("let s = \"a // b\";"),
            "let s = \"a // b\";"
        );
    }

    #[test]
    fn braces_inside_strings_do_not_count() {
        assert_eq!(brace_balance("match x { \"}\""), 1);
        assert_eq!(brace_balance("{ }"), 0);
    }

    #[test]
    fn a_statement_is_joined_up_to_its_semicolon() {
        let lines = lines(&["let x = foo(", "    a,", ");", "next();"]);

        assert_eq!(
            join_statement(&lines, 0),
            ("let x = foo( a, );".to_string(), 3)
        );
    }

    #[test]
    fn a_let_else_statement_ends_after_its_else_block() {
        let lines = lines(&[
            "let Some(x) = y",
            "else {",
            "    return Task::none();",
            "};",
            "after();",
        ]);

        assert_eq!(join_statement(&lines, 0).1, 4);
    }

    #[test]
    fn a_block_ends_at_its_closing_brace() {
        let lines = lines(&[
            "if x {",
            "    if y {",
            "    }",
            "} else {",
            "    z();",
            "}",
        ]);

        assert_eq!(block_end(&lines, 0), 3);
        assert_eq!(block_end(&lines, 1), 2);
        assert_eq!(block_end(&lines, 3), 5);
    }
}
