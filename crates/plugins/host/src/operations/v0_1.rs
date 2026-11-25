//! The bindings of `otty:host@0.1.0`, from the WIT files of
//! `otty-plugin-sdk`.

wasmtime::component::bindgen!({
    path: "../sdk/wit/host/0.1.0",
    world: "otty:host/operations@0.1.0",
    with: {
        "otty:host/keys.key": crate::keys::KeyHandle,
        "otty:host/programs.talk": crate::call_programs::TalkHandle,
        "otty:host/terminal.hidden-terminal": crate::call_terminals::TerminalHandle,
    },
});
