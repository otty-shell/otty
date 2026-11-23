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
- Each part says in one sentence what it does with `description`, and
  so does the plugin, with commas, not semicolons: Plugins mode shows a
  part's on its card.
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
- An agent part knows nothing of accounts: `detect` says only found,
  or not logged in with its need. OTTY asks only about a folder that is
  there, so the part decides what an empty one needs. Each answer carries
  its money: the cost the agent wrote (`cost_microdollars` of a made-up
  answer), else its tokens at the API price OTTY gives for the model,
  both made by `agents::answer_cost`. OTTY only adds the money up.
- A provider part owns its accounts. It names the profiles of the agent
  parts of its own plugin it reads with `[[agents.provider.logins]]`: the
  agent part, the files it reads inside each profile folder (`read`),
  the files it may write and delete there, such as what a program it ran
  left behind (`write`, never a key file), the key files there with their
  key fields (`keys`, keys as handles), and the variable that points the
  agent at a folder (`folder_env`). The paths are written from inside the
  folder. OTTY calls `accounts` with the folders of those profiles,
  wherever they are, and the rules apply only inside them and only for
  that call. A key file outside every profile folder, such as one next
  to the default folder, is listed under
  `[[agents.provider.permissions.keys]]`, JSON fields of files only.
  The part gives each account it finds a stable id (OTTY keeps the
  user's name and Hide by it), a name, a kind, the folders it was found
  in and its needs; the same login in two folders is one account with
  two places. The template's account is the key of `auth.json`, known by
  its fingerprint and named by the email of the file.
- A `logins` entry with `log_in = true` offers Add account → "Log in with
  <agent>". OTTY makes a folder of its own, asks the part's `log_in` for
  the agent's login command pointed at it (`example-agent login` with
  `EXAMPLE_AGENT_HOME`), runs it in a terminal tab, and then calls
  `accounts` with that folder as `LoginFolder::OwnLogin`, read with the
  rules of that entry. It is never a profile. The agent keeps that login
  fresh when the part runs it there to read the numbers, and the part
  never renews a token itself. A login refused there is the need "Log in
  again" on the account.
- A provider part reads an account's numbers any way it can, only in the
  folders the account was found in (`account.found_in`): from the
  provider's server with the login there, or by asking the agent there,
  under the provider part's own permissions. With no way that works,
  answer numbers with nothing in them, not an error. The template's
  provider part has only its server, so an account whose key is gone
  reads nothing. Its tests run it with `TestHost::as_provider_in`, which
  gives the folders as OTTY does.
- What the user must do for the agent or the provider, the part says
  itself as a need: an id, a title, a text and one button
  (`src/needs.rs`). The agent part gives one when nobody is logged in
  (its button runs `example-agent login`). The provider part asks for an
  admin key under its slot `admin` when the bill needs one, and for a new
  key when the provider refused it. A refused or expired login of an
  account is the provider part's need too, on that account. The texts are in the language of
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
