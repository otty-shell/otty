# otty-plugin-example

The template of an OTTY plugin. A plugin teaches OTTY about coding agents
and model providers without any change to OTTY: it is a small
WebAssembly component with one or more parts, listed in
`otty-plugin.toml`. This one has an agent part, `example-agent`, over a
made-up agent that keeps its files in `~/.example-agent/`, and a provider
part, `example-cloud`, that reads the balance of a made-up provider. Copy
the folder into a new repository and replace what the parts read.

## Parts, permissions and `[api]`

- Each part belongs to a domain of OTTY. This plugin fills the `agents`
  domain, Agents mode: each `[[agents.agent]]` and `[[agents.provider]]`
  of the manifest is a part with its own id and its own permissions
  (`[agents.agent.permissions]`, `[agents.provider.permissions]`). While
  OTTY calls a function of one part, only that part's permissions apply:
  here the provider part may not read `~/.example-agent/`, and the agent
  part may not call `api.example.com`.
- All agent parts are served by one `impl Agent`, all provider parts by
  one `impl Provider`; the part's id comes with each call. The traits and
  the types of the domain are in `otty_plugin_sdk::agents`; the host
  operations (`files`, `http`, `keys`, `env` and the rest), a `Need` and a
  `Key` are at the root of the SDK, since no domain owns them.
- `[api]` names the version of each contract the plugin exports, one per
  kind of part it has, by its domain and its name: `agents.agent = "0.1"`
  for the `Agent` trait, `agents.provider = "0.1"` for the `Provider`
  trait, and `language = "0.1"` for the optional `Language` trait, which
  belongs to no part and no domain. The last line of `src/lib.rs`,
  `otty_plugin_sdk::export!(Example with agents::agent, agents::provider, language);`,
  must name the same contracts. A plugin with only agent parts and texts
  in English only writes `export!(Example with agents::agent)` and only
  `agents.agent` in `[api]`.
- A provider part reads an account's numbers any way it can: from the
  provider's server with the key, or by asking the agent in a profile
  the account was found in (`found.found_in`, each with the command its
  agent part starts the agent with there), under the provider part's
  own permissions. A key that is not at its place is a normal case:
  using its handle fails with `HostError::NotFound`. With no way that
  works, answer numbers with nothing in them, not an error. The
  template's provider part has only its server, so a key that is not
  there reads nothing.
- What the user must do for the agent or the provider, the part says
  itself as a need: an id, a title, a text and one button
  (`src/needs.rs`). The agent part gives one when nobody is logged in
  (its button runs `example-agent login`), and one from `login_refused`
  when a provider part refused a login it found: a provider part only
  answers `ReadError::LoginRefused` for a login an agent keeps, since it
  does not know which agent that is. The provider part asks for an
  admin key under its slot `admin` when the bill needs one, and for a new
  key when the provider refused it. The texts are in the language of
  OTTY's interface (`env::language()`, the one OTTY shows at that call),
  or in English. The template has every text in OTTY's ten languages, so
  it implements `Language`: OTTY calls it when the user picks another
  language, and reads the parts again when it answers `true`. A key the user
  gave comes back to `read_bill` and `numbers` as a handle with its slot.
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
Plugins mode.

## Release

Set `version` in `otty-plugin.toml`, commit, and push a tag with the same
version, such as `v0.1.0`. The workflow in `.github/workflows/release.yml`
tests and builds the plugin, checks that the tag equals the manifest
version, and attaches `otty-plugin.toml`, `example.wasm` and the icons to
the GitHub release. OTTY installs and updates the plugin from those
releases.
