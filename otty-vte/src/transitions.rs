//! Transition helpers for the `otty-vte` finite state machine.
//!
//! The VTE parser is driven by a table of state transitions that mirrors the
//! DEC/ECMA-48 specification. Each function in this module is responsible for a
//! specific parser state: given an input byte it returns the next [`State`] and
//! the [`Action`] the higher level controller should perform. This keeps
//! terminal emulation logic table-driven and makes it straightforward to audit
//! coverage for the different control-sequence families (ESC, CSI, DCS, OSC,
//! SOS/PM/APC, and UTF-8 handling).

use crate::enums::{Action, State};

/// Transition that applies from any state when processing C1 controls and
/// common single-byte sequences.
#[inline(always)]
const fn anywhere(state: State, byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x18 | 0x1a | 0x80..=0x8f | 0x91..=0x97 | 0x99 | 0x9a => {
            (Ground, Execute)
        },
        0x9c => (Ground, None),
        0x1b => (Escape, None),
        0x98 | 0x9e | 0x9f => (SosPmApcString, None),
        0x90 => (DcsEntry, None),
        0x9d => (OscString, None),
        0x9b => (CsiEntry, None),
        _ => (state, None),
    }
}

/// Ground state handling printable data and C0 controls.
#[inline(always)]
const fn ground(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (Ground, Execute),
        0x20..=0x7f => (Ground, Print),
        0xc2..=0xf4 => (Utf8Sequence, Utf8),
        _ => anywhere(Ground, byte),
    }
}

/// ESC state waiting for the next byte to identify the sequence family.
#[inline(always)]
const fn escape(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (Escape, Execute),
        0x7f => (Escape, Ignore),
        0x20..=0x2f => (EscapeIntermediate, Collect),
        0x30..=0x4f | 0x51..=0x57 | 0x59 | 0x5a | 0x5c | 0x60..=0x7e => {
            (Ground, EscDispatch)
        },
        0x5b => (CsiEntry, None),
        0x5d => (OscString, None),
        0x50 => (DcsEntry, None),
        0x58 | 0x5e | 0x5f => (SosPmApcString, None),
        _ => anywhere(Escape, byte),
    }
}

/// ESC state that collects intermediate bytes before dispatch.
#[inline(always)]
const fn escape_intermidiate(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (EscapeIntermediate, Execute),
        0x20..=0x2f => (EscapeIntermediate, Collect),
        0x7f => (EscapeIntermediate, Ignore),
        0x30..=0x7e => (Ground, EscDispatch),
        _ => anywhere(EscapeIntermediate, byte),
    }
}

/// CSI entry point that validates and routes subsequent parameter bytes.
#[inline(always)]
const fn csi_entry(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (CsiEntry, Execute),
        0x7f => (CsiEntry, Ignore),
        0x20..=0x2f => (CsiIntermediate, Collect),
        0x3a => (CsiIgnore, None),
        0x30..=0x39 | 0x3b => (CsiParam, Param),
        0x3c..=0x3f => (CsiParam, Collect),
        0x40..=0x7e => (Ground, CsiDispatch),
        _ => anywhere(CsiEntry, byte),
    }
}

/// CSI parameter collection handling numeric fields and separators.
#[inline(always)]
const fn csi_param(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (CsiParam, Execute),
        0x30..=0x3b => (CsiParam, Param),
        0x7f => (CsiParam, Ignore),
        0x3c..=0x3f => (CsiIgnore, None),
        0x20..=0x2f => (CsiIntermediate, Collect),
        0x40..=0x7e => (Ground, CsiDispatch),
        _ => anywhere(CsiParam, byte),
    }
}

/// CSI intermediate state collecting extra bytes prior to dispatch.
#[inline(always)]
const fn csi_intermediate(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (CsiIntermediate, Execute),
        0x20..=0x2f => (CsiIntermediate, Collect),
        0x7f => (CsiIntermediate, Ignore),
        0x30..=0x3f => (CsiIntermediate, None),
        0x40..=0x7e => (Ground, CsiDispatch),
        _ => anywhere(CsiIntermediate, byte),
    }
}

/// CSI ignore state consuming bytes after an invalid introducer.
#[inline(always)]
const fn csi_ignore(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (CsiIgnore, Execute),
        0x20..=0x3f | 0x7f => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, None),
        _ => anywhere(CsiIgnore, byte),
    }
}

/// DCS entry point collecting the introducer and preparing parameters.
#[inline(always)]
const fn dcs_entry(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (State::DcsEntry, Action::Execute),
        0x7f => (DcsEntry, Ignore),
        0x3a => (DcsIgnore, None),
        0x20..=0x2f => (DcsIntermediate, Collect),
        0x30..=0x39 | 0x3b => (DcsParam, Param),
        0x3c..=0x3f => (DcsParam, Collect),
        0x40..=0x7e => (DcsPassthrough, None),
        _ => anywhere(DcsEntry, byte),
    }
}

/// DCS parameter collection equivalent to `csi_param` but for DCS strings.
#[inline(always)]
const fn dcs_param(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f | 0x7f => (DcsParam, Ignore),
        0x30..=0x39 | 0x3b => (DcsParam, Param),
        0x3a | 0x3c..=0x3f => (DcsIgnore, None),
        0x20..=0x2f => (DcsIntermediate, Collect),
        0x40..=0x7e => (DcsPassthrough, None),
        _ => anywhere(DcsParam, byte),
    }
}

