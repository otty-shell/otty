use crate::{
    Actor,
    actor::Action,
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
        EscSequence::Index => actor.handle(Action::LineFeed),
        EscSequence::NextLine => {
            actor.handle(Action::NextLine)
            // TODO: remove after integrate
            // actor.linefeed();
            // actor.carriage_return();
        },
        EscSequence::HorizontalTabSet => actor.handle(Action::SetHorizontalTab),
        EscSequence::ReverseIndex => actor.handle(Action::ReverseIndex),
        EscSequence::ReturnTerminalId => {
            actor.handle(Action::IdentifyTerminal(None))
        },
        EscSequence::FullReset => actor.handle(Action::ResetState),
        EscSequence::DecSaveCursorPosition => {
            actor.handle(Action::SaveCursorPosition)
        },
        EscSequence::DecScreenAlignmentDisplay => {
            actor.handle(Action::ScreenAlignmentDisplay)
        },
        EscSequence::DecRestoreCursorPosition => {
            actor.handle(Action::RestoreCursorPosition)
        },
        EscSequence::DecApplicationKeyPad => {
            actor.handle(Action::SetKeypadApplicationMode)
        },
        EscSequence::DecNormalKeyPad => {
            actor.handle(Action::UnsetKeypadApplicationMode)
        },
        EscSequence::DecLineDrawingG0 => actor.handle(
            Action::ConfigureCharset(Charset::DecLineDrawing, CharsetIndex::G0),
        ),
        EscSequence::DecLineDrawingG1 => actor.handle(
            Action::ConfigureCharset(Charset::DecLineDrawing, CharsetIndex::G1),
        ),
        EscSequence::DecLineDrawingG2 => actor.handle(
            Action::ConfigureCharset(Charset::DecLineDrawing, CharsetIndex::G2),
        ),
        EscSequence::DecLineDrawingG3 => actor.handle(
            Action::ConfigureCharset(Charset::DecLineDrawing, CharsetIndex::G3),
        ),
        EscSequence::AsciiCharacterSetG0 => actor
            .handle(Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G0)),
        EscSequence::AsciiCharacterSetG1 => actor
            .handle(Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G1)),
        EscSequence::AsciiCharacterSetG2 => actor
            .handle(Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G2)),
        EscSequence::AsciiCharacterSetG3 => actor
            .handle(Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G3)),
        // do nothing
        EscSequence::StringTerminator => {},
        EscSequence::Unspecified {
            control,
            intermediates,
        } => debug!(
            "[unexpected: esc] control: {:02X} intermediates: {:?}",
            control, intermediates
        ),
    }
}
