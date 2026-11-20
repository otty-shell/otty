# otty-plugin-example

The template of an OTTY plugin. A plugin teaches OTTY about coding agents
and model providers without any change to OTTY: it is a small
WebAssembly component with one or more parts, listed in
`otty-plugin.toml`. This one has an agent part, `example-agent`, over a
made-up agent that keeps its files in `~/.example-agent/`, and a provider
part, `example-cloud`, that reads the balance of a made-up provider. Copy
the folder into a new repository and replace what the parts read.

## Parts, permissions and `[api]`

- Each `[[agent]]` and `[[provider]]` of the manifest is a part with its
  own id and its own permissions. While OTTY calls a function of one part,
  only that part's permissions apply: here the provider part may not read
  `~/.example-agent/`, and the agent part may not call `api.example.com`.
- All agent parts are served by one `impl Agent`, all provider parts by
  one `impl Provider`; the part's id comes with each call.
- `[api]` names the version of each contract the plugin exports, one per
  kind of part it has: `agent = "0.1"` for the `Agent` trait,
  `provider = "0.1"` for the `Provider` trait. The last line of
  `src/lib.rs`, `otty_plugin_sdk::export!(Example with agent, provider);`,
  must name the same traits. A plugin with only agent parts writes
  `export!(Example with agent)` and only `agent` in `[api]`.
- An agent part that reads the limits its agent shows implements
  `read_limits(profile, login)`. Where a number comes from is the
  plugin's business: try your sources in your own order and return the
  first limits you get, or one error that says why. OTTY asks the
  provider part of the login first, and the agent part only when that
  gives nothing. The template keeps the default, "not supported".
- Field names, file names, JSON shapes and URLs live in the plugin, never
  in OTTY. Keys are never read: files, variables and database columns
  that hold keys are listed in `keys`, and the plugin gets a handle,
  which it can put into a request header, fingerprint, or read the claims
  of a JWT through.

## No `HashMap`

Do not use `std::collections::HashMap` or `HashSet`: their random seed
makes the `.wasm` import `wasi:random`, and OTTY refuses the plugin at
install, since a plugin may import only `wasi:cli`, `wasi:io` and
`otty:*`. Use `BTreeMap` and `BTreeSet`. For the same reason, read files,
send requests and take the time through the SDK, not `std::fs`,
`std::net` or `std::time::SystemTime`.

## The SDK

Inside this repository the template depends on `otty-plugin-sdk` by path.
A plugin in its own repository depends on a git tag of OTTY:

```toml
[dependencies]
otty-plugin-sdk = { git = "https://github.com/otty-shell/otty", tag = "v0.1.0" }
```

## Test

`cargo test` runs natively. The SDK's test host serves `fixtures/` as the
home folder, answers HTTP requests from canned answers, and checks every
call against the permissions of the part being tested. Keep only made-up
keys and made-up session text in `fixtures/`.

## Build

```sh
rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2
```

The component is `target/wasm32-wasip2/release/otty_plugin_example.wasm`.
To try it in OTTY, put it as `example.wasm` in a folder with
`otty-plugin.toml` and the icons, and use **Add plugin from folder…** on
the Plugins page.

## Release

Set `version` in `otty-plugin.toml`, commit, and push a tag with the same
version, such as `v0.1.0`. The workflow in `.github/workflows/release.yml`
tests and builds the plugin, checks that the tag equals the manifest
version, and attaches `otty-plugin.toml`, `example.wasm` and the icons to
the GitHub release. OTTY installs and updates the plugin from those
releases.
