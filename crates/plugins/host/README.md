# otty-plugin-host

Runs OTTY plugins: WebAssembly components whose parts fill the
extension points of a domain of OTTY, such as an agent or a provider of
Agents mode. It holds the one wasmtime engine, the cache of compiled
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

Calling a part is no domain's business, so it lives here too: one read of
a part is one batch with that part's permissions only; a trap or a
missed deadline fails that read, and three failed reads in a row pause
the part until Try again; the last failure is kept for the part's card
on the Plugins page, and the report of a failure names the plugin, its
version and repository, the part, the call and the kind of error, never
a value. What a domain's parts answer is the domain's: `otty-agents`
turns the `read-error` of the `agents` contracts into a short tag of the
report. So the Plugins page lists the parts of any domain without
knowing one.

Plugins never see a key: the host gives them key handles and fills a key
into a request itself. The network and the system keyring sit behind
traits, so the tests use mocks and never reach them.

A plugin may also export the language receiver, which belongs to no
part: the engine gives every plugin the language OTTY shows at each
call, and `LoadedPlugin::language_changed` tells a plugin that exports
the receiver that the user picked another one. That call runs with no
permissions at all.

The bindings read the WIT files of `otty-plugin-sdk`. The contracts of a
domain are a module of that domain, and each contract version has its own
module in it: `agents::agent::v0_1` (`otty:agents-agent@0.1.0`, with the
domain's shared values `otty:agents-agent/types`),
`agents::provider::v0_1` (`otty:agents-provider@0.1.0`), and
`language::v0_1` for the language receiver of no domain. `contract.rs` is
the one place that maps a version of a manifest's `[api]`, keyed by the
domain and the point such as `agents.agent`, to them. A new version, such
as `agents.agent` 0.2, is added next to the old one: its WIT folder in
the SDK, `agents::agent::v0_2` here, a line in that map and in the list
of known versions of `otty-plugin-manifest`; the types of the
`agents::agent` module then come from 0.2, and the 0.1 arm of
`AgentCalls` converts between the 0.1 types and them with `From`. The
old version stays as long as plugins use it.

The test plugins are built from `tests/plugins/` and the template plugin
from `plugins/template/` by `scripts/build-test-plugins.sh`, and kept as
`.wasm` files in `tests/fixtures/`, so the tests need no WebAssembly
target.
