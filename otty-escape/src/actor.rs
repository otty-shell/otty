//! High-level escape sequence consumer interface.
//!
//! The [`Parser`](crate::Parser) translates the raw byte stream into semantic
//! events and relays them to an [`Actor`] implementation.  Downstream crates
//! can implement this trait to mutate their terminal model, update UI state or
//! collect metrics without re-implementing the escape sequence finite state
//! machine.

use crate::{
    csi::CsiSequence,
    dcs::{EnterDeviceControl, ShortDeviceControl},
    esc::EscapeSequence,
    osc::OperatingSystemCommand,
};
use std::fmt;

/// Trait implemented by consumers of the escape sequence parser.
///
/// All methods have a default empty implementation so that downstream crates
/// only need to override the variants they actually care about.  The parser
/// will invoke these callbacks synchronously while it walks through the input
/// byte stream.
pub trait Actor {
    /// Emits a printable Unicode scalar value.
    fn print(&mut self, _: char) {}

    /// Executes a single C0/C1 control code outside the context of a longer
    /// escape sequence.
    fn control(&mut self, _: ControlCode) {}

    /// Dispatches a Control Sequence Introducer (CSI) with the collected
    /// parameters and intermediates.
    fn csi(&mut self, _: CsiSequence) {}

    /// Dispatches a standard escape sequence.
    fn esc(&mut self, _: EscapeSequence) {}

    /// Dispatches an Operating System Command (OSC).
    fn osc(&mut self, _: OperatingSystemCommand) {}

    /// Signals entry into a device control mode.
    fn device_control_enter(&mut self, _: EnterDeviceControl) {}

    /// Streams raw data for the active device control mode.
    fn device_control_data(&mut self, _: u8) {}

    /// Signals exit from the current device control mode.
    fn device_control_exit(&mut self) {}

    /// Emits a short/inline device control payload that is self-contained.
    fn short_device_control(&mut self, _: ShortDeviceControl) {}

    /// Emits the termcap capability names requested by XTGETTCAP.
    fn xt_get_tcap(&mut self, _: Vec<String>) {}
}

/// Enumeration of the C0/C1 control codes that may be observed outside of an
/// escape sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlCode {
    Null,
    StartOfHeading,
    StartOfText,
    EndOfText,
    EndOfTransmission,
    Enquiry,
    Acknowledge,
    Bell,
    Backspace,
    HorizontalTab,
    LineFeed,
    VerticalTab,
    FormFeed,
    CarriageReturn,
    ShiftOut,
    ShiftIn,
    DataLinkEscape,
    DeviceControlOne,
    DeviceControlTwo,
    DeviceControlThree,
    DeviceControlFour,
    NegativeAcknowledge,
    SynchronousIdle,
    EndOfTransmissionBlock,
    Cancel,
    EndOfMedium,
    Substitute,
    Escape,
    FileSeparator,
    GroupSeparator,
    RecordSeparator,
    UnitSeparator,
    BreakPermittedHere,
    NoBreakHere,
    Index,
    NextLine,
    StartSelectedArea,
    EndSelectedArea,
    HorizontalTabSet,
    HorizontalTabWithJustify,
    VerticalTabSet,
    PartialLineForward,
    PartialLineBackward,
    ReverseIndex,
    SingleShiftTwo,
    SingleShiftThree,
    DeviceControlString,
    PrivateUseOne,
    PrivateUseTwo,
    SetTransmittingState,
    CancelCharacter,
    MessageWaiting,
    StartProtectedArea,
    EndProtectedArea,
    StartOfString,
    SingleCharacterIntroducer,
    ControlSequenceIntroducer,
    StringTerminator,
    OperatingSystemCommand,
    PrivacyMessage,
    ApplicationProgramCommand,
    Unknown(u8),
}

