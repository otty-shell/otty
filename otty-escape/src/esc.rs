#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscSequence {
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
    ///DECALN https://vt100.net/docs/vt510-rm/DECALN.html
    DecScreenAlignmentDisplay,
    Unspecified {
        control: u8,
        intermediates: Vec<u8>,
    },
}

impl From<(&[u8], u8)> for EscSequence {
    fn from(value: (&[u8], u8)) -> Self {
        let (intermediates, control) = value;
        
        let intermediate = if intermediates.len() == 1 {
            Some(intermediates[0])
        } else {
            None
        };

        match (intermediate, control) {
            (None, b'D') => EscSequence::Index,
            (None, b'E') => EscSequence::NextLine,
            (None, b'H') => EscSequence::HorizontalTabSet,
            (None, b'M') => EscSequence::ReverseIndex,
            (None, b'\\') => EscSequence::StringTerminator,
            (None, b'c') => EscSequence::FullReset,
            (None, b'Z') => EscSequence::ReturnTerminalId,
            (None, b'7') => EscSequence::DecSaveCursorPosition,
            (None, b'8') => EscSequence::DecRestoreCursorPosition,
            (None, b'=') => EscSequence::DecApplicationKeyPad,
            (None, b'>') => EscSequence::DecNormalKeyPad,
            (Some(b'('), b'0') => EscSequence::DecLineDrawingG0,
            (Some(b'('), b'B') => EscSequence::AsciiCharacterSetG0,
            (Some(b')'), b'0') => EscSequence::DecLineDrawingG1,
            (Some(b')'), b'B') => EscSequence::AsciiCharacterSetG1,
            (Some(b'*'), b'0') => EscSequence::DecLineDrawingG2,
            (Some(b'*'), b'B') => EscSequence::AsciiCharacterSetG2,
            (Some(b'+'), b'0') => EscSequence::DecLineDrawingG3,
            (Some(b'+'), b'B') => EscSequence::AsciiCharacterSetG3,
            (Some(b'#'), b'8') => EscSequence::DecScreenAlignmentDisplay,
            _ => EscSequence::Unspecified {
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
        let cases: Vec<(EscSequence, &[u8], u8)> = vec![
            (EscSequence::FullReset, &[], b'c'),
            (EscSequence::Index, &[], b'D'),
            (EscSequence::NextLine, &[], b'E'),
            (EscSequence::HorizontalTabSet, &[], b'H'),
            (EscSequence::ReverseIndex, &[], b'M'),
            (EscSequence::ReturnTerminalId, &[], b'Z'),
            (EscSequence::StringTerminator, &[], b'\\'),
            (EscSequence::DecSaveCursorPosition, &[], b'7'),
            (EscSequence::DecRestoreCursorPosition, &[], b'8'),
            (EscSequence::DecApplicationKeyPad, &[], b'='),
            (EscSequence::DecNormalKeyPad, &[], b'>'),
            (EscSequence::DecLineDrawingG0, &[b'('], b'0'),
            (EscSequence::AsciiCharacterSetG0, &[b'('], b'B'),
            (EscSequence::DecLineDrawingG1, &[b')'], b'0'),
            (EscSequence::AsciiCharacterSetG1, &[b')'], b'B'),
            (EscSequence::DecLineDrawingG2, &[b'*'], b'0'),
            (EscSequence::AsciiCharacterSetG2, &[b'*'], b'B'),
            (EscSequence::DecLineDrawingG3, &[b'+'], b'0'),
            (EscSequence::AsciiCharacterSetG3, &[b'+'], b'B'),
            (EscSequence::DecScreenAlignmentDisplay, &[b'#'], b'8'),
            (
                EscSequence::Unspecified {
                    control: b'%',
                    intermediates: vec![b'?'],
                },
                &[b'?'],
                b'%'
            )
        ];

        for (expected, intermediates, control) in cases {
            assert_eq!(EscSequence::from((intermediates, control)), expected);
        }
    }
}
