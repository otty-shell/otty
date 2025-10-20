#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscapeSequence {
    /// RIS - Full Reset
    FullReset,
    /// IND - Index.  Note that for Vt52 and Windows 10 ANSI consoles,
    /// this is interpreted as CursorUp
    Index,
    /// NEL - Next Line
    NextLine,
    /// Move the cursor to the bottom left corner of the screen
    CursorPositionLowerLeft,
    /// HTS - Horizontal Tab Set
    HorizontalTabSet,
    /// RI - Reverse Index – Performs the reverse operation of \n, moves cursor up one line,
    /// maintains horizontal position, scrolls buffer if necessary
    ReverseIndex,
    /// SS2 Single shift of G2 character set affects next character only
    SingleShiftG2,
    /// SS3 Single shift of G3 character set affects next character only
    SingleShiftG3,
    /// SPA - Start of Guarded Area
    StartOfGuardedArea,
    /// EPA - End of Guarded Area
    EndOfGuardedArea,
    /// SOS - Start of String
    StartOfString,
    /// DECID - Return Terminal ID (obsolete form of CSI c - aka DA)
    ReturnTerminalId,
    /// ST - String Terminator
    StringTerminator,
    /// PM - Privacy Message
    PrivacyMessage,
    /// APC - Application Program Command
    ApplicationProgramCommand,
    /// DECBI - Back Index
    DecBackIndex,
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
    /// Designate G0 Character Set - UK
    UkCharacterSetG0,
    /// Designate G0 Character Set – US ASCII
    AsciiCharacterSetG0,
    /// Designate G1 Character Set – DEC Line Drawing
    DecLineDrawingG1,
    /// Designate G1 Character Set - UK
    UkCharacterSetG1,
    /// Designate G1 Character Set – US ASCII
    AsciiCharacterSetG1,
    /// https://vt100.net/docs/vt510-rm/DECALN.html
    DecScreenAlignmentDisplay,
    /// DECDHL - DEC double-height line, top half
    DecDoubleHeightTopHalfLine,
    /// DECDHL - DEC double-height line, bottom half
    DecDoubleHeightBottomHalfLine,
    /// DECSWL - DEC single-width line
    DecSingleWidthLine,
    /// DECDWL - DEC double-width line
    DecDoubleWidthLine,
    /// These are typically sent by the terminal when keys are pressed
    ApplicationModeArrowUpPress,
    ApplicationModeArrowDownPress,
    ApplicationModeArrowRightPress,
    ApplicationModeArrowLeftPress,
    ApplicationModeHomePress,
    ApplicationModeEndPress,
    F1Press,
    F2Press,
    F3Press,
    F4Press,
    Unspecified {
        control: u8,
        intermediates: Vec<u8>,
    },
}

