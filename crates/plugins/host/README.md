# otty-plugin-host

Runs OTTY plugins: WebAssembly components that teach OTTY about an agent
or a provider. It holds the one wasmtime engine, the cache of compiled
plugins, the bindings of every contract version OTTY keeps, and the host
operations a plugin calls. It calls a plugin with a memory cap and a
deadline, with only the permissions of the part being called; the manifest
and the permissions themselves come from `otty-plugin-manifest`. It starts
no program itself: the app gives it the way to start programs and hidden
terminals, and the host checks each one and stops it when the call that
started it ends.

It is a separate crate so that running untrusted code stays in one place
that knows no agent and no provider. It must not depend on `iced`, on any
crate under `crates/app/`, or on `otty-agents`; `otty-agents` uses it.

Plugins never see a key: the host gives them key handles and fills a key
into a request itself. The network and the system keyring sit behind
traits, so the tests use mocks and never reach them.

A plugin may also export the language receiver, which belongs to no
part: the engine gives every plugin the language OTTY shows at each
call, and `LoadedPlugin::language_changed` tells a plugin that exports
the receiver that the user picked another one. That call runs with no
permissions at all.

The bindings read the WIT files of `otty-plugin-sdk`. Each contract version
has its own module, such as `agent::v0_1`, `provider::v0_1` and
`language::v0_1`, and
`contract.rs` is the one place that maps a version of a manifest's `[api]`
to them. A new version, such as agent 0.2, is added next to the old one:
its WIT folder in the SDK, `agent::v0_2` here, a line in that map and in
the list of known versions of `otty-plugin-manifest`; the types of the
`agent` module then come from 0.2, and the 0.1 arm of `AgentCalls`
converts between the 0.1 types and them with `From`. The old version stays
as long as plugins use it.

The test plugins are built from `tests/plugins/` and the template plugin
from `plugins/template/` by `scripts/build-test-plugins.sh`, and kept as
`.wasm` files in `tests/fixtures/`, so the tests need no WebAssembly
target.
