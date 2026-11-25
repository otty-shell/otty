//! The bindings of the agent contract `otty:agents-agent@0.1.0` and the
//! domain's shared values `otty:agents-agent/types`, from the WIT files of
//! `otty-plugin-sdk`.

wasmtime::component::bindgen!({
    path: ["../sdk/wit/host/0.1.0", "../sdk/wit/agents-agent/0.1.0"],
    world: "otty:agents-agent/agent-part@0.1.0",
    with: {
        "otty:host@0.1.0": crate::operations::v0_1::otty::host,
    },
});
