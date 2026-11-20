# otty-plugin-sdk

What an OTTY plugin is written with: the WIT files of every contract
version (`wit/host/`, `wit/agent/`, `wit/provider/`), one trait per kind of
plugin part (`Agent`, `Provider`), plain functions over the host
operations, key handles, and the `export!` macro that names the plugin
type and the traits it exports. The host's bindings read the same WIT
files.

It also holds a test host, so a plugin's tests run natively with plain
`cargo test`: the host operations are served from the plugin's
`fixtures/` folder and canned HTTP answers, and every call is checked
against the permissions of the part being tested. The test host is never
built for `wasm32-wasip2`.

Where a number comes from is the plugin's business. An agent part that
reads the limits its agent shows implements `Agent::read_limits(profile,
login)`: it tries its own programs, files or servers in its own order and
returns the first limits it gets, or one `ReadError` that says why
(`KeyRefused` for a login the agent must renew). OTTY asks the provider
part of the login first and the agent part only when that gives nothing.
The default says "not supported", and OTTY shows no limits of the agent.

Plugin authors depend on it by a git tag of the OTTY repository; it goes
to crates.io with its first stable version. The plugins in this
repository depend on it by path. See `plugins/template/` for a plugin to
start from.

Do not use `std::collections::HashMap` or `HashSet` in a plugin: their
random seed makes the `.wasm` import `wasi:random`, and OTTY refuses a
plugin that imports anything but `wasi:cli`, `wasi:io` and `otty:*` at
install. Use `BTreeMap` and `BTreeSet`.
