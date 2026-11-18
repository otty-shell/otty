//! Reads the session files of the agents line by line: each line is one
//! JSON record, and most lines are looked at only for a marker before they
//! are parsed.

use std::io::{BufRead, BufReader, Read};

/// Calls `each_line` with every line read from `reader`, with its newline
/// when it has one.
///
/// Reading stops at the end or at the first error, so a file that breaks
/// in the middle gives the lines read before.
pub(crate) fn for_each_line(
    reader: impl Read,
    mut each_line: impl FnMut(&[u8]),
) {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();

    while matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
        each_line(&line);
        line.clear();
    }
}

/// Whether `marker` is somewhere in `line`.
///
/// It tells cheaply whether a line can hold the wanted record, so that
/// only such lines are parsed.
pub(crate) fn has_marker(line: &[u8], marker: &[u8]) -> bool {
    line.windows(marker.len()).any(|part| part == marker)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_line_is_given_the_last_one_even_with_no_newline() {
        let mut lines = Vec::new();

        for_each_line(&b"{\"a\":1}\n\n{\"b\":2}"[..], |line| {
            lines.push(line.to_vec());
        });

        assert_eq!(lines, [&b"{\"a\":1}\n"[..], b"\n", b"{\"b\":2}"]);
    }

    #[test]
    fn a_marker_is_found_anywhere_in_the_line() {
        let line = br#"{"type":"event_msg","payload":{"type":"token_count"}}"#;

        assert!(has_marker(line, b"token_count"));
        assert!(has_marker(line, b"{\"type\""));
        assert!(!has_marker(line, b"user_message"));
        assert!(!has_marker(b"token", b"token_count"));
    }
}
