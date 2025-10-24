use crate::{
    Actor,
    charset::{Charset, CharsetIndex},
};
use log::debug;

#[derive(Debug, Clone, PartialEq, Eq)]
enum EscSequence {
    /// IND - Index.  Note that for Vt52 and Windows 10 ANSI consoles,
    /// this is interpreted as CursorUp
    Index,
    /// NEL - Next Line
    NextLine,
    /// HTS - Horizontal Tab Set
    HorizontalTabSet,
    /// RI - Reverse Index – Performs the reverse operation of \n, moves cursor up one line,
    /// maintains horizontal position, scrolls buffer if necessary
    ReverseIndex,
    /// ST - String Terminator
    StringTerminator,
    /// RIS - Full Reset
    FullReset,
    /// DECID - Return Terminal ID (obsolete form of CSI c - aka DA)
    ReturnTerminalId,
    /// DECSC - Save cursor position
    DecSaveCursorPosition,
    /// DECRC - Restore saved cursor position
    DecRestoreCursorPosition,
    /// DECPAM - Application Keypad
    DecApplicationKeyPad,
    /// DECPNM - Normal Keypad
    DecNormalKeyPad,
    /// Designate G0 Character Set – DEC Line Drawing
    DecLineDrawingG0,
    /// Designate G0 Character Set – US ASCII
    AsciiCharacterSetG0,
    /// Designate G1 Character Set – DEC Line Drawing
    DecLineDrawingG1,
    /// Designate G1 Character Set – US ASCII
    AsciiCharacterSetG1,
    /// Designate G2 Character Set – DEC Line Drawing
    DecLineDrawingG2,
    /// Designate G3 Character Set – US ASCII
    AsciiCharacterSetG2,
    /// Designate G3 Character Set – DEC Line Drawing
    DecLineDrawingG3,
    /// Designate G3 Character Set – US ASCII
    AsciiCharacterSetG3,
    /// DECALN https://vt100.net/docs/vt510-rm/DECALN.html
    DecScreenAlignmentDisplay,
    /// Misc sequences
    Unspecified { control: u8, intermediates: Vec<u8> },
}

impl From<(&[u8], u8)> for EscSequence {
    fn from(value: (&[u8], u8)) -> Self {
        let (intermediates, control) = value;

        match (intermediates, control) {
            ([], b'D') => EscSequence::Index,
            ([], b'E') => EscSequence::NextLine,
            ([], b'H') => EscSequence::HorizontalTabSet,
            ([], b'M') => EscSequence::ReverseIndex,
            ([], b'\\') => EscSequence::StringTerminator,
            ([], b'c') => EscSequence::FullReset,
            ([], b'Z') => EscSequence::ReturnTerminalId,
            ([], b'7') => EscSequence::DecSaveCursorPosition,
            ([], b'8') => EscSequence::DecRestoreCursorPosition,
            ([], b'=') => EscSequence::DecApplicationKeyPad,
            ([], b'>') => EscSequence::DecNormalKeyPad,
            ([b'('], b'0') => EscSequence::DecLineDrawingG0,
            ([b')'], b'0') => EscSequence::DecLineDrawingG1,
            ([b'*'], b'0') => EscSequence::DecLineDrawingG2,
            ([b'+'], b'0') => EscSequence::DecLineDrawingG3,
            ([b'('], b'B') => EscSequence::AsciiCharacterSetG0,
            ([b')'], b'B') => EscSequence::AsciiCharacterSetG1,
            ([b'*'], b'B') => EscSequence::AsciiCharacterSetG2,
            ([b'+'], b'B') => EscSequence::AsciiCharacterSetG3,
            ([b'#'], b'8') => EscSequence::DecScreenAlignmentDisplay,
            _ => EscSequence::Unspecified {
                control,
                intermediates: intermediates.to_vec(),
            },
        }
    }
}

pub(crate) fn perform<A: Actor>(actor: &mut A, intermediates: &[u8], byte: u8) {
    match EscSequence::from((intermediates, byte)) {
        EscSequence::Index => actor.linefeed(),
        EscSequence::NextLine => {
            actor.linefeed();
            actor.carriage_return();
        },
        EscSequence::HorizontalTabSet => actor.set_horizontal_tab(),
        EscSequence::ReverseIndex => actor.reverse_index(),
        EscSequence::ReturnTerminalId => actor.identify_terminal(None),
        EscSequence::FullReset => actor.reset_state(),
        EscSequence::DecSaveCursorPosition => actor.save_cursor_position(),
        EscSequence::DecScreenAlignmentDisplay => {
            actor.screen_alignment_display()
        },
        EscSequence::DecRestoreCursorPosition => {
            actor.restore_cursor_position()
        },
        EscSequence::DecApplicationKeyPad => {
            actor.set_keypad_application_mode()
        },
        EscSequence::DecNormalKeyPad => actor.unset_keypad_application_mode(),
        EscSequence::DecLineDrawingG0 => {
            actor.configure_charset(Charset::DecLineDrawing, CharsetIndex::G0)
        },
        EscSequence::DecLineDrawingG1 => {
            actor.configure_charset(Charset::DecLineDrawing, CharsetIndex::G1)
        },
        EscSequence::DecLineDrawingG2 => {
            actor.configure_charset(Charset::DecLineDrawing, CharsetIndex::G2)
        },
        EscSequence::DecLineDrawingG3 => {
            actor.configure_charset(Charset::DecLineDrawing, CharsetIndex::G3)
        },
        EscSequence::AsciiCharacterSetG0 => {
            actor.configure_charset(Charset::Ascii, CharsetIndex::G0)
        },
        EscSequence::AsciiCharacterSetG1 => {
            actor.configure_charset(Charset::Ascii, CharsetIndex::G1)
        },
        EscSequence::AsciiCharacterSetG2 => {
            actor.configure_charset(Charset::Ascii, CharsetIndex::G2)
        },
        EscSequence::AsciiCharacterSetG3 => {
            actor.configure_charset(Charset::Ascii, CharsetIndex::G3)
        },
        // do nothing
        EscSequence::StringTerminator => {},
        EscSequence::Unspecified {
            control,
            intermediates,
        } => println!(
            "[unexpected: esc] control: {:02X} intermediates: {:?}",
            control, intermediates
        ),
    }
}
