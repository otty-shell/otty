//! Callbacks invoked by the virtual terminal parser.
//!
//! The [`Parser`](crate::parser::Parser) walks through a byte stream and
//! translates it into higher level terminal actions. Those actions are handed
//! over to an [`Actor`] implementation that is responsible for mutating the
//! terminal model, updating UI state, logging, or whatever else the embedding
//! application needs. The trait methods mirror the action set defined by the
//! ECMA-48 escape sequence state machine and DEC/xterm conventions. Implementations
//! should be prepared to receive any sequence of calls that is valid according
//! to the virtual terminal protocol, and should avoid performing additional
//! parsing themselves.
use crate::csi::CsiParam;

/// Consumer-facing interface for terminal actions emitted by the parser.
///
/// Each method corresponds to a particular class of escape sequence or
/// printable data encountered while decoding a stream of terminal bytes.  
/// The semantics follow ECMA-48 where possible, with a few well-established
/// extensions.
///
/// ## Terminology:
/// An intermediate is a character in the range 0x20-0x2f that
/// occurs before the final character in an escape sequence.
///
/// `ignored_excess_intermediates` is a boolean that is set in the case
/// where there were more than two intermediate characters; no standard
/// defines any codes with more than two. Intermediates after
/// the second will set this flag and are discarded.
///
/// `params` in most of the functions of this trait are decimal integer parameters in escape
/// sequences. They are separated by semicolon characters. An omitted parameter is returned in
/// this interface as a zero, which represents the default value for that parameter.
pub trait VTActor {
    /// Emits a single printable Unicode code point.
    fn print(&mut self, c: char);

    /// Executes an immediate single-byte control function.
    ///
    /// This covers completed C0/C1 control characters that are *not* part of
    /// longer sequences (e.g. `BEL`, `BS`, `CR`, `CAN`, `SUB`, `IND`, `NEL`,
    /// `HTS`).
    fn execute(&mut self, byte: u8);

    /// Signals the start of a Device Control String (DCS).
    fn hook(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
        byte: u8,
    );

    /// Marks the end of the current control string (DCS).
    fn unhook(&mut self);

    /// Pass bytes as part of a device control string (DCS) to the handle chosen in
    /// `hook`. C0 controls will also be passed to the handler.
    fn put(&mut self, byte: u8);

    /// Dispatches an Operating System Command (OSC).
    fn osc_dispatch(&mut self, params: &[&[u8]], byte: u8);

    /// Dispatches a Control Sequence Introducer (CSI) escape.
    fn csi_dispatch(
        &mut self,
        params: &[CsiParam],
        intermediates: &[u8],
        parameters_truncated: bool,
        byte: u8,
    );

    /// Dispatches a standard escape sequence.
    fn esc_dispatch(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
        byte: u8,
    );
}