impl ControlCode {
    /// Convert from the raw byte value of the control code.
    #[must_use]
    pub fn from_byte(byte: u8) -> Self {
        use ControlCode::*;
        match byte {
            0x00 => Null,
            0x01 => StartOfHeading,
            0x02 => StartOfText,
            0x03 => EndOfText,
            0x04 => EndOfTransmission,
            0x05 => Enquiry,
            0x06 => Acknowledge,
            0x07 => Bell,
            0x08 => Backspace,
            0x09 => HorizontalTab,
            0x0A => LineFeed,
            0x0B => VerticalTab,
            0x0C => FormFeed,
            0x0D => CarriageReturn,
            0x0E => ShiftOut,
            0x0F => ShiftIn,
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
            0x1B => Escape,
            0x1C => FileSeparator,
            0x1D => GroupSeparator,
            0x1E => RecordSeparator,
            0x1F => UnitSeparator,
            0x82 => BreakPermittedHere,
            0x83 => NoBreakHere,
            0x84 => Index,
            0x85 => NextLine,
            0x86 => StartSelectedArea,
            0x87 => EndSelectedArea,
            0x88 => HorizontalTabSet,
            0x89 => HorizontalTabWithJustify,
            0x8A => VerticalTabSet,
            0x8B => PartialLineForward,
            0x8C => PartialLineBackward,
            0x8D => ReverseIndex,
            0x8E => SingleShiftTwo,
            0x8F => SingleShiftThree,
            0x90 => DeviceControlString,
            0x91 => PrivateUseOne,
            0x92 => PrivateUseTwo,
            0x93 => SetTransmittingState,
            0x94 => CancelCharacter,
            0x95 => MessageWaiting,
            0x96 => StartProtectedArea,
            0x97 => EndProtectedArea,
            0x98 => StartOfString,
            0x9A => SingleCharacterIntroducer,
            0x9B => ControlSequenceIntroducer,
            0x9C => StringTerminator,
            0x9D => OperatingSystemCommand,
            0x9E => PrivacyMessage,
            0x9F => ApplicationProgramCommand,
            other => Unknown(other),
        }
    }
}

impl fmt::Display for ControlCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ControlCode::*;
        let name = match self {
            Null => "NUL",
            StartOfHeading => "SOH",
            StartOfText => "STX",
            EndOfText => "ETX",
            EndOfTransmission => "EOT",
            Enquiry => "ENQ",
            Acknowledge => "ACK",
            Bell => "BEL",
            Backspace => "BS",
            HorizontalTab => "HT",
            LineFeed => "LF",
            VerticalTab => "VT",
            FormFeed => "FF",
            CarriageReturn => "CR",
            ShiftOut => "SO",
            ShiftIn => "SI",
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
            Escape => "ESC",
            FileSeparator => "FS",
            GroupSeparator => "GS",
            RecordSeparator => "RS",
            UnitSeparator => "US",
            BreakPermittedHere => "BPH",
            NoBreakHere => "NBH",
            Index => "IND",
            NextLine => "NEL",
            StartSelectedArea => "SSA",
            EndSelectedArea => "ESA",
            HorizontalTabSet => "HTS",
            HorizontalTabWithJustify => "HTJ",
            VerticalTabSet => "VTS",
            PartialLineForward => "PLD",
            PartialLineBackward => "PLU",
            ReverseIndex => "RI",
            SingleShiftTwo => "SS2",
            SingleShiftThree => "SS3",
            DeviceControlString => "DCS",
            PrivateUseOne => "PU1",
            PrivateUseTwo => "PU2",
            SetTransmittingState => "STS",
            CancelCharacter => "CCH",
            MessageWaiting => "MW",
            StartProtectedArea => "SPA",
            EndProtectedArea => "EPA",
            StartOfString => "SOS",
            SingleCharacterIntroducer => "SCI",
            ControlSequenceIntroducer => "CSI",
            StringTerminator => "ST",
            OperatingSystemCommand => "OSC",
            PrivacyMessage => "PM",
            ApplicationProgramCommand => "APC",
            Unknown(_) => "UNKNOWN",
        };
        write!(f, "{name}")
    }
}
