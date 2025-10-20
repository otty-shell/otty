use crate::control;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscapeSequence {
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
    /// SS2 Single shift of G2 character set affects next character only
    SingleShiftTwo,
    /// SS3 Single shift of G3 character set affects next character only
    SingleShiftThree,
    /// SPA - Start of Protected Area
    StartProtectedArea,
    /// EPA - End of Protected Area
    EndProtectedArea,
    /// SOS - Start of String
    StartOfString,
    /// ST - String Terminator
    StringTerminator,
    /// PM - Privacy Message
    PrivacyMessage,
    /// APC - Application Program Command
    ApplicationProgramCommand,
    /// RIS - Full Reset
    FullReset,
    /// Move the cursor to the bottom left corner of the screen
    CursorPositionLowerLeft,
    /// DECID - Return Terminal ID (obsolete form of CSI c - aka DA)
    ReturnTerminalId,
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
    /// Arrow up key press
    ApplicationModeArrowUpPress,
    /// Arrow down key press
    ApplicationModeArrowDownPress,
    /// Arrow right key press
    ApplicationModeArrowRightPress,
    /// Arrow left key press
    ApplicationModeArrowLeftPress,
    /// Home key press
    ApplicationModeHomePress,
    /// End key press
    ApplicationModeEndPress,
    /// F1 key press
    F1Press,
    /// F2 key press
    F2Press,
    /// F3 key press
    F3Press,
    /// F4 key press
    F4Press,
    /// Unhandled escape sequence
    Unspecified {
        control: u8,
        intermediates: Vec<u8>,
    },
}

impl From<(&[u8], u8)> for EscapeSequence {
    fn from(value: (&[u8], u8)) -> Self {
        let (intermediates, control) = value;
        
        let intermediate = if intermediates.len() == 1 {
            Some(intermediates[0])
        } else {
            None
        };

        match (intermediate, control) {
            (None, b'D') => EscapeSequence::Index,
            (None, b'E') => EscapeSequence::NextLine,
            (None, b'H') => EscapeSequence::HorizontalTabSet,
            (None, b'M') => EscapeSequence::ReverseIndex,
            (None, b'N') => EscapeSequence::SingleShiftTwo,
            (None, b'O') => EscapeSequence::SingleShiftThree,
            (None, b'V') => EscapeSequence::StartProtectedArea,
            (None, b'W') => EscapeSequence::EndProtectedArea,
            (None, b'X') => EscapeSequence::StartOfString,
            (None, b'\\') => EscapeSequence::StringTerminator,
            (None, b'^') => EscapeSequence::PrivacyMessage,
            (None, b'_') => EscapeSequence::ApplicationProgramCommand,
            (None, b'c') => EscapeSequence::FullReset,
            (None, b'F') => EscapeSequence::CursorPositionLowerLeft,
            (None, b'Z') => EscapeSequence::ReturnTerminalId,
            (None, b'6') => EscapeSequence::DecBackIndex,
            (None, b'7') => EscapeSequence::DecSaveCursorPosition,
            (None, b'8') => EscapeSequence::DecRestoreCursorPosition,
            (None, b'=') => EscapeSequence::DecApplicationKeyPad,
            (None, b'>') => EscapeSequence::DecNormalKeyPad,
            (Some(b'('), b'0') => EscapeSequence::DecLineDrawingG0,
            (Some(b'('), b'A') => EscapeSequence::UkCharacterSetG0,
            (Some(b'('), b'B') => EscapeSequence::AsciiCharacterSetG0,
            (Some(b')'), b'0') => EscapeSequence::DecLineDrawingG1,
            (Some(b')'), b'A') => EscapeSequence::UkCharacterSetG1,
            (Some(b')'), b'B') => EscapeSequence::AsciiCharacterSetG1,
            (Some(b'#'), b'3') => EscapeSequence::DecDoubleHeightTopHalfLine,
            (Some(b'#'), b'4') => EscapeSequence::DecDoubleHeightBottomHalfLine,
            (Some(b'#'), b'5') => EscapeSequence::DecSingleWidthLine,
            (Some(b'#'), b'6') => EscapeSequence::DecDoubleWidthLine,
            (Some(b'#'), b'8') => EscapeSequence::DecScreenAlignmentDisplay,
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
    fn parses_escape_sequences_map() {
        let cases: Vec<(EscapeSequence, &[u8], u8)> = vec![
            (EscapeSequence::FullReset, &[], b'c'),
            (EscapeSequence::Index, &[], b'D'),
            (EscapeSequence::NextLine, &[], b'E'),
            (EscapeSequence::CursorPositionLowerLeft, &[], b'F'),
            (EscapeSequence::HorizontalTabSet, &[], b'H'),
            (EscapeSequence::ReverseIndex, &[], b'M'),
            (EscapeSequence::SingleShiftTwo, &[], b'N'),
            (EscapeSequence::SingleShiftThree, &[], b'O'),
            (EscapeSequence::StartProtectedArea, &[], b'V'),
            (EscapeSequence::EndProtectedArea, &[], b'W'),
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
            (
                EscapeSequence::Unspecified {
                    control: b'%',
                    intermediates: vec![b'?'],
                },
                &[b'?'],
                b'%'
            )
        ];

        for (expected, intermediates, control) in cases {
            assert_eq!(EscapeSequence::from((intermediates, control)), expected);
        }
    }
}
