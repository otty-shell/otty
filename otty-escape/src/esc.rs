use crate::{
    Actor,
    actor::Action,
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
        (b'Z', []) => actor.handle(Action::IdentifyTerminal(None)),
        // DECSC - Save cursor position
        (b'7', []) => actor.handle(Action::SaveCursorPosition),
        // DECRC - Restore saved cursor position
        (b'8', []) => actor.handle(Action::RestoreCursorPosition),
        // DECPAM - Application Keypad
        (b'=', []) => actor.handle(Action::SetKeypadApplicationMode),
        // DECPNM - Normal Keypad
        (b'>', []) => actor.handle(Action::UnsetKeypadApplicationMode),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[derive(Default)]
    struct RecordingActor {
        actions: Vec<Action>,
    }

    impl Actor for RecordingActor {
        fn handle(&mut self, action: Action) {
            self.actions.push(action);
        }
    }

    impl RecordingActor {
        fn parse(input: &str) -> Self {
            let mut parser = Parser::new();
            let mut actor = Self::default();
            parser.advance(input.as_bytes(), &mut actor);
            actor
        }
    }

    #[test]
    fn esc_basic_sequences() {
        let actor = RecordingActor::parse(
            "\x1bD\x1bE\x1bH\x1bM\x1bc\x1bZ\x1b7\x1b8\x1b=\x1b>",
        );

        assert_eq!(
            actor.actions,
            vec![
                Action::LineFeed,
                Action::NextLine,
                Action::SetHorizontalTab,
                Action::ReverseIndex,
                Action::ResetState,
                Action::IdentifyTerminal(None),
                Action::SaveCursorPosition,
                Action::RestoreCursorPosition,
                Action::SetKeypadApplicationMode,
                Action::UnsetKeypadApplicationMode,
            ]
        );
    }

    #[test]
    fn esc_configure_charset_sequences() {
        let actor = RecordingActor::parse(
            "\x1b(0\x1b)0\x1b*0\x1b+0\x1b(B\x1b)B\x1b*B\x1b+B",
        );

        assert_eq!(
            actor.actions,
            vec![
                Action::ConfigureCharset(
                    Charset::DecLineDrawing,
                    CharsetIndex::G0
                ),
                Action::ConfigureCharset(
                    Charset::DecLineDrawing,
                    CharsetIndex::G1
                ),
                Action::ConfigureCharset(
                    Charset::DecLineDrawing,
                    CharsetIndex::G2
                ),
                Action::ConfigureCharset(
                    Charset::DecLineDrawing,
                    CharsetIndex::G3
                ),
                Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G0),
                Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G1),
                Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G2),
                Action::ConfigureCharset(Charset::Ascii, CharsetIndex::G3),
            ]
        );
    }

    #[test]
    fn esc_screen_alignment_and_terminator() {
        let actor = RecordingActor::parse("\x1b#8\x1b\\");

        assert_eq!(actor.actions, vec![Action::ScreenAlignmentDisplay]);
    }
}
