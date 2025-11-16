use crate::{
    actor::{Action, EscapeActor},
    charset::CharsetIndex,
};
use log::debug;
use std::fmt;

/// Enumeration of the C0/C1 control codes that may be observed outside of an
/// escape sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControlCode {
    // C0
    /// (BEL, Caret = ^G, C = \a) Bell, triggers the bell, buzzer, or beeper on the terminal.
    Bell,
    /// (BS, Caret = ^H, C = \b) Backspace, can be used to define overstruck characters.
    Backspace,
    /// (HT, Caret = ^I, C = \t) Horizontal Tabulation, move to next predetermined position.
    HorizontalTab,
    /// (LF, Caret = ^J, C = \n) Linefeed, move to same position on next line (see also NL).
    LineFeed,
    /// (VT, Caret = ^K, C = \v) Vertical Tabulation, move to next predetermined line.
    VerticalTab,
    /// (FF, Caret = ^L, C = \f) Form Feed, move to next form or page.
    FormFeed,
    /// (CR, Caret = ^M, C = \r) Carriage Return, move to first character of current line.
    CarriageReturn,
    /// (SO, Caret = ^N) Shift Out, switch to G1 (other half of character set).
    ShiftOut,
    /// (SI, Caret = ^O) Shift In, switch to G0 (normal half of character set).
    ShiftIn,
    /// (SUB Caret = ^Z) Indicates that a character has been substituted for one that was found to be invalid or in error.
    Substitute,

    // C1
    /// (IND) Index.
    Index,
    /// (NEL) Next Line.
    NextLine,
    /// (HTS) Horizontal Tabulation Set.
    HorizontalTabSet,

    // Misc
    /// Unexpected control code
    Unexpected(u8),
}

impl From<u8> for ControlCode {
    fn from(byte: u8) -> Self {
        use ControlCode::*;
        match byte {
            // C0
            // 0x00 => Null,
            0x07 => Bell,
            0x08 => Backspace,
            0x09 => HorizontalTab,
            0x0A => LineFeed,
            0x0B => VerticalTab,
            0x0C => FormFeed,
            0x0D => CarriageReturn,
            0x0E => ShiftOut,
            0x0F => ShiftIn,
            0x1A => Substitute,

            // C1
            0x84 => Index,
            0x85 => NextLine,
            0x88 => HorizontalTabSet,

            // Misc
            other => Unexpected(other),
        }
    }
}

impl fmt::Display for ControlCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ControlCode::*;
        let code = match self {
            // C0
            Bell => "BEL",
            Backspace => "BS",
            HorizontalTab => "HT",
            LineFeed => "LF",
            VerticalTab => "VT",
            FormFeed => "FF",
            CarriageReturn => "CR",
            ShiftOut => "SO",
            ShiftIn => "SI",
            Substitute => "SUB",

            // C1
            Index => "IND",
            NextLine => "NEL",
            HorizontalTabSet => "HTS",

            // Misc
            Unexpected(_) => "UNEXPECTED",
        };

        match self {
            Unexpected(b) => write!(f, "{code}: 0x{:02X}", b),
            _ => write!(f, "{code}"),
        }
    }
}

pub(crate) fn perform<A: EscapeActor>(byte: u8, actor: &mut A) {
    let code = ControlCode::from(byte);
    match code {
        // C0
        ControlCode::HorizontalTab => actor.handle(Action::InsertTabs(1)),
        ControlCode::Backspace => actor.handle(Action::Backspace),
        ControlCode::CarriageReturn => actor.handle(Action::CarriageReturn),
        ControlCode::LineFeed
        | ControlCode::FormFeed
        | ControlCode::VerticalTab => actor.handle(Action::LineFeed),
        ControlCode::Bell => actor.handle(Action::Bell),
        ControlCode::Substitute => actor.handle(Action::Substitute),
        ControlCode::ShiftOut => {
            actor.handle(Action::SetActiveCharsetIndex(CharsetIndex::G1))
        },
        ControlCode::ShiftIn => {
            actor.handle(Action::SetActiveCharsetIndex(CharsetIndex::G0))
        },

        // C1
        ControlCode::Index => actor.handle(Action::LineFeed),
        ControlCode::NextLine => actor.handle(Action::NextLine),
        ControlCode::HorizontalTabSet => actor.handle(Action::SetHorizontalTab),
        _ => debug!("[unexpected: control_code] {code}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EscapeParser, charset::CharsetIndex, parser::Parser};

    #[derive(Default)]
    struct RecordingActor {
        actions: Vec<Action>,
    }

    impl EscapeActor for RecordingActor {
        fn handle(&mut self, action: Action) {
            self.actions.push(action);
        }
    }

    impl RecordingActor {
        fn parse(bytes: &[u8]) -> Self {
            let mut parser: Parser<otty_vte::Parser> = Parser::new();
            let mut actor = Self::default();
            parser.advance(bytes, &mut actor);
            actor
        }
    }

    #[test]
    fn perform_test() {
        let cases = vec![
            ("\t".as_bytes(), vec![Action::InsertTabs(1)]),
            ("\x08".as_bytes(), vec![Action::Backspace]),
            ("\r".as_bytes(), vec![Action::CarriageReturn]),
            (
                &[0x0A, 0x0B, 0x0C],
                vec![Action::LineFeed, Action::LineFeed, Action::LineFeed],
            ),
            (
                "\x07\x1A".as_bytes(),
                vec![Action::Bell, Action::Substitute],
            ),
            (
                "\x0E\x0F".as_bytes(),
                vec![
                    Action::SetActiveCharsetIndex(CharsetIndex::G1),
                    Action::SetActiveCharsetIndex(CharsetIndex::G0),
                ],
            ),
            (
                &[0x84, 0x85, 0x88],
                vec![
                    Action::LineFeed,
                    Action::NextLine,
                    Action::SetHorizontalTab,
                ],
            ),
            ("\x01".as_bytes(), vec![]),
            (
                "A\x08B\x0A".as_bytes(),
                vec![
                    Action::Print('A'),
                    Action::Backspace,
                    Action::Print('B'),
                    Action::LineFeed,
                ],
            ),
        ];

        for (input, expected) in cases {
            let actual = RecordingActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }
}
