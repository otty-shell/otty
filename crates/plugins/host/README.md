# otty-plugin-host

Runs OTTY plugins: WebAssembly components that teach OTTY about an agent
or a provider. It holds the one wasmtime engine, the cache of compiled
plugins, the plugin manifest (`otty-plugin.toml`) and the permissions of
each part of a plugin, and it calls a plugin with a memory cap and a
deadline, with only the permissions of the part being called.

It is a separate crate so that running untrusted code stays in one place
that knows no agent and no provider. It must not depend on `iced`, on any
crate under `crates/app/`, or on `otty-agents`; `otty-agents` uses it.

The test plugins are built from `tests/plugins/` by
`scripts/build-test-plugins.sh` and kept as `.wasm` files in
`tests/fixtures/`, so the tests need no WebAssembly target.
