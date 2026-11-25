//! The bindings of the provider contract `otty:agents-provider@0.1.0`,
//! from the WIT files of `otty-plugin-sdk`. The domain's shared values
//! and OTTY's price list are those of the agent contract's bindings.

wasmtime::component::bindgen!({
    path: [
        "../sdk/wit/host/0.1.0",
        "../sdk/wit/agents-agent/0.1.0",
        "../sdk/wit/agents-provider/0.1.0",
    ],
    world: "otty:agents-provider/provider-part@0.1.0",
    with: {
        "otty:host@0.1.0": crate::operations::v0_1::otty::host,
        "otty:agents-agent/types@0.1.0": crate::agents::agent::v0_1::otty::agents_agent::types,
        "otty:agents-agent/prices@0.1.0": crate::agents::agent::v0_1::otty::agents_agent::prices,
    },
});