/// DCS intermediate handler prior to entering passthrough mode.
#[inline(always)]
const fn dcs_intermediate(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f | 0x7f => (DcsIntermediate, Ignore),
        0x20..=0x2f => (DcsIntermediate, Collect),
        0x30..=0x3f => (DcsIgnore, None),
        0x40..=0x7e => (DcsPassthrough, None),
        _ => anywhere(DcsIntermediate, byte),
    }
}

/// DCS passthrough mode forwarding payload bytes to the active handler.
#[inline(always)]
const fn dcs_passthrough(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        // String Terminator (ST) in 8-bit form.
        0x9c => (Ground, None),
        // Mirror common VTE behavior: DCS payload is effectively a byte stream
        // (e.g. sixel or app-specific protocols). Accept high-bit bytes as
        // payload too, otherwise UTF-8 continuation bytes (0x80..=0xBF) can be
        // misinterpreted as C1 controls and prematurely terminate the DCS.
        0x00..=0x17
        | 0x19
        | 0x1c..=0x1f
        | 0x20..=0x7e
        | 0x80..=0x9b
        | 0x9d..=0xff => (DcsPassthrough, Put),
        0x7f => (DcsPassthrough, Ignore),
        _ => anywhere(DcsPassthrough, byte),
    }
}

/// DCS ignore state swallowing bytes after a malformed sequence.
#[inline(always)]
const fn dcs_ignore(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        // String Terminator (ST) in 8-bit form.
        0x9c => (Ground, None),
        0x00..=0x17
        | 0x19
        | 0x1c..=0x1f
        | 0x20..=0x7f
        | 0x80..=0x9b
        | 0x9d..=0xff => (DcsIgnore, Ignore),
        _ => anywhere(DcsIgnore, byte),
    }
}

/// OSC payload collection until BEL or ST is observed.
#[inline(always)]
const fn osc_string(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (OscString, Ignore),
        0x07 => (Ground, Ignore),
        0x20..=0x7f => (OscString, OscPut),
        0xc2..=0xf4 => (Utf8Sequence, Utf8),
        _ => anywhere(OscString, byte),
    }
}

/// SOS/PM/APC string collection mirroring OSC but with a different terminator.
#[inline(always)]
const fn sos_pm_apc_string(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f | 0x20..=0x7f => {
            (SosPmApcString, Ignore)
        },
        _ => anywhere(SosPmApcString, byte),
    }
}

/// Action to trigger upon entering a new state before reading the next byte.
#[inline(always)]
pub(crate) const fn entry_action(state: State) -> Action {
    use Action::*;
    use State::*;

    match state {
        Ground => None,
        Escape => Clear,
        EscapeIntermediate => None,
        CsiEntry => Clear,
        CsiParam => None,
        CsiIntermediate => None,
        CsiIgnore => None,
        DcsEntry => Clear,
        DcsParam => None,
        DcsIntermediate => None,
        DcsPassthrough => Hook,
        DcsIgnore => None,
        OscString => OscStart,
        SosPmApcString => None,
        Anywhere => None,
        Utf8Sequence => None,
        Nothing => None,
    }
}

/// Action to trigger after leaving a state, typically to finalize buffers.
#[inline(always)]
pub(crate) const fn exit_action(state: State) -> Action {
    use Action::*;
    use State::*;

    match state {
        Ground => None,
        Escape => None,
        EscapeIntermediate => None,
        CsiEntry => None,
        CsiParam => None,
        CsiIntermediate => None,
        CsiIgnore => None,
        DcsEntry => None,
        DcsParam => None,
        DcsIntermediate => None,
        DcsPassthrough => Unhook,
        DcsIgnore => None,
        OscString => OscEnd,
        SosPmApcString => None,
        Anywhere => None,
        Utf8Sequence => None,
        Nothing => None,
    }
}

/// Action to trigger in default ut8 parsing branch
#[inline(always)]
pub(crate) const fn utf8_state_action(state: State) -> Action {
    use Action::*;
    use State::*;

    match state {
        Ground => Print,
        OscString => OscPut,
        _ => None,
    }
}

/// Core transition table that delegates to state-specific helpers.
#[inline(always)]
pub(crate) const fn transit(state: State, byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match state {
        Ground => ground(byte),
        Escape => escape(byte),
        EscapeIntermediate => escape_intermidiate(byte),
        CsiEntry => csi_entry(byte),
        CsiParam => csi_param(byte),
        CsiIntermediate => csi_intermediate(byte),
        CsiIgnore => csi_ignore(byte),
        DcsEntry => dcs_entry(byte),
        DcsParam => dcs_param(byte),
        DcsIntermediate => dcs_intermediate(byte),
        DcsIgnore => dcs_ignore(byte),
        DcsPassthrough => dcs_passthrough(byte),
        OscString => osc_string(byte),
        SosPmApcString => sos_pm_apc_string(byte),
        Anywhere => anywhere(Anywhere, byte),
        _ => (Nothing, None),
    }
}
