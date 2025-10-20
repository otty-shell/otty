//! High-level escape sequence consumer interface.
//!
//! The [`Parser`](crate::Parser) translates the raw byte stream into semantic
//! events and relays them to an [`Actor`] implementation.  Downstream crates
//! can implement this trait to mutate their terminal model, update UI state or
//! collect metrics without re-implementing the escape sequence finite state
//! machine.

use crate::{
    control::ControlCode, csi::CsiSequence, dcs::{EnterDeviceControl, ShortDeviceControl}, esc::EscapeSequence, osc::OperatingSystemCommand
};

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