impl EscapeSequence {
    pub fn parse(intermediates: &[u8], control: u8) -> Self {
        let intermediate = if intermediates.len() == 1 {
            Some(intermediates[0])
        } else {
            None
        };

        match (intermediate, control) {
            (None, b'c') => EscapeSequence::FullReset, // RIS
            (None, b'D') => EscapeSequence::Index,     // IND
            (None, b'E') => EscapeSequence::NextLine,  // NEL
            (None, b'F') => EscapeSequence::CursorPositionLowerLeft, // "cursor to lower-left"
            (None, b'H') => EscapeSequence::HorizontalTabSet,        // HTS
            (None, b'M') => EscapeSequence::ReverseIndex,            // RI
            (None, b'N') => EscapeSequence::SingleShiftG2, // SS2  (next char only)
            (None, b'O') => EscapeSequence::SingleShiftG3, // SS3  (next char only)
            (None, b'V') => EscapeSequence::StartOfGuardedArea, // SPA
            (None, b'W') => EscapeSequence::EndOfGuardedArea, // EPA
            (None, b'X') => EscapeSequence::StartOfString, // SOS
            (None, b'Z') => EscapeSequence::ReturnTerminalId, // DECID (obsolete form of DA)
            (None, b'\\') => EscapeSequence::StringTerminator, // ST   (ESC \)
            (None, b'^') => EscapeSequence::PrivacyMessage,   // PM
            (None, b'_') => EscapeSequence::ApplicationProgramCommand, // APC
            (None, b'6') => EscapeSequence::DecBackIndex,     // DECBI
            (None, b'7') => EscapeSequence::DecSaveCursorPosition, // DECSC
            (None, b'8') => EscapeSequence::DecRestoreCursorPosition, // DECRC
            (None, b'=') => EscapeSequence::DecApplicationKeyPad, // DECPAM
            (None, b'>') => EscapeSequence::DecNormalKeyPad,  // DECPNM
            // ----- ESC '(' <final> — Designate G0 -----
            (Some(b'('), b'0') => EscapeSequence::DecLineDrawingG0, // DEC line drawing
            (Some(b'('), b'A') => EscapeSequence::UkCharacterSetG0, // UK
            (Some(b'('), b'B') => EscapeSequence::AsciiCharacterSetG0, // US ASCII
            // ----- ESC ')' <final> — Designate G1 -----
            (Some(b')'), b'0') => EscapeSequence::DecLineDrawingG1,
            (Some(b')'), b'A') => EscapeSequence::UkCharacterSetG1,
            (Some(b')'), b'B') => EscapeSequence::AsciiCharacterSetG1,
            // ----- ESC '#' <final> — DEC line layout / alignment -----
            (Some(b'#'), b'3') => EscapeSequence::DecDoubleHeightTopHalfLine, // DECDHL top
            (Some(b'#'), b'4') => EscapeSequence::DecDoubleHeightBottomHalfLine, // DECDHL bottom
            (Some(b'#'), b'5') => EscapeSequence::DecSingleWidthLine, // DECSWL
            (Some(b'#'), b'6') => EscapeSequence::DecDoubleWidthLine, // DECDWL
            (Some(b'#'), b'8') => EscapeSequence::DecScreenAlignmentDisplay, // DECALN
            // ----- ESC 'O' <final> — Application cursor keys & F1..F4 -----
            (Some(b'O'), b'A') => EscapeSequence::ApplicationModeArrowUpPress,
            (Some(b'O'), b'B') => EscapeSequence::ApplicationModeArrowDownPress,
            (Some(b'O'), b'C') => {
                EscapeSequence::ApplicationModeArrowRightPress
            },
            (Some(b'O'), b'D') => EscapeSequence::ApplicationModeArrowLeftPress,
            (Some(b'O'), b'H') => EscapeSequence::ApplicationModeHomePress,
            (Some(b'O'), b'F') => EscapeSequence::ApplicationModeEndPress,
            (Some(b'O'), b'P') => EscapeSequence::F1Press,
            (Some(b'O'), b'Q') => EscapeSequence::F2Press,
            (Some(b'O'), b'R') => EscapeSequence::F3Press,
            (Some(b'O'), b'S') => EscapeSequence::F4Press,
            _ => EscapeSequence::Unspecified {
                control,
                intermediates: intermediates.to_vec(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_escape_sequences() {
        let cases: Vec<(EscapeSequence, &[u8], u8)> = vec![
            (EscapeSequence::FullReset, &[], b'c'),
            (EscapeSequence::Index, &[], b'D'),
            (EscapeSequence::NextLine, &[], b'E'),
            (EscapeSequence::CursorPositionLowerLeft, &[], b'F'),
            (EscapeSequence::HorizontalTabSet, &[], b'H'),
            (EscapeSequence::ReverseIndex, &[], b'M'),
            (EscapeSequence::SingleShiftG2, &[], b'N'),
            (EscapeSequence::SingleShiftG3, &[], b'O'),
            (EscapeSequence::StartOfGuardedArea, &[], b'V'),
            (EscapeSequence::EndOfGuardedArea, &[], b'W'),
            (EscapeSequence::StartOfString, &[], b'X'),
            (EscapeSequence::ReturnTerminalId, &[], b'Z'),
            (EscapeSequence::StringTerminator, &[], b'\\'),
            (EscapeSequence::PrivacyMessage, &[], b'^'),
            (EscapeSequence::ApplicationProgramCommand, &[], b'_'),
            (EscapeSequence::DecBackIndex, &[], b'6'),
            (EscapeSequence::DecSaveCursorPosition, &[], b'7'),
            (EscapeSequence::DecRestoreCursorPosition, &[], b'8'),
            (EscapeSequence::DecApplicationKeyPad, &[], b'='),
            (EscapeSequence::DecNormalKeyPad, &[], b'>'),
            (EscapeSequence::DecLineDrawingG0, &[b'('], b'0'),
            (EscapeSequence::UkCharacterSetG0, &[b'('], b'A'),
            (EscapeSequence::AsciiCharacterSetG0, &[b'('], b'B'),
            (EscapeSequence::DecLineDrawingG1, &[b')'], b'0'),
            (EscapeSequence::UkCharacterSetG1, &[b')'], b'A'),
            (EscapeSequence::AsciiCharacterSetG1, &[b')'], b'B'),
            (EscapeSequence::DecDoubleHeightTopHalfLine, &[b'#'], b'3'),
            (EscapeSequence::DecDoubleHeightBottomHalfLine, &[b'#'], b'4'),
            (EscapeSequence::DecSingleWidthLine, &[b'#'], b'5'),
            (EscapeSequence::DecDoubleWidthLine, &[b'#'], b'6'),
            (EscapeSequence::DecScreenAlignmentDisplay, &[b'#'], b'8'),
            (EscapeSequence::ApplicationModeArrowUpPress, &[b'O'], b'A'),
            (EscapeSequence::ApplicationModeArrowDownPress, &[b'O'], b'B'),
            (
                EscapeSequence::ApplicationModeArrowRightPress,
                &[b'O'],
                b'C',
            ),
            (EscapeSequence::ApplicationModeArrowLeftPress, &[b'O'], b'D'),
            (EscapeSequence::ApplicationModeHomePress, &[b'O'], b'H'),
            (EscapeSequence::ApplicationModeEndPress, &[b'O'], b'F'),
            (EscapeSequence::F1Press, &[b'O'], b'P'),
            (EscapeSequence::F2Press, &[b'O'], b'Q'),
            (EscapeSequence::F3Press, &[b'O'], b'R'),
            (EscapeSequence::F4Press, &[b'O'], b'S'),
        ];

        for (expected, intermediates, control) in cases {
            assert_eq!(EscapeSequence::parse(intermediates, control), expected);
        }
    }

    #[test]
    fn parses_unspecified_escape_sequence() {
        let result = EscapeSequence::parse(&[b'?'], b'%');
        assert_eq!(
            result,
            EscapeSequence::Unspecified {
                control: b'%',
                intermediates: vec![b'?'],
            }
        );
    }
}
