use std::fmt;
use log::debug;
use crate::{actor::Actor, charset::CharsetIndex};

/// Enumeration of the C0/C1 control codes that may be observed outside of an
/// escape sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlCode {
    // C0
    /// (NUL, Caret = ^@, C = \0) Null filler, terminal should ignore this character.
    Null,
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
    /// (ESC, Caret = ^[, C = \e) Prefix to an escape sequence.
    Escape,
    /// (SOH, Caret = ^A) Start of Header.
    StartOfHeading,
    /// (STX, Caret = ^B) Start of Text, implied end of header.
    StartOfText,
    /// (ETX, Caret = ^C) End of Text, causes some terminal to respond with ACK or NAK.
    EndOfText,
    /// (EOT, Caret = ^D) End of Transmission.
    EndOfTransmission,
    /// (ENQ, Caret = ^E) Enquiry, causes terminal to send ANSWER-BACK ID.
    Enquiry,
    /// (ACK, Caret = ^F) Acknowledge, usually sent by terminal in response to ETX.
    Acknowledge,
    /// (DLE, Caret = ^P) Data Link Escape, interpret next control character specially.
    DataLinkEscape,
    /// (DC1/XON, Caret = ^Q) Terminal is allowed to resume transmitting.
    DeviceControlOne,
    /// (DC2, Caret = ^R) Device Control 2, causes ASR-33 to activate paper-tape reader.
    DeviceControlTwo,
    /// (DC3/XOFF, Caret = ^S) Terminal must pause and refrain from transmitting.
    DeviceControlThree,
    /// (DC4, Caret = ^T) Device Control 4, causes ASR-33 to deactivate paper-tape reader.
    DeviceControlFour,
    /// (NAK, Caret = ^U) Negative Acknowledge, used sometimes with ETX and ACK.
    NegativeAcknowledge,
    /// (SYN, Caret = ^V) Synchronous Idle, used to maintain timing in Sync communication.
    SynchronousIdle,
    /// (ETB, Caret = ^W) End of Transmission block.
    EndOfTransmissionBlock,
    /// (CAN, Caret = ^X) Cancel (makes VT100 abort current escape sequence if any).
    Cancel,
    /// (EM, Caret = ^Y) End of Medium.
    EndOfMedium,
    /// (SUB, Caret = ^Z) Substitute (VT100 uses this to display parity errors).
    Substitute,
    /// (FS, Caret = ^\) File Separator.
    FileSeparator,
    /// (GS, Caret = ^]) Group Separator.
    GroupSeparator,
    /// (RS, Caret = ^^) Record Separator (sent by VT132 in block-transfer mode).
    RecordSeparator,
    /// (US, Caret = ^_) Unit Separator.
    UnitSeparator,
    // (SP) Space
    Space,
    /// (DEL, Caret = ^?) Delete, should be ignored by terminal.
    Delete,

    // C1
    /// (IND) Index.
    Index,
    /// (NEL) Next Line.
    NextLine,
    /// (HTS) Horizontal Tabulation Set.
    HorizontalTabSet,
    /// (DCS) Device Control String.
    DeviceControlString,
    /// (ST) String Terminator.
    StringTerminator,
    /// (OSC) Operating System Command.
    OperatingSystemCommand,
    /// (PM) Privacy Message.
    PrivacyMessage,
    /// (APC) Application Program Command.
    ApplicationProgramCommand,
    /// (CSI) Control Sequence Introducer.
    ControlSequenceIntroducer,
    /// (PAD) Padding Character.
    PaddingCharacter,
    /// (HOP) High Octet Preset.
    HighOctetPreset,
    /// (BPH) Break Permitted Here.
    BreakPermittedHere,
    /// (NBH) No Break Here.
    NoBreakHere,
    /// (SSA) Start of Selected Area.
    StartSelectedArea,
    /// (ESA) End of Selected Area.
    EndSelectedArea,
    /// (HTJ) Horizontal Tabulation With Justification.
    HorizontalTabWithJustify,
    /// (VTS) Vertical Tabulation Set.
    VerticalTabSet,
    /// (PLD) Partial Line Down.
    PartialLineDown,
    /// (PLU) Partial Line Up.
    PartialLineUp,
    /// (RI) Reverse Index.
    ReverseIndex,
    /// (SS2) Single-Shift 2
    SingleShiftTwo,
    /// (SS3) Single-Shift 3
    SingleShiftThree,
    /// (PU1) Private Use 1
    PrivateUseOne,
    /// (PU2) Private Use 2
    PrivateUseTwo,
    /// (STS) Set Transmit State
    SetTransmittingState,
    /// (CCH) Destructive backspace, intended to eliminate ambiguity about meaning of BS.
    CancelCharacter,
    /// (MW) Message Waiting
    MessageWaiting,
    /// (SPA) Start of Protected Area
    StartProtectedArea,
    /// (EPA) End of Protected Area
    EndProtectedArea,
    /// (SOS) Start of String
    StartOfString,
    /// (SCI) Single Character Introducer
    SingleCharacterIntroducer,
    /// (SGCI) Single Graphic Character Introducer
    SingleGraphicCharacterIntroducer,

    // Misc
    /// Unexpected control code
    Unexpected(u8),
}

