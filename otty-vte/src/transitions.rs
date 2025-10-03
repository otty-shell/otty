use crate::enums::{Action, State};

#[inline(always)]
pub(crate) fn anywhere(state: State, byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn ground(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (Ground, Execute),
        0x20..=0x7f => (Ground, Print),
        0xc2..=0xdf | 0xe0..=0xef | 0xf0..=0xf4 => (Utf8Sequence, Utf8),
        _ => anywhere(Ground, byte),
    }
}

#[inline(always)]
pub(crate) fn escape(byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn escape_intermidiate(byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn csi_entry(byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn csi_param(byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn csi_intermediate(byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn csi_ignore(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (CsiIgnore, Execute),
        0x20..=0x3f | 0x7f => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, None),
        _ => anywhere(CsiIgnore, byte),
    }
}

#[inline(always)]
pub(crate) fn dcs_entry(byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn dcs_param(byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn dcs_intermediate(byte: u8) -> (State, Action) {
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

#[inline(always)]
pub(crate) fn dcs_passthrough(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f | 0x20..=0x7e => (DcsPassthrough, Put),
        0x7f => (DcsPassthrough, Ignore),
        _ => anywhere(DcsPassthrough, byte),
    }
}

#[inline(always)]
pub(crate) fn dcs_ignore(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f | 0x20..=0x7f => (DcsIgnore, Ignore),
        _ => anywhere(DcsIgnore, byte),
    }
}

#[inline(always)]
pub(crate) fn osc_string(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (OscString, Ignore),
        0x07 => (Ground, Ignore),
        0x20..=0x7f => (OscString, OscPut),
        0xc2..=0xdf | 0xe0..=0xef | 0xf0..=0xf4 => (Utf8Sequence, Utf8),
        _ => anywhere(OscString, byte),
    }
}

#[inline(always)]
pub(crate) fn sos_pm_apc_string(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f | 0x20..=0x7f => {
            (SosPmApcString, Ignore)
        },
        _ => anywhere(SosPmApcString, byte),
    }
}

#[inline(always)]
pub(crate) fn entry_action(state: State) -> Action {
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

#[inline(always)]
pub(crate) fn exit_action(state: State) -> Action {
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

#[inline(always)]
pub(crate) fn transit(state: State, byte: u8) -> (State, Action) {
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
