//! Cuts the engine's answer to a logs request into lines, in bounded memory.
//!
//! The answer comes in chunks that fall anywhere. It is either a row of frames
//! (kind, three zero bytes, big-endian length, text) or, for a container with
//! a terminal, plain text. Nothing here waits for a whole frame or for a line
//! break: a stream that never sends a line break holds at most [`CUT_BYTES`].

use std::mem;

use crate::logs::MAX_LOG_LINE_BYTES;

/// The most bytes of a character that can wait for the rest of it: a character
/// has at most four bytes.
pub(crate) const MAX_WAITING_CHAR_BYTES: usize = 3;
/// Where a run without a line break is cut. It leaves room for the start of a
/// character that waited, so normal text stays within
/// [`MAX_LOG_LINE_BYTES`] without one more cut.
const CUT_BYTES: usize = MAX_LOG_LINE_BYTES - MAX_WAITING_CHAR_BYTES;
/// The bytes ahead of the text of a frame.
const FRAME_HEADER_BYTES: usize = 8;
/// The frame kinds of the output a container wrote. Kind 0 is its input, and
/// any first byte above [`STDERR_FRAME`] says the answer is plain text.
const STDOUT_FRAME: u8 = 1;
const STDERR_FRAME: u8 = 2;

/// Where a line of the answer came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputKind {
    Stdout,
    Stderr,
}

/// One line of the answer, as bytes and still with its time.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AnswerLine {
    bytes: Vec<u8>,
    kind: OutputKind,
    is_continuation: bool,
    is_open: bool,
}

impl AnswerLine {
    /// Which output the line belongs to.
    pub(crate) fn kind(&self) -> OutputKind {
        self.kind
    }

    /// Whether it goes on a line that was cut, so it starts with text and not
    /// with a time.
    pub(crate) fn is_continuation(&self) -> bool {
        self.is_continuation
    }

    /// Whether no line break ended it, so its last character may go on in the
    /// next line of the same output.
    pub(crate) fn is_open(&self) -> bool {
        self.is_open
    }

    /// The bytes, taken out of the line.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
impl AnswerLine {
    /// A whole line of stdout, as an engine sends it.
    pub(crate) fn whole(text: &str) -> Self {
        Self {
            bytes: text.as_bytes().to_vec(),
            kind: OutputKind::Stdout,
            is_continuation: false,
            is_open: false,
        }
    }
}

/// The frame whose text is being read.
#[derive(Debug, Clone, Copy)]
struct Frame {
    /// `None` for what is not output: the text is skipped.
    kind: Option<OutputKind>,
    bytes_left: usize,
}

/// What ended a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineEnd {
    Break,
    Cut,
    EndOfText,
}

/// Reads one answer, chunk by chunk.
#[derive(Debug, Default)]
pub(crate) struct LogAnswer {
    /// `None` until the first byte tells.
    is_plain: Option<bool>,
    /// The bytes of a frame header that came so far.
    header: Vec<u8>,
    frame: Option<Frame>,
    /// The line that has no end yet, at most [`CUT_BYTES`].
    line: Vec<u8>,
    /// The line goes on one that was cut.
    is_continuation: bool,
}

impl LogAnswer {
    /// The lines that `chunk`, the next part of the answer, completes.
    pub(crate) fn read(&mut self, chunk: &[u8]) -> Vec<AnswerLine> {
        let mut lines = Vec::new();

        let Some(first) = chunk.first() else {
            return lines;
        };
        let is_plain = *self.is_plain.get_or_insert(*first > STDERR_FRAME);
        if is_plain {
            self.read_text(chunk, OutputKind::Stdout, &mut lines);
            return lines;
        }

        let mut rest = chunk;
        while !rest.is_empty() {
            rest = match self.frame {
                Some(frame) => self.read_frame_text(frame, rest, &mut lines),
                None => self.read_header(rest),
            };
        }

        lines
    }

    /// The answer is over: what is left is the last line, without a break.
    pub(crate) fn finish(&mut self) -> Vec<AnswerLine> {
        let mut lines = Vec::new();

        let kind = self.frame.and_then(|frame| frame.kind);
        self.end_text(kind.unwrap_or(OutputKind::Stdout), &mut lines);

        lines
    }

