#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Anywhere,
    Utf8Sequence,
    #[default]
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsIgnore,
    DcsPassthrough,
    OscString,
    SosPmApcString,
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    None,
    Ignore,
    Utf8,
    Print,
    Execute,
    Clear,
    Collect,
    Param,
    EscDispatch,
    CsiDispatch,
    Hook,
    Put,
    Unhook,
    OscStart,
    OscPut,
    OscEnd,
}
