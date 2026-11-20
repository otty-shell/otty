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

Plugin authors depend on it by a git tag of the OTTY repository; it goes
to crates.io with its first stable version. The plugins in this
repository depend on it by path. See `plugins/template/` for a plugin to
start from.