    /// Take the bytes of a frame header from `rest`; returns what follows.
    fn read_header<'a>(&mut self, rest: &'a [u8]) -> &'a [u8] {
        let missing = FRAME_HEADER_BYTES - self.header.len();
        let (part, rest) = rest.split_at(missing.min(rest.len()));
        self.header.extend_from_slice(part);

        if self.header.len() == FRAME_HEADER_BYTES {
            self.frame = Some(frame_of_header(&self.header));
            self.header.clear();
        }

        rest
    }

    /// Take the text of `frame` from `rest`; returns what follows.
    fn read_frame_text<'a>(
        &mut self,
        frame: Frame,
        rest: &'a [u8],
        lines: &mut Vec<AnswerLine>,
    ) -> &'a [u8] {
        let (text, rest) = rest.split_at(frame.bytes_left.min(rest.len()));
        let bytes_left = frame.bytes_left - text.len();

        if let Some(kind) = frame.kind {
            self.read_text(text, kind, lines);
        }

        if bytes_left > 0 {
            self.frame = Some(Frame {
                bytes_left,
                ..frame
            });
            return rest;
        }

        if let Some(kind) = frame.kind {
            self.end_text(kind, lines);
        }
        self.frame = None;

        rest
    }

    /// Add `text` to the open line and end the line at each line break, and
    /// where it would get longer than [`CUT_BYTES`].
    fn read_text(
        &mut self,
        mut text: &[u8],
        kind: OutputKind,
        lines: &mut Vec<AnswerLine>,
    ) {
        while !text.is_empty() {
            let room = CUT_BYTES - self.line.len();
            let line_break =
                text.iter().take(room + 1).position(|byte| *byte == b'\n');

            if let Some(line_break) = line_break {
                self.line.extend_from_slice(&text[..line_break]);
                self.end_line(kind, LineEnd::Break, lines);
                text = &text[line_break + 1..];
            } else if text.len() <= room {
                self.line.extend_from_slice(text);
                return;
            } else {
                self.line.extend_from_slice(&text[..room]);
                self.end_line(kind, LineEnd::Cut, lines);
                text = &text[room..];
            }
        }
    }

    /// The text ended without a line break: what is open is a line of its own.
    fn end_text(&mut self, kind: OutputKind, lines: &mut Vec<AnswerLine>) {
        if !self.line.is_empty() {
            self.end_line(kind, LineEnd::EndOfText, lines);
        }
    }

    fn end_line(
        &mut self,
        kind: OutputKind,
        end: LineEnd,
        lines: &mut Vec<AnswerLine>,
    ) {
        let mut bytes = mem::take(&mut self.line);
        if end != LineEnd::Cut && bytes.last() == Some(&b'\r') {
            bytes.pop();
        }

        lines.push(AnswerLine {
            bytes,
            kind,
            is_continuation: self.is_continuation,
            is_open: end != LineEnd::Break,
        });
        self.is_continuation = end == LineEnd::Cut;
    }
}

