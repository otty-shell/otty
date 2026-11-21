//! The bindings `wit-bindgen` makes from `wit/`: the host operations once,
//! and one binding per trait a plugin exports. A plugin that exports
//! several traits uses several bindings; `wit-component` joins their
//! worlds when the component is made, so a set of them needs no binding
//! of its own.

/// The host operations of `otty:host@0.1.0` and the values the contracts
/// share.
pub mod host {
    wit_bindgen::generate!({
        path: "wit/host/0.1.0",
        world: "otty:host/operations@0.1.0",
        // The contracts use values of `types` no operation uses.
        generate_unused_types: true,
        // Plugin tests compare values; the types that hold a key can not
        // be compared.
        additional_derives: [PartialEq],
        additional_derives_ignore: ["json-with-keys", "value", "header", "request"],
    });
}

/// The agent contract `otty:agent@0.1.0`.
pub mod agent {
    wit_bindgen::generate!({
        path: ["wit/host/0.1.0", "wit/agent/0.1.0"],
        world: "otty:agent/agent-part@0.1.0",
        with: {
            "otty:host/types@0.1.0": crate::bindings::host::otty::host::types,
            "otty:host/keys@0.1.0": crate::bindings::host::otty::host::keys,
            "otty:host/files@0.1.0": crate::bindings::host::otty::host::files,
            "otty:host/sqlite@0.1.0": crate::bindings::host::otty::host::sqlite,
            "otty:host/http@0.1.0": crate::bindings::host::otty::host::http,
            "otty:host/programs@0.1.0": crate::bindings::host::otty::host::programs,
            "otty:host/terminal@0.1.0": crate::bindings::host::otty::host::terminal,
            "otty:host/keyring@0.1.0": crate::bindings::host::otty::host::keyring,
            "otty:host/env@0.1.0": crate::bindings::host::otty::host::env,
            "otty:host/clock@0.1.0": crate::bindings::host::otty::host::clock,
            "otty:host/log@0.1.0": crate::bindings::host::otty::host::log,
        },
        additional_derives: [PartialEq],
        pub_export_macro: true,
        export_macro_name: "export_agent",
        default_bindings_module: "otty_plugin_sdk::bindings::agent",
    });
}

/// The provider contract `otty:provider@0.1.0`.
pub mod provider {
    wit_bindgen::generate!({
        path: ["wit/host/0.1.0", "wit/provider/0.1.0"],
        world: "otty:provider/provider-part@0.1.0",
        with: {
            "otty:host/types@0.1.0": crate::bindings::host::otty::host::types,
            "otty:host/keys@0.1.0": crate::bindings::host::otty::host::keys,
            "otty:host/http@0.1.0": crate::bindings::host::otty::host::http,
            "otty:host/env@0.1.0": crate::bindings::host::otty::host::env,
            "otty:host/clock@0.1.0": crate::bindings::host::otty::host::clock,
            "otty:host/log@0.1.0": crate::bindings::host::otty::host::log,
        },
        additional_derives: [PartialEq],
        // A given key holds a handle, which can not be compared.
        additional_derives_ignore: ["given-key"],
        pub_export_macro: true,
        export_macro_name: "export_provider",
        default_bindings_module: "otty_plugin_sdk::bindings::provider",
    });
}

/// The language receiver `otty:language@0.1.0`, which imports nothing.
pub mod language {
    wit_bindgen::generate!({
        path: "wit/language/0.1.0",
        world: "otty:language/language-receiver@0.1.0",
        pub_export_macro: true,
        export_macro_name: "export_language",
        default_bindings_module: "otty_plugin_sdk::bindings::language",
    });
}