impl From<u8> for ControlCode {
    fn from(byte: u8) -> Self {
        use ControlCode::*;
        match byte {
            // C0
            0x00 => Null,
            0x07 => Bell,
            0x08 => Backspace,
            0x09 => HorizontalTab,
            0x0A => LineFeed,
            0x0B => VerticalTab,
            0x0C => FormFeed,
            0x0D => CarriageReturn,
            0x0E => ShiftOut,
            0x0F => ShiftIn,
            0x1B => Escape,
            0x01 => StartOfHeading,
            0x02 => StartOfText,
            0x03 => EndOfText,
            0x04 => EndOfTransmission,
            0x05 => Enquiry,
            0x06 => Acknowledge,
            0x10 => DataLinkEscape,
            0x11 => DeviceControlOne,
            0x12 => DeviceControlTwo,
            0x13 => DeviceControlThree,
            0x14 => DeviceControlFour,
            0x15 => NegativeAcknowledge,
            0x16 => SynchronousIdle,
            0x17 => EndOfTransmissionBlock,
            0x18 => Cancel,
            0x19 => EndOfMedium,
            0x1A => Substitute,
            0x1C => FileSeparator,
            0x1D => GroupSeparator,
            0x1E => RecordSeparator,
            0x1F => UnitSeparator,
            0x20 => Space,
            0x7F => Delete,

            // C1
            0x84 => Index,
            0x85 => NextLine,
            0x88 => HorizontalTabSet,
            0x90 => DeviceControlString,
            0x9B => ControlSequenceIntroducer,
            0x9C => StringTerminator,
            0x9D => OperatingSystemCommand,
            0x9E => PrivacyMessage,
            0x9F => ApplicationProgramCommand,
            0x80 => PaddingCharacter,
            0x81 => HighOctetPreset,
            0x82 => BreakPermittedHere,
            0x83 => NoBreakHere,
            0x86 => StartSelectedArea,
            0x87 => EndSelectedArea,
            0x89 => HorizontalTabWithJustify,
            0x8A => VerticalTabSet,
            0x8B => PartialLineDown,
            0x8C => PartialLineUp,
            0x8D => ReverseIndex,
            0x8E => SingleShiftTwo,
            0x8F => SingleShiftThree,
            0x91 => PrivateUseOne,
            0x92 => PrivateUseTwo,
            0x93 => SetTransmittingState,
            0x94 => CancelCharacter,
            0x95 => MessageWaiting,
            0x96 => StartProtectedArea,
            0x97 => EndProtectedArea,
            0x98 => StartOfString,
            0x9A => SingleCharacterIntroducer,
            0x99 => SingleGraphicCharacterIntroducer,

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
            Null => "NUL",
            Bell => "BEL",
            Backspace => "BS",
            HorizontalTab => "HT",
            LineFeed => "LF",
            VerticalTab => "VT",
            FormFeed => "FF",
            CarriageReturn => "CR",
            ShiftOut => "SO",
            ShiftIn => "SI",
            Escape => "ESC",
            StartOfHeading => "SOH",
            StartOfText => "STX",
            EndOfText => "ETX",
            EndOfTransmission => "EOT",
            Enquiry => "ENQ",
            Acknowledge => "ACK",
            DataLinkEscape => "DLE",
            DeviceControlOne => "DC1",
            DeviceControlTwo => "DC2",
            DeviceControlThree => "DC3",
            DeviceControlFour => "DC4",
            NegativeAcknowledge => "NAK",
            SynchronousIdle => "SYN",
            EndOfTransmissionBlock => "ETB",
            Cancel => "CAN",
            EndOfMedium => "EM",
            Substitute => "SUB",
            FileSeparator => "FS",
            GroupSeparator => "GS",
            RecordSeparator => "RS",
            UnitSeparator => "US",
            Space => "SPACE",
            Delete => "DEL",

            // C1
            Index => "IND",
            NextLine => "NEL",
            HorizontalTabSet => "HTS",
            DeviceControlString => "DCS",
            ControlSequenceIntroducer => "CSI",
            StringTerminator => "ST",
            OperatingSystemCommand => "OSC",
            PrivacyMessage => "PM",
            ApplicationProgramCommand => "APC",
            PaddingCharacter => "PAD",
            HighOctetPreset => "HOP",
            BreakPermittedHere => "BPH",
            NoBreakHere => "NBH",
            StartSelectedArea => "SSA",
            EndSelectedArea => "ESA",
            HorizontalTabWithJustify => "HTJ",
            VerticalTabSet => "VTS",
            PartialLineDown => "PLD",
            PartialLineUp => "PLU",
            ReverseIndex => "RI",
            SingleShiftTwo => "SS2",
            SingleShiftThree => "SS3",
            PrivateUseOne => "PU1",
            PrivateUseTwo => "PU2",
            SetTransmittingState => "STS",
            CancelCharacter => "CCH",
            MessageWaiting => "MW",
            StartProtectedArea => "SPA",
            EndProtectedArea => "EPA",
            StartOfString => "SOS",
            SingleCharacterIntroducer => "SCI",
            SingleGraphicCharacterIntroducer => "SGCI",

            // Misc
            Unexpected(_) => "UNEXPECTED",
        };

        match self {
            Unexpected(b) => write!(f, "{code}: 0x{:02X}", b),
            _ => write!(f, "{code}")
        }
    }
}

impl ControlCode {
    pub(crate) fn perform<'a, A: Actor>(
        byte: u8,
        actor: &mut A,
    ) {
        let code = ControlCode::from(byte);
        match code {
            ControlCode::HorizontalTab => actor.put_tab(1),
            ControlCode::Backspace => actor.backspace(),
            ControlCode::CarriageReturn => actor.carriage_return(),
            ControlCode::LineFeed |
            ControlCode::FormFeed |
            ControlCode::VerticalTab => actor.linefeed(),
            ControlCode::Bell => actor.bell(),
            ControlCode::Substitute => actor.substitute(),
            ControlCode::ShiftOut => actor.set_active_charset(CharsetIndex::G1),
            ControlCode::ShiftIn => actor.set_active_charset(CharsetIndex::G0),
            _ => debug!("[unexpected: control_code] {code}")
        }
    }
}