/// The frame a whole header announces.
fn frame_of_header(header: &[u8]) -> Frame {
    let kind = match header[0] {
        STDOUT_FRAME => Some(OutputKind::Stdout),
        STDERR_FRAME => Some(OutputKind::Stderr),
        _ => None,
    };
    let length = [header[4], header[5], header[6], header[7]];

    Frame {
        kind,
        bytes_left: u32::from_be_bytes(length) as usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STDIN_FRAME: u8 = 0;

    fn frame(kind: u8, text: &[u8]) -> Vec<u8> {
        let length = u32::try_from(text.len()).expect("short text");
        let mut frame = vec![kind, 0, 0, 0];
        frame.extend_from_slice(&length.to_be_bytes());
        frame.extend_from_slice(text);
        frame
    }

    fn text(line: &AnswerLine) -> String {
        String::from_utf8_lossy(&line.bytes).into_owned()
    }

    fn texts(lines: &[AnswerLine]) -> Vec<String> {
        lines.iter().map(text).collect()
    }

    /// Everything the answer gives for `chunks`, the end included.
    fn read_all(chunks: &[&[u8]]) -> Vec<AnswerLine> {
        let mut answer = LogAnswer::default();
        let mut lines = Vec::new();
        for chunk in chunks {
            lines.extend(answer.read(chunk));
        }
        lines.extend(answer.finish());
        lines
    }

    #[test]
    fn frames_give_their_lines_in_order_with_their_output() {
        let mut body = frame(STDOUT_FRAME, b"starting\nlistening\n");
        body.extend(frame(STDERR_FRAME, b"warning\r\n"));

        let lines = read_all(&[&body]);

        assert_eq!(texts(&lines), ["starting", "listening", "warning"]);
        assert_eq!(lines[1].kind(), OutputKind::Stdout);
        assert_eq!(lines[2].kind(), OutputKind::Stderr);
        assert!(lines.iter().all(|line| !line.is_open()));
    }

    #[test]
    fn a_frame_that_comes_byte_by_byte_gives_the_same_lines() {
        let mut body = frame(STDOUT_FRAME, b"one\ntwo\n");
        body.extend(frame(STDERR_FRAME, b"three\n"));
        let chunks: Vec<&[u8]> = body.chunks(1).collect();

        let lines = read_all(&chunks);

        assert_eq!(texts(&lines), ["one", "two", "three"]);
        assert_eq!(lines[2].kind(), OutputKind::Stderr);
    }

    /// Docker sends a long line as frames without a line break; each has its
    /// own time, so each is a line of its own.
    #[test]
    fn a_frame_without_a_line_break_is_an_open_line() {
        let mut body = frame(STDOUT_FRAME, b"first part");
        body.extend(frame(STDOUT_FRAME, b"second part\n"));

        let lines = read_all(&[&body]);

        assert_eq!(texts(&lines), ["first part", "second part"]);
        assert!(lines[0].is_open());
        assert!(!lines[0].is_continuation());
        assert!(!lines[1].is_open());
        assert!(!lines[1].is_continuation());
    }

    #[test]
    fn what_the_container_read_gives_no_lines() {
        let mut body = frame(STDIN_FRAME, b"typed\n");
        body.extend(frame(STDOUT_FRAME, b"output\n"));

        let lines = read_all(&[&body]);

        assert_eq!(texts(&lines), ["output"]);
    }

    #[test]
    fn an_empty_frame_gives_no_line_and_a_line_break_an_empty_one() {
        let mut body = frame(STDOUT_FRAME, b"");
        body.extend(frame(STDOUT_FRAME, b"\n"));

        let lines = read_all(&[&body]);

        assert_eq!(texts(&lines), [""]);
    }

    #[test]
    fn plain_text_is_cut_at_its_line_breaks_and_the_rest_comes_at_the_end() {
        let lines = read_all(&[b"$ ls\r\nbin  ", b"etc\r\nlast line"]);

        assert_eq!(texts(&lines), ["$ ls", "bin  etc", "last line"]);
        assert!(!lines[1].is_open());
        assert!(lines[2].is_open());
    }

    #[test]
    fn plain_text_that_waits_for_a_line_break_gives_nothing_yet() {
        let mut answer = LogAnswer::default();

        assert!(answer.read(b"no line break yet").is_empty());
        assert_eq!(answer.line.len(), 17);
    }

    /// A container with a terminal that never prints a line break.
    #[test]
    fn plain_text_without_a_line_break_stays_bounded_forever() {
        let chunk = vec![b'x'; 8 * 1024];
        let mut answer = LogAnswer::default();
        let mut bytes = 0;

        for _ in 0..2_000 {
            for line in answer.read(&chunk) {
                assert!(line.bytes.len() <= CUT_BYTES);
                bytes += line.bytes.len();
            }
            assert!(answer.line.len() <= CUT_BYTES);
        }
        for line in answer.finish() {
            bytes += line.bytes.len();
        }

        assert_eq!(bytes, 2_000 * chunk.len());
    }

    /// The header announces more than any buffer should hold.
    #[test]
    fn a_huge_frame_is_read_as_it_comes_and_stays_bounded() {
        let total: usize = 8 * 1024 * 1024;
        let length = u32::try_from(total).expect("fits");
        let mut header = vec![STDOUT_FRAME, 0, 0, 0];
        header.extend_from_slice(&length.to_be_bytes());
        let chunk = vec![b'x'; 16 * 1024];
        let mut answer = LogAnswer::default();
        let mut bytes = 0;

        assert!(answer.read(&header).is_empty());
        for _ in 0..(total / chunk.len()) {
            for line in answer.read(&chunk) {
                bytes += line.bytes.len();
            }
            assert!(answer.line.len() <= CUT_BYTES);
        }

        assert!(answer.frame.is_none());
        assert_eq!(bytes, total);
    }

    #[test]
    fn the_pieces_of_a_cut_line_say_that_they_go_on() {
        let mut long = vec![b'x'; CUT_BYTES * 2 + 5];
        long.push(b'\n');
        long.extend_from_slice(b"next\n");

        let lines = read_all(&[&frame(STDOUT_FRAME, &long)]);

        let shape: Vec<(usize, bool, bool)> = lines
            .iter()
            .map(|line| {
                (line.bytes.len(), line.is_continuation(), line.is_open())
            })
            .collect();
        assert_eq!(
            shape,
            [
                (CUT_BYTES, false, true),
                (CUT_BYTES, true, true),
                (5, true, false),
                (4, false, false),
            ]
        );
    }

    #[test]
    fn a_line_exactly_at_the_cut_gets_no_empty_line_after_it() {
        let mut exact = vec![b'x'; CUT_BYTES];
        exact.push(b'\n');

        let lines = read_all(&[&exact]);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].bytes.len(), CUT_BYTES);
        assert!(!lines[0].is_open());
    }

    #[test]
    fn a_line_break_right_after_a_cut_ends_the_next_piece() {
        let mut body = vec![b'x'; CUT_BYTES + 1];
        body.push(b'\n');

        let lines = read_all(&[&body]);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].bytes.len(), 1);
        assert!(lines[1].is_continuation());
    }

    #[test]
    fn the_line_after_a_frame_that_ended_on_a_cut_starts_afresh() {
        let mut body = frame(STDOUT_FRAME, &vec![b'x'; CUT_BYTES + 1]);
        body.extend(frame(STDOUT_FRAME, b"next\n"));

        let lines = read_all(&[&body]);

        assert_eq!(lines.len(), 3);
        assert!(lines[1].is_continuation());
        assert!(!lines[2].is_continuation());
    }

    #[test]
    fn an_answer_with_nothing_gives_no_lines() {
        assert!(read_all(&[b""]).is_empty());
    }

    /// The connection broke inside a frame: what came of it is not lost.
    #[test]
    fn an_answer_that_ends_inside_a_frame_gives_what_came() {
        let mut body = frame(STDERR_FRAME, b"whole text");
        body.truncate(body.len() - 5);

        let lines = read_all(&[&body]);

        assert_eq!(texts(&lines), ["whole"]);
        assert_eq!(lines[0].kind(), OutputKind::Stderr);
    }
}
