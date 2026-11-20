//! The bindings of the provider contract `otty:provider@0.1.0`, from the
//! WIT files of `otty-plugin-sdk`.

wasmtime::component::bindgen!({
    path: ["../sdk/wit/host/0.1.0", "../sdk/wit/provider/0.1.0"],
    world: "otty:provider/provider-part@0.1.0",
    with: {
        "otty:host@0.1.0": crate::operations::v0_1::otty::host,
    },
});
