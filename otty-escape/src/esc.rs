use crate::{
    Actor,
    actor::{Action, TerminalControlAction},
    charset::{Charset, CharsetIndex},
};
use log::debug;

pub(crate) fn perform<A: Actor>(actor: &mut A, intermediates: &[u8], byte: u8) {
    match (byte, intermediates) {
        // IND - Index.  Note that for Vt52 and Windows 10 ANSI consoles,
        // this is interpreted as CursorUp
        (b'D', []) => actor.handle(Action::LineFeed),
        // NEL - Next Line
        (b'E', []) => actor.handle(Action::NextLine),
        // HTS - Horizontal Tab Set
        (b'H', []) => actor.handle(Action::SetHorizontalTab),
        // RI - Reverse Index – Performs the reverse operation of \n, moves cursor up one line,
        // maintains horizontal position, scrolls buffer if necessary
        (b'M', []) => actor.handle(Action::ReverseIndex),
        // RIS - Full Reset
        (b'c', []) => actor.handle(Action::ResetState),
        // DECID - Return Terminal ID (obsolete form of CSI c - aka DA)
        (b'Z', []) => {
            actor.handle(TerminalControlAction::IdentifyTerminal(None).into())
        },
        // DECSC - Save cursor position
        (b'7', []) => actor.handle(Action::SaveCursorPosition),
        // DECRC - Restore saved cursor position
        (b'8', []) => actor.handle(Action::RestoreCursorPosition),
        // DECPAM - Application Keypad
        (b'=', []) => {
            actor.handle(TerminalControlAction::SetKeypadApplicationMode.into())
        },
        // DECPNM - Normal Keypad
        (b'>', []) => actor
            .handle(TerminalControlAction::UnsetKeypadApplicationMode.into()),
        // Designate G0 Character Set – DEC Line Drawing
        (b'0', [b'(']) => actor.handle(Action::ConfigureCharset(
            Charset::DecLineDrawing,
            CharsetIndex::G0,
        )),
        // Designate G1 Character Set – DEC Line Drawing
        (b'0', [b')']) => actor.handle(Action::ConfigureCharset(
            Charset::DecLineDrawing,
            CharsetIndex::G1,
        )),
        // Designate G2 Character Set – DEC Line Drawing
        (b'0', [b'*']) => actor.handle(Action::ConfigureCharset(
            Charset::DecLineDrawing,
            CharsetIndex::G2,
        )),
        // Designate G3 Character Set – DEC Line Drawing
        (b'0', [b'+']) => actor.handle(Action::ConfigureCharset(
            Charset::DecLineDrawing,
            CharsetIndex::G3,
        )),
        // Designate G0 Character Set – US ASCII
        (b'B', [b'(']) => actor
            .handle(Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G0)),
        // Designate G1 Character Set – US ASCII
        (b'B', [b')']) => actor
            .handle(Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G1)),
        // Designate G2 Character Set – US ASCII
        (b'B', [b'*']) => actor
            .handle(Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G2)),
        // Designate G3 Character Set – US ASCII
        (b'B', [b'+']) => actor
            .handle(Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G3)),
        // DECALN https://vt100.net/docs/vt510-rm/DECALN.html
        (b'8', [b'#']) => actor.handle(Action::ScreenAlignmentDisplay),
        // ST - String Terminator
        (b'\\', []) => {},
        _ => debug!(
            "[unexpected: esc] control: {:02X} intermediates: {:?}",
            byte, intermediates
        ),
    };
}
