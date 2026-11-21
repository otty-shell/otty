//! The bindings of the language receiver `otty:language@0.1.0`, from the
//! WIT files of `otty-plugin-sdk`.

wasmtime::component::bindgen!({
    path: "../sdk/wit/language/0.1.0",
    world: "otty:language/language-receiver@0.1.0",
});
