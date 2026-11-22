# Agent and provider plugins

Status: decisions agreed with the owner on 2026-11-19; the plugin model,
the official set and the open questions settled on 2026-11-20; the needs
of plugins, the language and the prices settled on 2026-11-21 (issue 20)

## Problem

OTTY reads coding agents and providers with Rust code written for each of
them by name: `crates/agents/src/claude_code/` (7,354 lines with tests),
`codex/` (6,947), `open_code/` (2,999), `account_numbers/` (1,374), the
closed `Provider` enum in `provider.rs`, and the `Provider::` matches in 26
files under `crates/app/`.

Agents change their files without notice. Issue 11 of spec
`2026-11-19-agents-accounts` made OTTY say "format changed" instead of
showing fewer tokens, but the fix is still an OTTY release. Every new agent
or provider is OTTY code too, so OTTY can only support what its own team
uses (Goose was paused, Kimi Code deferred, Moonshot is written but unused).

## Goal

No code in OTTY is written for one agent or one provider by name. Support
for an agent or a provider is a **plugin**: a small WebAssembly program in
its author's git repository, which users install by URL. When an agent
update breaks its format, the plugin's author ships a new release and OTTY
picks it up; OTTY itself does not change.

OTTY keeps the generic work (walking files, adding up tokens, removing
repeats, joining one account found in several agents, "format changed",
prices, the whole UI) and gives plugins low-level operations that know no
format. OTTY ships an official set of plugins that are reviewed and kept
safer than community ones.

The research is `docs/research/wasm-plugins.md`; the measurements are
`docs/research/wasm-plugins-spike.md` (spike code on branch
`spike/wasm-plugins`, commit `adca59f3`). Where this spec differs from the
research, this spec wins: the owner decided on releases from git instead of
a catalog, WASM only instead of declarative parts, and no disk cache of
per-file numbers.

## Words

- **WebAssembly (WASM)**: a compact program format that runs inside OTTY
  in a closed box: it can do nothing outside except call the functions
  OTTY gives it.
- **Plugin**: one git repository, one `.wasm` file, one release version
  (a semver tag). It is installed, updated and removed as a whole. Its
  release holds a manifest, the `.wasm` and its icons.
- **Part**: what a plugin does for one agent (an **agent part**, such as
  Codex) or one provider (a **provider part**, such as OpenAI). A plugin
  has one or more parts, all listed in its manifest; it never adds a part
  while running. Each part has its own id and its own permissions, and
  can be turned on or off on its own.
- **Domain**: a place of OTTY that plugins extend, with extension points
  of its own (owner, 2026-11-21, issue 21). Now `agents`, after Agents
  mode; a second domain, such as containers, may have a "provider" of its
  own, so every name of a part is the domain and the point.
- **Extension point**: a kind of part OTTY can load, within a domain. Now
  `agents.agent` and `agents.provider`; later commands, panels, block
  actions.
- **Contract (API)**: the typed interface of one extension point, written
  in WIT (the interface language of the WebAssembly component model). It
  lists what the part gives OTTY and what OTTY gives the part. Each
  contract has its own version, such as `otty:agents-agent@0.1.0`; a
  manifest names it without the patch number, as `agents.agent = "0.1"`.
- **Host**: OTTY's side, the code that runs plugins.
- **Host operation**: one function the host gives plugins, such as "read
  file bytes" or "HTTP request".
- **Manifest**: the plugin's `otty-plugin.toml`: its id, contract
  versions, parts, icons, and the permissions of each part.
- **Permission**: one thing a part may touch, listed in the manifest and
  shown to the user before install: a file glob, a program, an HTTP host,
  a keyring entry.
- **Key handle**: a number the plugin holds in place of a key or a login
  token. The plugin can put it into a request header, ask for its
  fingerprint, read the claims of a JWT through it or ask whether the
  user gave the key in OTTY, but never sees the key itself.
- **Registry**: the one list, in memory, of the agents and providers OTTY
  knows right now, filled from the parts that are on. The UI takes names
  and icons only from it.
- **Official plugin**: a plugin whose repository is under
  `github.com/otty-shell/`, the GitHub organization of the OTTY team.

Words of spec `2026-11-19-agents-accounts` (Agent, Profile, Provider,
Account, Source, Need) keep their meaning.

## Decisions

### 1. Goal: nothing in OTTY by agent or provider name

No module, constant, `match` or icon in OTTY names an agent or a provider.
Users and plugin authors maintain support for their agents and providers.
A format break after an agent update is fixed by a plugin release, not an
OTTY release.

### 2. WASM only, plugins written in Rust

- Every plugin is a WebAssembly component built for `wasm32-wasip2` with
  `opt-level = 3` (the spike: `"s"` saves 150 KB and costs up to 2x speed).
  There are no declarative plugins and no "click the fields" editor.
- Rust is the only documented and tested plugin language.
- New dependencies approved by the owner: `wasmtime` and `wasmtime-wasi`
  48 (LTS line, security fixes for 24 months), with the licence
  `Apache-2.0 WITH LLVM-exception` added to `deny.toml`; `toml` for the
  manifest; `globset` for permission globs; `wit-bindgen` 0.57 for the SDK
  and plugins; the `rusqlite` feature `hooks`; `usvg` as a direct
  dependency for the icon check (2026-11-20). `memchr` is already in
  `Cargo.lock`. Any other new dependency needs owner approval (see
  "Crates and dependencies").
- The binary grows by about 16.5 MB stripped (+26 %, the spike); the owner
  accepted this on 2026-11-20.

### 3. The SDK: one trait and one versioned contract per extension point

- The crate `otty-plugin-sdk` holds one trait per extension point, in a
  module per domain: now `otty_plugin_sdk::agents::{Agent, Provider}`
  with every type only that domain uses, later others. The plain helper
  functions that wrap the host operations, keys and needs stay at the
  crate root, since no domain owns them (issue 21).
- Each trait maps to its own versioned WIT contract:
  `otty:agents-agent@0.1.0` and `otty:agents-provider@0.1.0` (WIT names
  nested by domain, such as `otty:agents:agent`, are experimental, so the
  domain is the first word of the package name). The values both share
  (`read-error`, `identity`, `login-kind`, `limits`, `limit-window`,
  `credits`) are the interface `otty:agents-agent/types`, which the
  provider package imports. `otty:host` keeps only what any domain could
  use: the host operations and their errors, keys and key places, a
  command, a need and a key the user gave (`given-key`). There is no
  combined contract: a component exports one or both, and the plugin's
  manifest names the version of each one it exports in `[api]`
  (`agents.agent = "0.1"`, `agents.provider = "0.1"`).
- One optional contract belongs to no part (owner, 2026-11-21, issue 20):
  the language receiver `otty:language@0.1.0`, at the root of the SDK
  and in no domain. It has one function,
  `language-changed(tag) -> bool`, true when the plugin's texts depend on
  the language. The SDK trait is `Language`, exported with
  `export!(T with agents::agent, agents::provider, language)`; the
  manifest names it as `[api] language = "0.1"` and needs no part for it.
- A plugin exports each trait once, so all its agent parts share one
  `agent` version and all its provider parts share one `provider`
  version. The parts are told apart by the id OTTY passes to each call.
- OTTY keeps several versions of each contract working, as Zed does: each
  version stays in the host with converters from its types to the newest
  ones. A plugin runs only if OTTY knows every contract version in its
  `[api]`; otherwise the whole plugin shows "Needs a newer OTTY".
- The only macro is `export!`, which names the plugin type and the
  extension points it exports, each by its domain:
  `export!(OpenAi with agents::agent, agents::provider, language)`. It
  wraps the export that `wit-bindgen` makes.
- Plugin authors depend on the SDK by a git tag of the OTTY repository
  for now. It goes to crates.io with its first stable version
  (2026-11-20). The official plugins in this repository use it by path.

### 4. Plugins and their parts

- **One plugin**: one git repository, one `.wasm`, one release version.
  A new version updates all its parts at once.
- **Parts are listed up front.** The manifest lists every part under its
  domain (`[[agents.agent]]`, `[[agents.provider]]`), each with its id
  and its own permissions.
  A plugin can never add a part while running; a new part is a new
  release. A plugin may have any number of parts: the OpenAI plugin has
  the agent Codex and the provider OpenAI; a pack such as "OpenAI-
  compatible providers" may list many provider parts.
- **Agent part**: finds the agent's profiles (config folders), reads
  sessions, tokens, projects and the agent's version, gives the launch
  and resume commands, and reports which logins and keys sit in a
  profile and where (file and field, environment variable, keyring entry,
  database field), with the provider id or the host each is for. It reads
  no numbers of an account (issue 29): the profile is the main thing,
  and the agent part lists the accounts of a profile, whose they are,
  where their key sits and what else the files tell. A profile nobody is
  logged in with is `not-logged-in(need)`: the need's `run` is the
  login command (issue 20).
- **Provider part**: given a key handle, says whose account it is (account
  id or organization id, email, plan) and reads the account's numbers:
  limits (with the extra usage a subscription may buy), credits, Left, Key
  limit, Spent this month, the bill of an organization; and names its
  usage pages. A key the part needs and the account does not have, such
  as the admin key of an Anthropic or OpenAI bill, the part asks the user
  for with a need (`needs-user` with `ask-key`); OTTY then passes it to
  `numbers` and `read-bill` among the given keys, as a handle with the
  part's slot (issue 20). **It reads every way it can** (owner,
  2026-11-22, issue 29): OTTY tells it, in `login-facts.found-in`, every
  profile the account was found in, each with the plugin and the agent
  part that found it, its config folder and the command that part's
  `launch` gives for a new session there; the provider world imports
  files, programs and hidden terminals for it, under the provider part's
  own declared permissions. So the part reads the numbers from the
  provider's server with the key, or by asking the agent itself, such as
  `claude /usage` in a Claude Code profile (the Anthropic part) or the
  limits Codex knows through `codex app-server` and its session files
  (the OpenAI part). A key that is not at its place, such as a Claude
  Code profile whose `.credentials.json` is `{}` while Claude Code still
  works, is a normal case: using the handle fails with `not-found`, and
  the part tries its other ways. When no way works it answers numbers
  with nothing in them, not an error, and OTTY shows no limits. OTTY
  never skips a provider part on its own: the part decides. A list of
  found profiles was chosen over one profile because an account found
  in OpenCode and in Claude Code can only be probed in the latter, and
  the launch command over the folder alone because the agent part, not
  the provider part, knows which variables point the agent at a folder.
  It never sees when a login token
  expires or what it may read: it sends the request, and a refused or
  expired login that an agent keeps is `login-refused`; a part may read
  the expiry from the token's JWT claims and send nothing, as OpenAI's
  does. The provider part does not know which agent keeps the login, so
  it writes no need for it: OTTY asks the agent part of each profile that
  holds the login (`login-refused(profile, login) -> option<need>` of the
  agent contract) and shows its need, such as `run(codex)` or
  `run(opencode auth login)` (owner, 2026-11-21, issue 20). OTTY never
  renews a login.
- **A part gives its own needs** (owner, 2026-11-21, issue 20). A need is
  a stable id (so the same need from two reads shows once), a title, a
  text and one action: `run(command)` in a terminal tab, `ask-key` (a
  field for a key: the part's slot, or none for the account's own key
  (Decision 5, issue 26), a label, a hint, an optional prefix and an
  optional `https` page), `open-page(url)` (`https` only) or
  `copy-report(text)`. Everything the user must do for an agent or a
  provider comes from the part: not logged in, the login expired, a key
  missing or refused, a key of another kind to read a bill. OTTY shows it
  as it is, with a label naming the plugin ("OpenAI plugin"). OTTY keeps
  only the needs about plugins and its own permissions, which a plugin
  can not tell about itself, and shows them with no plugin label: a
  plugin update waits for the user's OK, a part is off, a key was found
  for a host no plugin serves, a plugin failed, ran out of time or was
  paused, the format of an agent's files changed for the plugin, allow
  the system keyring, and "the login was refused" (no button) when the
  agent part of a refused login gives no need, fails or runs out of
  time. Needs of a part are the same when they have the same part,
  profile and id.
- **Texts in the user's language** (owner, 2026-11-21, issue 20). The
  host tells the part the language of OTTY's interface (`env.language`),
  the one shown at that call, not at load; the part writes its needs in
  it or in English. The contract carries no translation tables. When the
  user changes the language, OTTY calls the language receiver (Decision
  3) of every plugin that exports it, and reads Agents mode again at once
  when one answers true; a plugin without it is not called.
- **Each part on its own.** The user turns each part on or off on the
  Plugins mode. A failing part is paused alone (Decision 8). Remove
  deletes the whole plugin.
- **No fight over names** (owner, 2026-11-22, issue 29). Nothing is
  asked at install or later, and `plugins.json` keeps no choices. Every
  provider part that serves an account reads it: when two plugins serve
  one provider, both read its accounts, and each reading is an account of
  its own (`Account::read_by`, the plugin id), named with its plugin
  ("Anthropic · you@example.com (Anthropic Fork)") while a part of
  another plugin serves the same provider id or host. An account known by
  a host is read by every part that serves the host, whatever its
  provider id: one copy per part, under that part's id (owner,
  2026-11-22, issue 31). The user hides the one they do not want; the
  choices for an account (name, hidden, given keys) are kept in
  `agents.json` with its plugin. An account the user added keeps its one
  key, given once: it is the entry with its key and no plugin, read by
  every plugin that offers Add account for its provider, and the name,
  the hidden flag and the given keys of each copy are an entry with the
  same key and that plugin, so each copy hides alone (issue 31). Two
  agent parts with one agent id: the first installed reads it, and the
  user turns the other off in Plugins mode.
- **Accounts and profiles are not plugins.** OTTY joins one account found
  in several agents by account id, organization id, email, or SHA-256 key
  fingerprint, as today (`merge_accounts`), within one provider id;
  copies under different provider ids are never joined, and with
  another provider or another identity it is two accounts, and that is
  fine (issues 29 and 31). An account no installed plugin reads is shown, with
  the line that no installed plugin reads it; its sessions and their
  estimated money still count.
- **Hidden accounts are left out of every total** (issue 29): no read
  starts for them, and their tokens go, in the totals by account, to
  another copy of the same login the user did not hide, else to no
  account. A copy is hidden alone: the other copies of the login stay
  shown, read and counted (issue 31).

### 5. Plugins never see a raw key

OTTY reads a key from the place the agent part named and hands the
provider part an opaque key handle. Through it the plugin can put the key
into a request header, get its fingerprint, check a prefix, or read the
claims (the middle part) of a JWT. Files, variables and database columns
that hold keys are readable by plugins only with the keys replaced by
handles. The host reads the key right before each use and drops it right
after, as today (`account_numbers/mod.rs`).

Of where a key came from, a handle tells one fact: whether the user gave
it in OTTY (on Add account, or in the field of an `ask-key` need) or OTTY
read it from a place of an agent (`given-in-otty`). It never tells the
place or its path. A part needs that fact for one thing: a refused key is
replaced where it sits (owner, 2026-11-21, issue 26). A refused key the
user gave in OTTY gets an `ask-key` need with no slot: OTTY opens the
field with the place of the old key and saves the new place in its stead,
as Edit key… does, and the next read gets the new key. A refused key an
agent keeps gets an `open-page` need for the provider's page of new keys,
and the user puts the new key where the agent keeps it. A refused key
given under a slot is asked for again under its slot.

### 6. Host operations are low-level and know no format

- The host gives: list files by glob, read file bytes, read lines with a
  host-side marker filter (`memchr::memmem`; the spike: 2.1x faster than
  copying every line; zstd files decoded), read-only SQLite query, HTTP
  request, run a program, line program (talk to a program line by line),
  hidden terminal (start a program unseen, send keys, read the screen
  text), keyring read, environment variable read, clock, log. The table is
  in "Host operations".
- **The rule**: anything that breaks when an agent or a provider updates
  (field names, JSON shapes, database schemas, SQL text, screen text,
  URLs, file names) lives in the plugin. A plugin may carry its own parser
  for a new format over raw bytes.
- The host does the generic work: walking files and noticing which
  changed, for the sessions as for the tokens (issue 25: a part names its
  session places and reads one file at a time, and keeps nothing between
  reads), joining what several files tell about one session, adding up
  tokens by day, hour, project and model, removing repeated answers by an
  identity the plugin returns, prices and money
  estimates, joining accounts, and the "format changed" verdict from the
  counts of records the plugin saw and understood.
- **A plugin gives rows, OTTY sums** (owner, 2026-11-21; issue 22). A
  row is one fact as the agent or the provider wrote it: one answer of
  the model with its time, model, provider, login, session, project and
  tokens; a limit or a balance a provider told; a bill. Every total is
  OTTY's: today, this week, by project, by model, money estimates,
  accounts joined across agents, and the numbers of one session (first
  and last answer, answers, tokens by model, tokens by day) and of one
  project (first and last answer, tokens by model) in their statistics.
  These come from the same answers, deduplicated and priced the same way
  as the totals, so the statistics agree with the totals. The contract
  has no call for statistics: `read-answers` gives every answer of the
  whole history, each naming the session `read-sessions` lists it under.
  A panel reads only the answer files the session places tie to its
  session, or to the sessions in its project folder; only while some
  answer file is no session place does it read the whole history, since
  then no file tells whose answers that one holds. A number no row can
  give, such as how many messages the user typed, is not shown until the
  owner decides how a plugin tells it.
- **Prices are OTTY's** (owner, 2026-11-21). The price table (LiteLLM) is
  an OTTY feature, fetched once for all plugins. Plugins report tokens,
  and OTTY prices them. A host operation for a plugin to ask OTTY for the
  price of a model is added only when a plugin needs to compute money
  itself; none does today.
- **The plugin says who served an answer, OTTY picks the price** (owner,
  2026-11-21; issue 23). Each answer names the model as the provider
  names it and who served it, read from the agent's settings and files:
  the provider's name as the agent wrote it, even one no plugin serves
  (OpenCode's `providerID`, such as `google`), or the host of its
  server. OTTY, not the plugin, joins the name to the provider part that
  serves it. The table's key tells whose price a row is: the plain model
  name (`claude-sonnet-4-5`) is the maker's own price, `<provider>/…`
  (`openrouter/…`, `azure/…`) the price of that provider. OTTY prices an
  answer at the row of who served it and the model; else at the row of
  the plain model name; else it has no price and shows "no price", never
  a guess. A provider is known to the table by its id, unless its part
  names another name with `priced_as` in its manifest. No list of makers
  or providers is in OTTY.

### 7. Permissions: declared per part, shown, and applied by context

- Each part lists its own permissions. An agent part: the file globs it
  reads and writes, the SQLite databases it queries, the programs it runs,
  hidden terminals, the keyring entries and environment variables it
  reads, and where its keys are. A provider part: the HTTP hosts it calls,
  and the keys it gets (only those of accounts of its own provider id or
  hosts).
- **Applied by context.** When OTTY calls a function of part X, only X's
  permissions apply. For example, while the `openai` provider part's
  `numbers` runs, reading `~/.codex` files is refused, though the `codex`
  agent part of the same plugin may read them.
- A part MAY write or delete files when it declares it; for example the
  Claude Code part's `/usage` probe deletes the session files it caused.
- The user sees the permissions, part by part, before install and again
  when an update asks for more. Nothing is granted silently.
- OTTY limits plugins only by extension point and permissions. Inside them
  the plugin decides: which of its declared hosts a key goes to, which
  `codex app-server` methods it calls.
- The official set is reviewed by the OTTY team and is the safer choice;
  the UI marks it "Official".

### 8. Engine (from the spike)

wasmtime with the sync API; epoch deadline (not fuel); `StoreLimits`
memory cap 64 MiB per plugin; empty WASI context (no files, no network, no
environment, stdio closed); import allowlist `wasi:cli`, `wasi:io` and
`otty:*` only, so a plugin importing `wasi:filesystem` or `wasi:sockets` is
refused at install; compiled plugins cached on disk (first compile about
0.5 s, cached load 0.3 ms). No disk cache of per-file numbers (the spike:
plugin 1.3x slower than native on 1.46 GB, identical totals). A trap or a
missed deadline fails that call only; three failures in a row pause only
the part whose calls failed, and the plugin's other parts keep working.

### 9. Distribution: git repositories and GitHub Releases

- A plugin lives in its author's git repository. The user installs it by
  pasting the repository URL in OTTY.
- A version is a GitHub Release with a semver tag (`v1.2.0` or `1.2.0`);
  other tags are ignored, and a repository with none is refused. OTTY
  downloads the built `.wasm`, the manifest and the icons from the
  release's assets and records their SHA-256. If the same tag later serves
  different bytes, OTTY warns and does not install them.
- **Updates are automatic** when the newest release asks for no new
  permissions and OTTY knows all its contract versions (2026-11-20). A
  release that asks for new permissions (a new part counts) shows the
  permission diff and waits for the user; the installed version keeps
  running. There is no catalog for now.
- A plugin template repository with CI builds the `.wasm` and attaches it
  to a release.
- **Local plugins for authors.** **Add plugin from folder…** installs a
  plugin from a local folder that holds its manifest, its built `.wasm`
  and its icons (2026-11-20). It goes through the same checks and the
  same install dialog, is marked "Local", is never updated on its own,
  and picks up a rebuilt copy when the user presses **Reload**. The owner
  uses it for the hand checks of the issues, and users install the
  official plugins with it until they have repositories of their own
  (issue 28).

### 10. The official set

Grouped by vendor, one plugin each (changed on 2026-11-20):

| Plugin | Agent parts | Provider parts | Built in `assets/plugins/` |
| --- | --- | --- | --- |
| Anthropic (`anthropic`) | Claude Code (`claude-code`) | Anthropic (`anthropic`) | Yes |
| OpenAI (`openai`) | Codex (`codex`) | OpenAI (`openai`) | Yes |
| OpenCode (`opencode`) | OpenCode (`opencode`) | none | Yes |
| DeepSeek (`deepseek`) | none | DeepSeek (`deepseek`) | Yes |
| OpenRouter (`openrouter`) | none | OpenRouter (`openrouter`) | Yes |

- **Not Kilo and not Kilo Gateway**: today's Kilo code (the Kilo variant
  of the `open_code` reader, Kilo logins and the Kilo Gateway balance in
  `account_numbers/kilo.rs`, `Provider::Kilo`, the Kilo commands and icon)
  is removed with the core code, not ported. **Not Goose**: the Goose
  keyring entry and its constants in `keyring_entry.rs` are removed the
  same way; the generic keyring host operation stays for any plugin that
  declares an entry. **Not Moonshot**: the owner does not use it; its
  code of issue 09 of the old spec is removed the same way. **Not Kimi
  Code**: deferred.
- OTTY ships no plugin (owner, 2026-11-21, issue 28; this replaces the
  embedding of 2026-11-20). On first start nothing is installed and
  Agents mode says in one line that no agent plugin is on, with a button
  that opens Plugins mode. The user installs the official plugins the
  same way as any other plugin; installed plugins stay in `plugins.json`
  between starts. A plugin that finds nothing on this computer simply
  does nothing.
- For now the official plugins live in `plugins/<id>/` of this
  repository, and `plugins/build.sh <folder>` builds each into
  `assets/plugins/<id>/` (manifest, `.wasm`, icons), which the tests read
  and users install with **Add plugin from folder…**. The owner moves them
  to their own repositories later; from then on users add them with
  **Add plugin…** and they update from their releases like any other
  plugin.
- "Official" means the repository is under `github.com/otty-shell/`. The
  organization may be renamed later, so OTTY keeps its name in one
  constant.

### 11. Migration: a plugin replaces its in-core reader in one issue

A plugin replaces the in-core reader of its agent or provider in the same
issue, and that issue deletes the in-core code. The owner checks the
result by hand on the real files; nothing is compared with the old
version, not even from git (2026-11-20). Until then the in-core readers
stay only as built-in registry entries that keep the app working. Codex
goes first among the hard cases: its reader keeps state across lines,
some files are zstd-compressed, and its files take 10.8 GB; the spike did
not cover it. Kilo, Goose and Moonshot have no plugin and are deleted.

### 12. UI

- **Plugins mode** (owner, 2026-11-22, issue 30; it was a page over
  Settings until then): a mode of its own, like Agents mode, in the mode
  switcher and All modes and not pinned by default (plugins will not
  only be for agents). The left panel lists the installed plugins with a
  search by name, Add by git URL and Add from a local folder; the main
  area shows the selected plugin with its parts, a switch per part, their
  permissions and health, updates with the permission changes, Reload
  and Remove. Settings' Manage plugins… and the links and needs of
  Agents mode open it.
- Account info and Profile info get a row "Read by: <plugin> <version>".
- No table of sources (owner, 2026-11-20, issue 19): where a number comes
  from is the plugin's business. A page tells how old its numbers are,
  greys the last good ones with why when a read fails, puts what the
  user must do in "Needs from you", marks money estimated at the list
  prices "estimate, list prices", and names the plugin in "Read by". The
  card of a part in Plugins mode shows its last failed call.
- One mark of an estimate everywhere (owner, 2026-11-21): money at the
  list prices has no "~" and no note; the mark "estimate, list prices"
  stands under the sum (account page, usage block in Money) with the
  time of the prices in its tooltip, or follows the sum where there is no
  room under it: "$42.10 · estimate, list prices" in the left panel, the
  All accounts table, Found by OTTY, the account chips and the Cost of a
  project.
- The "format changed" need and its report name the plugin, its version
  and its repository.
- New needs: "Key for <host> found in <agent>: no provider plugin [Add
  plugin…]" and "Plugin update needs your OK".
- A part's need shows as the part wrote it, with a small label naming the
  plugin ("OpenAI plugin"); OTTY's own needs carry no label (Decision 4).
  The button runs the action: "Run `<command>`" opens a terminal tab,
  "<label>…" opens a key field named by the part's label with its hint,
  prefix and page, "Open <host>" opens the page, "Copy report" copies the
  text. A key given there is kept in `agents.json` as a file path or a
  variable name under the part's slot, never the key (issue 20).
- Agents mode keeps its look: the left panel with Accounts, Profiles and
  Projects, and the account, profile and project pages of spec
  `2026-11-19-agents-accounts`.

### 13. Safety rules for official plugins

Official plugins never print or log keys, tokens, prompts or session text;
never renew or write another agent's login; never call `thread/list` on a
real Codex folder (it deleted rows of Codex's session index on the owner's
computer); read the keyring only with the user's permission (Decision 8 of
spec `2026-11-19-agents-accounts`). The review of each official release
checks these rules.

### 14. The old spec

In `specs/2026-11-19-agents-accounts`: issue 13 (Goose) is dropped by the
owner (2026-11-20, `wontfix`): Goose gets neither core code nor an
official plugin; issue 14 (Kimi Code) stays deferred; issue 15 (public
docs) moves to the end of this spec (issue 18 here).

### 15. Links in the interface stay when plugins change

Providers and agents become data from installed plugins, and every link in
the UI is made so that it does not break when a plugin or a part is
missing, off, updated or failing. See "Links in the interface".

## What a plugin is

A release of a plugin has these assets:

- `otty-plugin.toml`: the manifest;
- `<id>.wasm`: the component, built for `wasm32-wasip2`;
- each SVG icon the manifest names (single color, `currentColor`, at most
  16 KiB, no scripts, no external links, no embedded images).

The manifest of the OpenAI plugin, which has one agent part and one
provider part (version numbers are examples):

```toml
# otty-plugin.toml
[plugin]
id = "openai"                     # stable: a new id is a new plugin
name = "OpenAI"
version = "0.4.0"                 # equals the release tag without "v"
description = "Codex sessions, tokens and limits; ChatGPT and OpenAI account numbers."
repository = "https://github.com/otty-shell/otty-plugin-openai"
license = "Apache-2.0"
authors = ["OTTY team"]
wasm = "openai.wasm"

[api]                             # one version per contract the .wasm exports
agents.agent = "0.1"              # domain.point
agents.provider = "0.1"
language = "0.1"                  # the language receiver; belongs to no part and no domain

[[agents.agent]]
id = "codex"                      # saved in agents.json; never changes
name = "Codex"
icon = "codex.svg"
program = "codex"                 # looked up on PATH, "installed or not"
tested_version = "0.169.0"        # newest agent version this release was checked with

[agents.agent.permissions]               # apply only while a function of this part runs
read = ["~/.codex*/**", "{CODEX_HOME}/**"]
env = ["CODEX_HOME"]

[[agents.agent.permissions.run]]
program = "codex"
args = ["--version"]

[[agents.agent.permissions.run]]         # talked to line by line
program = "codex"
args = ["-s", "read-only", "-a", "never", "app-server"]

# Where keys are. These places are readable only with the keys
# replaced by key handles.
[[agents.agent.permissions.keys]]
files = ["~/.codex*/auth.json", "{CODEX_HOME}/auth.json"]
fields = ["/OPENAI_API_KEY", "/tokens/access_token", "/tokens/id_token",
          "/tokens/refresh_token"]

[[agents.provider]]
id = "openai"                     # provider ids and agent ids are separate lists
name = "OpenAI"
icon = "openai.svg"
hosts = ["openai.com", "chatgpt.com"]  # a host is this or ends with "." + this
usage_page = "https://platform.openai.com/usage"            # of a key or an organization
plan_usage_page = "https://chatgpt.com/codex/settings/usage" # of a subscription; usage_page when absent
add_by_hand = true                # Add account offers it for a key no agent uses
# priced_as = "gemini"            # its name in the LiteLLM price table, when not the id

[agents.provider.permissions]
http = ["chatgpt.com", "api.openai.com"]
```

A plugin with one part is shorter. The OpenCode plugin:

```toml
[plugin]
id = "opencode"
name = "OpenCode"
version = "0.2.0"
description = "Sessions and tokens of OpenCode."
repository = "https://github.com/otty-shell/otty-plugin-opencode"
license = "Apache-2.0"
authors = ["OTTY team"]
wasm = "opencode.wasm"

[api]
agents.agent = "0.1"
language = "0.1"

[[agents.agent]]
id = "opencode"
name = "OpenCode"
icon = "opencode.svg"
program = "opencode"
tested_version = "2.3.2"

[agents.agent.permissions]
read = ["~/.local/share/opencode/**", "{XDG_DATA_HOME}/opencode/**"]
sqlite = ["~/.local/share/opencode/*.db", "{XDG_DATA_HOME}/opencode/*.db", "{OPENCODE_DB}"]
env = ["XDG_DATA_HOME", "OPENCODE_DB"]

[[agents.agent.permissions.keys]]
files = ["~/.local/share/opencode/auth.json", "{XDG_DATA_HOME}/opencode/auth.json"]
fields = ["/*/key", "/*/access", "/*/refresh"]

[[agents.agent.permissions.keys]]        # a JSON column of a table
sqlite = ["~/.local/share/opencode/*.db", "{XDG_DATA_HOME}/opencode/*.db", "{OPENCODE_DB}"]
table = "credential"
column = "value"
fields = ["/key", "/access", "/refresh"]
```

A pack such as "OpenAI-compatible providers" has no `[[agents.agent]]`,
only `[api] agents.provider = "0.1"` and many `[[agents.provider]]`
parts, each with its own hosts and `http` permission. A part outside its
domain, such as a top-level `[[agent]]`, is an unknown field and refused.

Paths start with `~` (home) or with a variable the part listed in `env`,
written `{XDG_DATA_HOME}`; a glob may not contain `..` and may not leave
home unless a listed variable points there. Writes are listed as
`write = [...]`, programs as `[[agents.agent.permissions.run]]` with the
program and an argument pattern, hidden terminals as
`[[agents.agent.permissions.terminal]]`, keyring entries as
`[[agents.agent.permissions.keyring]]` with service, user and the JSON fields
holding keys. The manifest is checked as a whole: every part needs the
contract of its kind in `[api]`, and `[api]` names no contract without a
part, except `language`, which belongs to the whole plugin.

## SDK

`otty-plugin-sdk` is what a plugin author depends on. It holds the WIT
files of every contract version, the bindings `wit-bindgen` makes from
them, the traits below, helpers for the host operations, and a test host
for plain `cargo test` (the host operations served from a fixtures folder
and canned HTTP answers, so plugin tests need no wasm). A sketch, not the
final API:

```rust
//! otty-plugin-sdk (sketch)

/// One coding agent program: its profiles, sessions, tokens and logins.
/// A plugin implements it once and serves each agent part its manifest
/// lists; `profile.agent()` says which one is asked.
pub trait Agent {
    /// The config folders of this agent found on this computer, default
    /// first: the default folder (or the one its variable names) and
    /// extra folders such as `~/.codex-work` that hold a login or a session.
    fn profiles(agent: &AgentId, env: &Env) -> Vec<ProfilePlace>;

    /// Whether the profile is logged in. Nobody logged in is
    /// `ProfileStatus::NotLoggedIn(need)`, the need saying how to log in.
    fn detect(profile: &Profile) -> ProfileStatus;

    /// The agent's version written in the profile's newest files.
    /// `None`: OTTY runs `<program> --version` itself.
    fn version(profile: &Profile) -> Option<String>;

    /// The logins and keys the profile holds, each with where it sits
    /// and whom it is for. Never the key itself.
    fn logins(profile: &Profile) -> Vec<Login>;

    /// What holds the profile's sessions: file globs or a database. The
    /// host walks them, keeps what each file gave and asks again only for
    /// what is new or changed (issue 25).
    fn session_places(profile: &Profile) -> Vec<Place>;

    /// What one file or database tells about its sessions: the id, and
    /// the folder and the title when the file names them. The host joins
    /// what all files tell about one id.
    fn read_sessions(profile: &Profile, item: &PlaceItem) -> Result<Vec<Session>, ReadError>;

    /// What holds the profile's answers: file globs or a database. The
    /// host walks them and asks again only for what changed.
    fn spend_places(profile: &Profile) -> Vec<Place>;

    /// Every answer of the model in one file or database, of the whole
    /// history, each with its session and project, with the counts of
    /// records seen and understood. OTTY sums the totals and the
    /// statistics of a session and of a project from these rows.
    fn read_answers(profile: &Profile, item: &PlaceItem) -> Result<Answers, ReadError>;

    /// The command that starts a new session, or continues `session`.
    fn launch(profile: &Profile, session: Option<&SessionId>) -> Command;

    /// What the user must do now that a provider part refused `login` of
    /// the profile (`ReadError::LoginRefused`), such as run the agent so
    /// it renews the login. Default: none, and OTTY says the login was
    /// refused, with no button.
    fn login_refused(_profile: &Profile, _login: &LoginRef) -> Option<Need> { None }
}

/// One company that sells model use: whose account a key is and its
/// numbers. `provider` says which provider part of the manifest is asked.
pub trait Provider {
    /// Whose account `key` is: account or organization id, email, plan.
    fn account(provider: &ProviderId, key: &Key, found: &LoginFacts)
        -> Result<AccountFacts, ReadError>;

    /// The account's numbers right now. Each part is optional. `given`
    /// holds the keys the user gave for the account after an `AskKey`
    /// need of the part, each under its slot.
    fn numbers(provider: &ProviderId, account: &AccountFacts, key: &Key,
               given: &[GivenKey]) -> Result<AccountNumbers, ReadError>;

    /// The bill of an organization over the UTC days `days`, read with the
    /// account's `key` or a key the user gave. A part that needs a key it
    /// has not got answers `ReadError::NeedsUser` with an `AskKey` need.
    /// The part walks the pages; OTTY makes one day of each. Default: none.
    fn read_bill(_provider: &ProviderId, _account: &AccountFacts, _key: &Key,
                 _given: &[GivenKey], _days: &DayRange)
        -> Result<Bill, ReadError> { Err(ReadError::NotSupported) }
}

/// The optional language receiver of the whole plugin, not of a part.
pub trait Language {
    /// The user changed the language of OTTY's interface to `tag`. `true`
    /// when the plugin's texts depend on it: OTTY reads its parts again.
    fn language_changed(tag: &str) -> bool;
}

/// One thing only the user can do, with one button, in the language of
/// `env::language()` or in English.
pub struct Need {
    pub id: String,                 // stable within the part
    pub title: String,
    pub text: String,
    pub action: NeedAction,
}

pub enum NeedAction {
    Run(Command),                   // in a terminal tab: `codex login`, or the agent itself
    AskKey(KeyAsk),                 // slot (none: the account's own key), label, hint, prefix, page
    OpenPage(String),               // `https` only
    CopyReport(String),
}

/// Why a part could not give what OTTY asked for.
pub enum ReadError {
    NotSupported,
    Host(HostError),
    Unexpected(String),
    NeedsUser(Need),                // the part can not read until the user does this
    LoginRefused,                   // the provider refused a login an agent keeps
    Failed(String),
}

/// A login or key found in a profile.
pub struct Login {
    pub kind: LoginKind,            // Subscription, ApiKey, GatewayToken
    pub for_whom: ProviderRef,      // Id("openai") or Host("api.deepseek.com")
    pub place: KeyPlace,            // where the host reads the key again
    pub identity: Option<Identity>, // account id, organization id or email, when the file says
    pub plan: Option<String>,
}

/// Where a key sits. The host reads it there right before each use.
pub enum KeyPlace {
    JsonField { file: String, pointer: String },  // "/tokens/access_token"
    EnvVar { name: String },
    Keyring { service: String, user: String, pointer: Option<String> },
    SqliteField { database: String, table: String, column: String, row_id: i64,
                  pointer: Option<String> },      // a field of a JSON column
}

/// One answer of the model.
pub struct Answer {
    pub identity: Option<String>,   // the host counts answers with the same identity once
    pub session: Option<String>,    // the id `read-sessions` gives; the host sums a session from it
    pub at_ms: i64,
    pub model: Option<String>,
    pub provider: Option<ProviderRef>,
    pub login: Option<LoginRef>,    // when the agent records which login served it
    pub project: Option<String>,
    pub tokens: TokenCount,         // input, cached, cache_written, output, reasoning
    pub cost_microdollars: Option<u64>, // when the agent writes the cost
}

/// What one read gives, and what it saw.
pub struct Answers {
    pub answers: Vec<Answer>,
    pub records: RecordCounts,      // seen, understood, version, missing and found field names
}

/// The numbers of an account: a subscription has windows, credits and
/// extra usage; a key the money left, its own limit, the money spent this
/// month and credits.
pub struct AccountNumbers {
    pub windows: Vec<LimitWindow>,  // "62 % used", resets at
    pub credits: Option<Credits>,
    pub extra_usage: Option<ExtraUsage>, // spent this month of a monthly limit
    pub left: Option<Money>,
    pub key_limit: Option<KeyLimit>,
    pub spent_this_month: Option<Money>,
    pub plan: Option<String>,
}
```

The helpers are plain functions: `files::list`, `files::read`,
`files::lines(path, markers)` (an iterator over batches),
`files::json_with_keys(path)`, `sqlite::query`, `http::request`,
`programs::run`, `programs::talk`, `terminal::start`, `keyring::read`,
`env::var`, `env::language`, `env::in_language(texts)` (one of the
part's translations for the language now, else English), `clock::now`,
`log::warn`, and `given_key(given, slot)`. A `Key` has `fingerprint()`,
`has_prefix(text)` and `jwt_claims()`, and goes into a request as
`request.header_with_key("authorization", "Bearer ", &key)`.

A plugin ends with one line that names the extension points it exports,
each by its domain; they must match its `[api]`:

```rust
otty_plugin_sdk::export!(OpenAi with agents::agent, agents::provider, language);
```

Inside, the SDK keeps one generated binding per contract; when the
component is made, the worlds of the bindings a plugin uses are joined,
so a set of traits is not a contract and has no version of its own.

## The first part: Codex

**Decision: the Codex agent part of the OpenAI plugin is the first part
built on the SDK, and DeepSeek is not.** Reasons:

- The spike already proved the easy path (line-by-line JSONL of Claude
  Code: identical totals, 1.3x native). What can still change the agent
  contract and the host operations is in Codex: state carried across
  lines (the previous session total, the model of the turn), zstd files,
  10.8 GB of files, a line program (`codex app-server`) that must never
  get `thread/list`, login tokens that are JWTs. Finding this after five
  plugins are written means rewriting five plugins.
- The OpenAI plugin was embedded in the binary, so it also proved the
  embedded path (2026-11-20; since issue 28 OTTY embeds no plugin).
- DeepSeek would test HTTP and key handles end to end, but both are
  generic host operations with unit tests, and the OpenAI provider part
  (issue 15) tests them again with a real paged API.

The template repository (issue 03) holds a small example plugin over
made-up files, so the SDK has a working plugin before Codex.

A sketch of the Codex token reader (the state that crosses lines lives in
one plain struct for the whole file; the host has already left out the
lines without a marker and decoded `.zst`):

```rust
use otty_plugin_sdk::agents::{Agent, Answer, Answers, PlaceItem, RecordCounts, TokenCount};
use otty_plugin_sdk::files;

const MARKERS: [&[u8]; 3] = [b"\"token_count\"", b"\"turn_context\"", b"\"session_meta\""];

/// What a session file has said so far: its project, the model of the
/// current turn and the session total of the last token_count.
#[derive(Default)]
struct FileState {
    project: Option<String>,
    model: Option<String>,
    last_total: Option<TokenCount>,
    answers: Vec<Answer>,
    records: RecordCounts,
}

impl Agent for OpenAi {
    fn read_answers(_profile: &Profile, item: &PlaceItem) -> Result<Answers, ReadError> {
        let mut state = FileState::default();

        for batch in files::lines(&item.file.path, &MARKERS) {
            for line in batch?.lines() {
                state.add_line(line);
            }
        }

        Ok(Answers { answers: state.answers, records: state.records })
    }
    // profiles, logins, sessions, launch, ...
}

impl FileState {
    fn add_line(&mut self, line: &[u8]) {
        let Ok(record) = serde_json::from_slice::<Line>(line) else {
            self.records.add_broken();
            return;
        };

        match record.kind() {
            Some("session_meta") => self.project = record.cwd(),
            Some("turn_context") => self.model = record.model(),
            Some("event_msg") => self.add_token_count(&record),
            _ => {}
        }
    }
}
```

## WIT sketch

```wit
// otty-plugin-sdk/wit/host/0.1.0/host.wit
package otty:host@0.1.0;

interface types {
  /* host-error, command: what any domain could use */
  record key-ask { slot: option<string>,   // none: replaces the account's own key
                   label: string, hint: string,
                   prefix: option<string>, page: option<string> }
  variant need-action { run(command), ask-key(key-ask), open-page(string),
                        copy-report(string) }
  /// One thing only the user can do; texts in the language of `env.language`.
  record need { id: string, title: string, text: string, action: need-action }
}

interface keys {
  /// A key or a login token the plugin may use but never read.
  resource key {
    fingerprint: func() -> string;            // SHA-256, hex
    has-prefix: func(text: string) -> bool;
    jwt-claims: func() -> option<string>;     // JSON of the middle part, never the token
    given-in-otty: func() -> bool;            // given on Add account or for ask-key, not read from an agent
  }
  variant key-place {
    json-field(tuple<string, string>),        // file, JSON pointer
    env-var(string),
    keyring(tuple<string, string, option<string>>),
    sqlite-field(tuple<string, string, string, s64, option<string>>),
  }
  /// A key the user gave for an ask-key need, under its slot; a handle for one call.
  record given-key { slot: string, key: key }
  /// The key at `place`, read now; it must be a key place of the called part.
  open: func(place: key-place) -> result<key, host-error>;
}

interface files {
  record file-info { path: string, size: u64, changed-ms: s64 }
  record batch { lines: list<u8>, next-offset: u64, skipped: u32, at-end: bool }
  list: func(glob: string) -> result<list<file-info>, host-error>;
  read: func(path: string, max-bytes: u32) -> result<list<u8>, host-error>;
  /// Complete lines from `offset` holding one of `markers`; zstd decoded.
  read-lines: func(path: string, offset: u64, max-bytes: u32,
                   markers: list<list<u8>>) -> result<batch, host-error>;
  /// A JSON file of a key place, its key fields replaced by handles.
  read-json-with-keys: func(path: string) -> result<json-with-keys, host-error>;
  remove: func(path: string) -> result<_, host-error>;   // needs `write`
}

interface sqlite {
  variant value { null, integer(s64), real(f64), text(string), blob(list<u8>),
                  key(own<key>), json-with-keys(json-with-keys) }
  /// One read-only SELECT; key columns come back as `key`, JSON key
  /// columns with their key fields replaced by handles.
  query: func(database: string, sql: string, params: list<value>)
        -> result<list<list<value>>, host-error>;
}

interface http {
  record header { name: string, value: string, key: option<borrow<key>> }
  record request { method: string, url: string, headers: list<header>, body: option<list<u8>> }
  record response { status: u16, headers: list<tuple<string, string>>, body: list<u8> }
  request: func(req: request) -> result<response, host-error>;
}

interface programs {
  record output { status: option<s32>, stdout: list<u8>, stderr: list<u8> }
  run: func(program: string, args: list<string>, env: list<tuple<string, string>>,
            timeout-ms: u32) -> result<output, host-error>;
  resource talk {           // a program spoken to line by line
    constructor(program: string, args: list<string>, env: list<tuple<string, string>>);
    send-line: func(line: string) -> result<_, host-error>;
    next-line: func(timeout-ms: u32) -> result<option<string>, host-error>;
  }
}

interface terminal {
  working-folders: func() -> result<list<string>, host-error>;  // where every hidden terminal starts
  resource hidden-terminal {
    constructor(program: string, args: list<string>, env: list<tuple<string, string>>,
                cols: u16, rows: u16);
    send-keys: func(keys: string) -> result<_, host-error>;
    screen: func() -> result<string, host-error>;           // the visible text
    wait-for-change: func(timeout-ms: u32) -> result<bool, host-error>;
  }
}

interface keyring { read: func(service: string, user: string) -> result<option<json-with-keys>, host-error>; }
interface env     { var: func(name: string) -> option<string>; home: func() -> string;
                    language: func() -> string; }   // the language shown now, such as `pt-BR`
interface clock   { now-ms: func() -> s64; }
interface log     { write: func(level: level, text: string); }
```

```wit
// otty-plugin-sdk/wit/agents-agent/0.1.0/types.wit
package otty:agents-agent@0.1.0;

/// The values of the `agents` domain both its contracts share.
interface types {
  use otty:host/types@0.1.0.{host-error, need};
  variant read-error { not-supported, host(host-error), unexpected(string),
                       needs-user(need), login-refused, failed(string) }
  /* identity, login-kind (subscription, api-key, gateway-token),
     limit-window, credits, limits */
}
```

```wit
// otty-plugin-sdk/wit/agents-agent/0.1.0/agent.wit
package otty:agents-agent@0.1.0;

interface agent {
  use types.{read-error, identity, login-kind, limits};
  /* profile, profile-status (connected, not-logged-in(need),
     config-folder-not-found), login, session (id, and the folder and
     title when the file names them), place (glob, kind), place-item
     (the listed file-info, kind), answer (with its session and
     project), answers, record-counts */
  profiles: func(agent: string, env: environment) -> list<profile-place>;
  detect: func(profile: profile) -> profile-status;
  version: func(profile: profile) -> option<string>;
  logins: func(profile: profile) -> list<login>;
  session-places: func(profile: profile) -> list<place>;
  read-sessions: func(profile: profile, item: place-item) -> result<list<session>, read-error>;
  spend-places: func(profile: profile) -> list<place>;
  read-answers: func(profile: profile, item: place-item) -> result<answers, read-error>;
  launch: func(profile: profile, session: option<string>) -> command;
  login-refused: func(profile: profile, login: login-ref) -> option<need>;
}

/// What a component with agent parts imports and exports.
world agent-part {
  import otty:host/files@0.1.0;  import otty:host/sqlite@0.1.0;
  import otty:host/keys@0.1.0;   import otty:host/http@0.1.0;
  import otty:host/programs@0.1.0; import otty:host/terminal@0.1.0;
  import otty:host/keyring@0.1.0; import otty:host/env@0.1.0;
  import otty:host/clock@0.1.0;  import otty:host/log@0.1.0;
  import types;
  export agent;
}
```

```wit
// otty-plugin-sdk/wit/agents-provider/0.1.0/provider.wit
package otty:agents-provider@0.1.0;

interface provider {
  use otty:host/types@0.1.0.{command};
  use otty:host/keys@0.1.0.{key, given-key};
  use otty:agents-agent/types@0.1.0.{read-error, identity, login-kind, limit-window, credits};
  /* a profile an account was found in (issue 29) */
  record found-profile { plugin: string, agent: string, folder: string, launch: command }
  record login-facts { kind: login-kind, identity: identity, plan: option<string>,
                       found-in: list<found-profile> }
  record extra-usage { used: option<f64>, monthly-limit: option<f64>, currency: option<string> }
  record account-numbers { windows: list<limit-window>, credits: option<credits>,
                           extra-usage: option<extra-usage>, left: option<money>,
                           key-limit: option<key-limit>, spent-this-month: option<money>,
                           plan: option<string>, recorded-at-ms: option<s64> }
  record day-range { first: string, last: string }       // UTC days, YYYY-MM-DD
  account: func(provider: string, key: borrow<key>, found: login-facts) -> result<account-facts, read-error>;
  numbers: func(provider: string, found: login-facts, account: account-facts, key: borrow<key>,
                given: list<given-key>) -> result<account-numbers, read-error>;   // every way it can
  read-bill: func(provider: string, found: login-facts, account: account-facts, key: borrow<key>,
                  given: list<given-key>, days: day-range) -> result<bill, read-error>;
}

/// What a component with provider parts imports and exports.
world provider-part {
  import otty:host/keys@0.1.0;     import otty:host/files@0.1.0;
  import otty:host/http@0.1.0;     import otty:host/programs@0.1.0;
  import otty:host/terminal@0.1.0; import otty:host/env@0.1.0;
  import otty:host/clock@0.1.0;    import otty:host/log@0.1.0;
  import otty:agents-agent/types@0.1.0;
  export provider;
}
```

```wit
// otty-plugin-sdk/wit/language/0.1.0/language.wit
package otty:language@0.1.0;

interface language {
  /// The user changed the language to `tag`; true when the plugin's texts
  /// depend on it, and OTTY reads its parts again at once.
  language-changed: func(tag: string) -> bool;
}

world language-receiver { export language; }
```

Each contract is its own package with its own version; there is no world
that joins them. A component exports `otty:agents-agent/agent@0.1.0`,
`otty:agents-provider/provider@0.1.0` or both, and may also export
`otty:language/language@0.1.0`, which imports nothing. The host reads which contracts and
versions a component exports from its export names (wasmtime resolves
semver-compatible imports itself), checks them against the manifest's
`[api]`, and refuses a plugin that names a version it does not have. The
host operations have their own version (`otty:host@0.1.0`); each contract
version names the host version it imports, so the manifest does not list
it.

## Host operations

Every operation checks the permissions of the part being called, on each
call, not only at install. A denied call returns `host-error::not-allowed`
with the path or host, and Plugins mode lists the last denied calls of
each part.

| Operation | What it does | Permission | Limits |
| --- | --- | --- | --- |
| `files.list(glob)` | Paths, sizes and change times | `read` | At most 100,000 entries |
| `files.read(path, max)` | The bytes of a small file | `read`; refused for a key file | At most 4 MiB |
| `files.read-lines(path, offset, max, markers)` | Complete lines from an offset, only those with a marker (`memchr::memmem`), zstd decoded | `read` | About 1 MiB per batch |
| `files.read-json-with-keys(path)` | A key file as JSON, key fields replaced by handles | `keys` (file) | At most 4 MiB (Claude Code's `.claude.json` holds its project list; issue 13) |
| `files.remove(path)` | Deletes one file (and an empty folder) | `write` | Never outside the globs |
| `sqlite.query(db, sql, params)` | One read-only `SELECT`; a declared key column in the result comes back as a handle, a JSON key column as JSON with handles in its key fields (rule below) | `sqlite` (+ `keys` for key columns) | Read-only open, `query_only`, no `ATTACH`, 10 s, 50,000 rows |
| `keys.open(place)` | A key handle for a declared key place | `keys` / `env` / `keyring` | Read again right before each use |
| `key.fingerprint`, `has-prefix`, `jwt-claims` | Use a key without reading it | none (needs a handle) | `jwt-claims` never returns the signature |
| `key.given-in-otty` | Whether the user gave the key in OTTY, not where | none (needs a handle) | The key is not read |
| `http.request(req)` | HTTPS request; a key handle in a header is filled in by the host | `http` hosts | No redirects, 30 s, 8 MiB answer, OTTY `User-Agent` |
| `programs.run(program, args, env)` | Runs a program once and returns its output | `run` (program + argument pattern) | 10 s, 1 MiB output, only listed variables passed |
| `programs.talk(...)` | Talks to a program line by line (`codex app-server`) | `run` | Stopped with the call; 60 s in all |
| `terminal.hidden-terminal(...)` | Starts a program in an unseen terminal, sends keys, reads screen text | `terminal` | OTTY's own working folder; stopped with the call; 60 s |
| `terminal.working-folders()` | OTTY's working folder of hidden terminals, also as the system resolves its links (issue 13) | `terminal` | |
| `keyring.read(service, user)` | One keyring entry as JSON with key fields as handles | `keyring` + the user's Allow (spec `2026-11-19-agents-accounts`, Decision 8) | Read only |
| `env.var(name)` | One environment variable as text | `env` | Key variables only through `keys.open` |
| `env.language()` | The language of OTTY's interface shown now, as a tag (`en`, `ru`, `pt-BR`) | none | May change between two calls |
| `clock.now-ms` | The time | none | |
| `log.write(level, text)` | A line in the plugin's log, shown in Plugins mode | none | 64 KiB per plugin, oldest dropped |

Key columns in `sqlite.query`. The plugin's SQL never sees a key: SQLite
reads every declared key column as `NULL` wherever the statement uses it
(in `WHERE`, a join, a function, a subquery, a view). A result column
that is the key column itself (the column, under any alias, also through
a subquery) comes back as a `key` value, or as `json-with-keys` for a
column declared with key fields, so the plugin can read the other fields
of a JSON column such as OpenCode's `credential.value`. The host reads
that key at the row named by the table's rowid, so the result must also
hold the rowid of that table (`rowid`, or its `INTEGER PRIMARY KEY`);
without it the query fails. A statement that names a key column outside
its result columns, such as `WHERE value LIKE 'sk-%'` or `upper(value)`,
is refused, so the plugin learns at once that such SQL cannot see the
key. A key column with no key in it, or a JSON key column that is not
JSON, comes back as `null`.

What the host does with the results, with no agent or provider in mind:
walks session and spend places and re-reads only files whose size or
change time changed (today's `FileMemory`, in memory only); joins what
the files tell about one session id (`session_files.rs`: the latest
time, and the folder and the title of the latest record naming them);
adds up answers by day,
hour, project, model, provider and login (`spend_tally.rs`); counts an
answer identity once across files (`CountedAnswer::identity`); gives the
"format changed" verdict from the newest files' seen and understood counts
(`record_format.rs`); prices tokens with LiteLLM (`price_source.rs`,
`money_estimate.rs`); joins accounts (`merge_accounts`, key fingerprints);
reads each account once per refresh, not per profile.

## Permissions

- **What is listed, per part.** Agent part: `read` and `write` globs;
  `sqlite` database globs; `keys` (JSON fields of files, SQLite columns
  and fields of JSON columns, environment variables); `env` variables;
  `run` programs with argument patterns; `terminal` programs; `keyring`
  entries. Provider part: `http` hosts. A provider part gets key handles
  only for accounts of its own provider id or `hosts`: keys at the places
  agent parts name, keys added by hand for it, keys the user gave for its
  `ask-key` needs (issue 20), and keys
  OTTY found only in the keyring (after the user's Allow). A key OTTY
  reads itself (added by hand, or from the keyring) is read once for one
  read of the part and dropped after it (issues 16 and 15).
- **Applied by context.** While OTTY runs a function of one part, only
  that part's permissions apply; the permissions of the plugin's other
  parts are refused like any undeclared path or host.
- **How it is shown.** In plain words, grouped by part, with `~` for home:
  "Codex (agent): Reads files in `~/.codex*/**`", "Reads your login in
  `~/.codex/auth.json` (OTTY gives it only as a handle)", "Runs `codex
  --version`", "Talks to `codex app-server`"; "OpenAI (provider): Sends
  requests to `chatgpt.com`", "Gets the logins and keys of OpenAI accounts
  found in your agents, as handles"; "Claude Code (agent): Deletes files
  in `~/.claude/projects/<OTTY probe folder>/*.jsonl`".
- **When it is shown.** Before install; before an update that adds any
  permission or part (the new ones are marked "new"; removed ones listed
  as "no longer"); at any time on the plugin's page.
- **What the user grants.** The whole list of the plugin, or nothing. The
  user can then turn single parts off. A plugin whose update needs more
  keeps running its old version until the user agrees.
- **What OTTY does not limit.** Inside its extension point and its
  permissions, the part decides: which declared host gets which key,
  which program methods it calls, what it writes in its declared folders.
  The UI marks official plugins; for others the install screen says
  "Community plugin: reviewed by its author only".
- **Keyring.** A part with `keyring` entries works without them until
  the user allows the keyring once in Agents mode (as today).

## Install and update flow

1. **Add.** Plugins mode → **Add plugin…** → the user pastes a URL such as
   `https://github.com/otty-shell/otty-plugin-opencode`. Only GitHub URLs are
   accepted at first.
2. **Find the version.** OTTY lists the repository's releases (GitHub
   REST API, no login, `User-Agent: OTTY`), keeps tags that are semver,
   and for the newest first downloads only `otty-plugin.toml`. It takes
   the newest release whose contract versions in `[api]` this OTTY all
   knows. No semver tag: "This repository has no release OTTY can
   install". None compatible: "Needs a newer OTTY (agent contract 0.2)".
3. **Check the manifest.** Its plugin id is not taken by another installed
   plugin (refused: "A plugin `opencode` is already installed from
   <repository>"); globs are valid and inside home or listed variables;
   icons exist; every part has its contract in `[api]`. A part whose ids
   or hosts another installed plugin serves too is no conflict (issue
   29).
4. **Show.** A dialog with name, author, repository link, version,
   "Official" or "Community", and each part with its permissions in words.
   Buttons **Install** and **Cancel**.
5. **Download.** The `.wasm` and the icons. OTTY computes SHA-256 of each
   asset and checks the `.wasm`: it exports exactly the contracts and
   versions of `[api]`, only allowed imports, at most 20 MiB; icons parse
   as SVG, have no scripts or external links, at most 16 KiB.
6. **Compile and keep.** The files go to
   `~/.local/share/otty/plugins/<id>/<version>/`; the compiled copy to
   `~/.cache/otty/plugins/<sha256>-<engine hash>.cwasm`. `plugins.json` in
   `~/.config/otty/` records the URL, version, asset hashes, the granted
   permissions of each part and each part on or off; every part is on.
7. **Run.** The registry gets the agents and providers of the parts that
   are on. Agents mode reads again; new profiles and accounts appear.

**Updates.** Once a day and on **Check for updates**, OTTY lists the
releases of each installed plugin and takes the newest one whose contract
versions it all knows. With no new permissions and no new part, OTTY
installs it on its own; the plugin's row says "Updated to 0.5.0". With new
permissions or a new part, the need "Plugin update needs your OK" opens
the permission diff, and the installed version keeps running until the
user agrees. A newer release that needs unknown contract versions is
skipped, and the plugin's row says "0.6.0 needs a newer OTTY". The update
installs next to the old version, switches, then removes the old folder;
parts keep their on or off state, and a new part is on. A tag whose
assets now hash differently from what was recorded: the update is refused
with "The release v1.2.0 of OpenCode changed after it was installed. OTTY
keeps the installed copy."

**No plugin ships with OTTY** (issue 28, 2026-11-21; this replaces
"Embedded plugins"). The binary carries no plugin and a start installs
none: the first start has an empty Plugins mode, and Agents mode with no
agent part on says so in one line with a button that opens the Plugins
page. The Found by OTTY report waits for the first read with an agent
part on, and is not marked shown before it was shown. The official
plugins are installed like any other plugin: with **Add plugin from
folder…** from `assets/plugins/<id>/` of a checkout of this repository
now (marked "Local"), with **Add plugin…** from their own repositories
once they have them (marked "Official"). An old `plugins.json` with
`"source": "embedded"` is not read; Agents mode was never released, so
there is no migration.

**Local plugins.** **Add plugin from folder…** takes a folder holding
`otty-plugin.toml`, the built `.wasm` and the icons. Steps 3 to 7 run as
for a release, with the files copied from the folder instead of
downloaded; `plugins.json` records the folder instead of a URL. The
plugin is marked "Local" (never "Official"), is left out of the update
check, and **Reload** copies the folder again, checks it again (showing
the permission diff when it asks for more) and switches to the new copy.

**Remove / turn off.** Turning a part off keeps its files and saved
settings and stops that part only; Remove deletes the plugin's folder and
compiled copy; saved names and hidden items in `agents.json` stay (see
"Links in the interface").

## How plugins talk to OTTY and to the outside world

```
 ┌──────────────────────────── OTTY process ────────────────────────────┐
 │                                                                      │
 │  Agents mode UI ── reads names, icons ──► Registry (agents, providers)│
 │       ▲                                          ▲                   │
 │       │ intents                                  │ fills             │
 │  otty-app-agents-io (background thread)          │                   │
 │       │ walk, tally, dedupe, join accounts,      │                   │
 │       │ "format changed", prices                 │                   │
 │       ▼                                          │                   │
 │  otty-agents ── PluginAgent / PluginProvider ────┘                   │
 │       │  calls one part, with a deadline                             │
 │       ▼                                                              │
 │  otty-plugin-host (wasmtime, 64 MiB, epochs, empty WASI)             │
 │   ┌────────────────────┐  host operations  ┌─────────────────────┐   │
 │   │ plugin .wasm        │ ───────────────► │ permission check    │   │
 │   │ (OpenAI, OpenCode…) │ ◄─── results ─── │ of the called part  │   │
 │   │ sees bytes, rows,   │   key handles,   │ (globs, hosts,      │   │
 │   │ screen text; never  │   never keys     │  programs)          │   │
 │   │ a key               │                  └──────────┬──────────┘   │
 │   └────────────────────┘                             │              │
 └──────────────────────────────────────────────────────┼──────────────┘
                                                        │
        ┌──────────────┬──────────────┬─────────────────┼──────────────┐
        ▼              ▼              ▼                 ▼              ▼
   agents' files   SQLite DBs    HTTPS hosts      programs and     system
   (read; write    (read-only)   (only listed;    hidden terminal  keyring
   only declared)                key filled in    (only listed)    (after Allow)
                                 by the host)
```

A typical read of one account: the `codex` part of the OpenAI plugin
reports a ChatGPT login at `~/.codex/auth.json` field
`/tokens/access_token` for provider `openai` → OTTY joins it with the same
login found in OpenCode → OTTY opens a key handle at one of those places →
the `openai` provider part of the same plugin gets the handle (and may not
read `~/.codex` itself), puts it into the `Authorization` header of a
request to `chatgpt.com` → the host fills in the token, sends, returns the
answer → the part reads the limits from the JSON → the host drops the
token.

## Engine limits

| Limit | Value | Why |
| --- | --- | --- |
| Memory per plugin | 64 MiB, trap when it can not grow | Spike: 7.5 MiB peak on real files |
| Instances per store | 4 | One component, a few inner instances |
| Deadline, quick calls (`detect`, `version`, `logins`, `launch`, `profiles`) | 2 s | |
| Deadline, `read-answers`, `read-sessions` | 120 s per call | One 1 GB file at 400 MB/s plus margin |
| Deadline, provider calls | 90 s | Host waits (HTTP, terminal) count |
| Epoch tick | 10 ms (spike: 1.005 s for 1 s) | |
| Plugin state between refreshes | none: a new instance per refresh of one profile or account (0.1 ms) | Nothing leaks between reads |
| WASI | Empty context; stderr to the plugin log | |
| Allowed imports | `wasi:cli/*`, `wasi:io/*`, `otty:*` | Checked at install and at load |
| Compiled cache | `~/.cache/otty/plugins/`, written only by OTTY | 0.5 s compile, 0.3 ms load |
| Traps | A trap fails that call only; three in a row pause the part whose calls failed, until restart or **Try again**; the plugin's other parts keep working | |

The deadline counts the whole call, host operations included; a host
operation that waits (HTTP, program, terminal) has its own timeout from the
table above.

## Links in the interface

### What replaces the `Provider` enum and the agent constants

- **Provider**: a `ProviderId` (a string such as `"deepseek"`, from the
  manifest; stable; changing it makes a new part) and a `ProviderEntry`
  in the registry: name, icon (SVG bytes from the plugin package), usage
  page, hosts it serves, whether Add account offers it (`add_by_hand`),
  and the plugin and part that serve it (plugin id, version, repository).
- **Agent**: `AgentId` becomes an owned string from the manifest (today it
  wraps `&'static str`), and `AgentDescriptor` becomes an `AgentEntry`:
  name, icon, program, the plugin and part. What today's descriptor has as
  flags (`has_bill`, `has_stats`, `writes_cost`) comes from what the part
  answers (`NotSupported` is "no").
- **The registry** is one value in `otty-agents`, filled by the
  plugin engine at start and refilled when a plugin is installed,
  updated or removed, or a part is turned on or off. It holds only parts
  that are on: one part per agent id, and every provider part, so a
  provider two plugins serve has two entries, each reading its own copy
  of the provider's accounts (issue 29), and so does a host two plugins
  serve under two provider ids (issue 31).
  The app gets it as an `Arc<Registry>` with the agents mode's context.
  App crates read names, icons, usage pages and hosts only through it.
  `otty-app-theme`'s `agent_icon` and `provider_icon` are removed; the kit
  draws an SVG from bytes, or a **letter icon** (the first letter of the
  name or host on a rounded square) when there is no entry.
- During the migration the in-core readers register themselves in the
  same registry as "built in" entries with today's names and the icons of
  `assets/svg/`, so the UI switches over before any agent is ported.

### Identity and links do not depend on a provider plugin

- An account's identity is (provider reference, account id or
  organization id or email or key fingerprint). The provider reference is
  the provider id when the agent part named one, else the host the key is
  for, normalized (`api.deepseek.com`). When provider parts later claim
  that host, every one of them reads the account, one copy under the id
  of each part (issue 31). Saved entries under `host:api.deepseek.com`
  are the choices of the account no plugin reads; the copies the parts
  read have their own.
- Keys are fingerprinted by the host, so joining accounts never needs a
  provider part.
- **Pays through** (profile → accounts) and **Used by** (account →
  profiles) come from the agent part's `logins` and OTTY's join. They stay
  when the provider part is missing or off.
- Tokens come from the agent part; money estimates come from LiteLLM
  prices in the host. Both work with no provider part.

### What the UI shows

| Situation | Accounts | Profiles | Needs and messages |
| --- | --- | --- | --- |
| No provider part installed | Listed by provider id or host, letter icon; Used by, tokens and money from LiteLLM prices with the mark "estimate, list prices"; no Left, Limits, Key limit or bill rows; Account info "Read by: no plugin" | Pays through chips stay, with letter icons | Account need "Key for api.deepseek.com found in Claude Code: no provider plugin [Add plugin…]"; Add plugin opens Plugins mode with the URL field empty |
| Provider part off | As above, Account info "Read by: DeepSeek 0.2.0 (off)" | Same | Need "DeepSeek of the DeepSeek plugin is off [Turn on]" |
| Agent part off, or its plugin removed | Accounts found only in its profiles leave the lists; accounts also found elsewhere stay, without its chips | Its profiles leave the lists | None. Names and hidden items in `agents.json` (keyed by agent id + folder) are kept and come back when the part is on again |
| Plugin updated to a new version | Nothing moves: ids are stable, names and hidden items stay | Same | "Read by" shows the new version |
| A part fails, traps or times out | Last good numbers greyed with their time | Same | Need "<Plugin> <version> could not read <what>" with **Copy report** (plugin id, version, repository, part id, the call, the error kind; no values). Three in a row pause that part only |
| A part says format changed | Same as today, greyed | "format changed" | The need names the agent version and the plugin: "Codex 0.170.0 changed its session files. OpenAI plugin 0.4.0 reads them up to 0.169.0." and **Copy report**, **Open plugin repository** |
| A part's provider id or provider host is served by another plugin too | One account per plugin that reads it, under that part's provider id, each named with its plugin; the user hides the one they do not want, each copy alone, an added account too (issues 29 and 31) | Same | Nothing is asked. An agent id two plugins serve is read by the first installed; the other part can be turned off |
| Plugin update with new permissions | — | — | Plugins mode sign and the need "Plugin update needs your OK: OpenAI 0.4.0 → 0.5.0 asks for new permissions [Review]" |
| Newest release needs a newer OTTY | Nothing moves; the installed version keeps running | Same | The plugin's row: "0.6.0 needs a newer OTTY". At install the whole plugin shows "Needs a newer OTTY" |

### `agents.json`

No compatibility with earlier agents.json: Agents mode was never released.

## Migration

| Today in `crates/agents/src/` | Goes to |
| --- | --- |
| `walk.rs`, `spend_files.rs`, `spend_tally.rs`, `file_memory.rs`, `file_stamp.rs`, `record_format.rs` (verdict), `price_source.rs`, `litellm.rs`, `money_estimate.rs`, `key_fingerprint.rs`, `account.rs::merge_accounts`, `saved_profiles/`, `profile_list.rs` | Stay in OTTY, made agent-free |
| `jsonl.rs`, `json_file.rs`, `https.rs`, `program.rs`, `hidden_terminal.rs`, `system_keyring.rs` | Become host operations in `otty-plugin-host` |
| `provider.rs` (`Provider` enum, `PROVIDER_DOMAINS`), `agent.rs` (`AgentDescriptor` consts) | Registry entries from manifests (issue 01) |
| `codex/*` except `usage.rs` | OpenAI plugin, agent part `codex` (issues 10, 11) |
| `codex/usage.rs`, `rate_limits.rs` for ChatGPT, `workspace_credits.rs` | OpenAI plugin, provider part `openai` (issue 15, done) |
| `codex/jwt.rs`, `token_expiry.rs` | Host `key.jwt-claims`; expiry read by the plugin from claims (issue 15, done) |
| `claude_code/*` except `usage.rs`, `cost_report.rs` | Anthropic plugin, agent part `claude-code` (issues 12, 13) |
| `claude_code/usage.rs`, `usage_answer.rs` | Anthropic plugin, provider part `anthropic` (issue 15, done) |
| `claude_code/cost_report.rs`, `account_numbers/openai_costs.rs` | Provider parts `anthropic` and `openai` (issue 15, done) |
| `claude_code/probe_memory.rs` | Host scheduling ("at most once in five minutes") |
| `open_code/*`, OpenCode only | OpenCode plugin, agent part `opencode` (issue 14) |
| `open_code/` Kilo variant (`fork.rs` `KILO`, Kilo's `account.json` logins), `account_numbers/kilo.rs` | Deleted, not ported (issues 14, 17) |
| `account_numbers/deepseek.rs`, `openrouter.rs` | DeepSeek and OpenRouter plugins (issue 16) |
| `account_numbers/moonshot.rs` | Deleted, not ported (issue 17) |
| `keyring_entry.rs` (Goose entry, `KEY_VARIABLES`) | Deleted, not ported (issue 17); `system_keyring.rs` becomes the keyring host operation |

**No parity check.** The issue that brings a plugin part deletes the
in-core code it replaces; the owner checks the numbers by hand.

Order: registry first (the UI switches over while the core still reads);
then the engine, SDK and host operations; then the plugin adapters; then
install and Plugins mode; then the agent parts
Codex (OpenAI plugin) and Claude Code (Anthropic plugin), then the
OpenCode plugin; then the provider parts and provider plugins;
then the cleanup and the docs.

## UI changes

- **Plugins mode** (issue 30), a mode like Agents mode, not pinned by
  default. Left panel: the installed plugins (icon, name, "Official" /
  "Community" / "Local", and a sign "failed" when a part is paused or
  the latest check or change failed, "update" when an update waits,
  "off" when every part is off), a search field that keeps the plugins
  whose names hold the text, letter case aside, and in the header `+`
  (**Add plugin…** with the URL field) and a folder (**Add plugin from
  folder…**). Main area: the page of the selected plugin: name, version,
  description, who reviewed it, authors, repository (a link that opens
  it), source; its parts, each with its kind, an on/off switch, its
  health ("Working", "Off", "Paused after three failures" with Try
  again), its last failure and its permissions; the news of the latest
  check and the waiting update with Review; Check for updates (Reload for
  a local plugin), Show log, Remove. With nothing installed or nothing
  selected a card says what to do and offers both ways to add a plugin.
  Dialogs over the window: the install dialog with permissions per part,
  the update dialog with the permission diff per part, the log, the
  question before Remove; errors in plain words. Settings' **Manage
  plugins…** closes Settings and opens the mode with nothing selected;
  "Read by" selects its plugin; Add plugin… of a need opens the URL
  field; Review selects the plugin and opens its update dialog. A plugin
  removed while selected leaves nothing selected; a plugin just
  installed is selected.
- **Account info** and **Profile info**: a row "Read by: <plugin>
  <version>" linking to its page in Plugins mode; "no plugin" for an
  account whose provider has no part.
- **No "Where the numbers come from"** (issue 19): the account and profile
  pages show the time of their numbers, the last good ones greyed with
  why after a failed read, "Needs from you", the mark "estimate, list
  prices" (with a tooltip of the prices' time) under money estimated from
  LiteLLM prices, and "Read by". The provider part that reads the
  account reads its numbers every way it can (issue 29); its ways are
  not shown as sources.
- **Plugins mode, card of a part**: the last call that failed or that the
  host refused, with its time and kind ("Last failure 5 min ago:
  numbers, the plugin did not finish in time"), never a value, key or
  session text, and **Copy report** (issue 19).
- **Found by OTTY** and **All accounts → Needs you**: the new needs
  "no provider plugin", "part is off", "plugin update needs your OK",
  "plugin could not read", "the login was refused", and the needs the
  parts give, each with its plugin label (issue 20).
- **The key a part asks for**: the `ask-key` button opens the dialog of
  Add account named by the part's label, with its hint, the prefix the
  key starts with and a button to its page; the user gives a file or a
  variable. There is no admin key field or menu item of OTTY's own
  (issue 20). With no slot, the dialog holds the place of the refused key
  of an account the user added and saves the new place in its stead, as
  Edit key… does; for an account an agent keeps it does not open
  (issue 26).
- **Language**: when the user changes it, the needs of plugins with the
  language receiver show in the new language at once (issue 20).
- **Add account**: the providers come from the registry (`add_by_hand`);
  a provider with no part is not offered.
- Everything else keeps the look of spec `2026-11-19-agents-accounts` and
  the prototype `website/src/lite/prototype-accounts/`.

## Crates and dependencies

New crates (each with a `README.md`, per the rules):

- `otty-plugin-host` (`crates/plugins/host`): the wasmtime engine, the
  compiled cache, the manifest and the permissions of each part, the host
  operations and the bindings of every contract version, a module per
  domain (`agents::agent::v0_1`, `agents::provider::v0_1`). Calling a part
  is no domain's business, so it lives here too (owner, 2026-11-21, issue
  21): `CalledPart` (one read is one batch with the part's permissions
  only), `PartHealth` and `LastFailure` (three failed reads in a row
  pause the part until Try again), `PluginFailure` with its general kinds
  and its report, and `PluginPart` (how a part is named in reports). Knows
  no agent and no provider; must not depend on `iced` or `crates/app/`.
- `otty-plugin-sdk` (`crates/plugins/sdk`): the guest side; the WIT files
  live here and the host's bindings read them. Built for `wasm32-wasip2`
  by plugin authors, natively for its own tests. Plugin authors depend on
  it by git tag; crates.io with its first stable version.
- `otty-plugin-releases` (`crates/plugins/releases`): finds, downloads,
  checks and installs releases from a GitHub repository; records hashes;
  compares permissions; holds the one constant with the official GitHub
  organization.
- `otty-app-plugins` and `otty-app-plugins-io` (`crates/app/features/plugins/`):
  Plugins mode and its I/O, following the app's architecture checks.
  They list the parts of any domain and must not depend on `otty-agents`
  (a crate dependency limit in `deny.toml`, issue 21).
- Official plugins are built in `plugins/<id>/` of this repository
  (`anthropic`, `openai`, `opencode`, `deepseek`, `openrouter`,
  and `template`), each its own cargo workspace (as the spike's
  `crates/plugins/claude-code`), outside the OTTY build.
  `plugins/build.sh <folder>` puts the built copy in
  `assets/plugins/<id>/`. The owner moves them to their own repositories
  later.

Dependencies:

| Dependency | Where | State |
| --- | --- | --- |
| `wasmtime` 48 (`runtime`, `cranelift`, `component-model`, `std`), `wasmtime-wasi` 48 (`p2`) | host | Approved |
| `Apache-2.0 WITH LLVM-exception` in `deny.toml` | | Approved |
| `toml` | host | Approved |
| `globset` | host | Approved |
| `memchr` (already in the lock) | host | Approved |
| `rusqlite`, `ureq`, `keyring`, `ruzstd`, `sha2`, `base64` | host (moved from `otty-agents`) | Already approved |
| `wit-bindgen` 0.57 (0.46 is in the lock through another crate) | SDK and plugins | Approved 2026-11-20 |
| `rusqlite` feature `hooks` (an authorizer that refuses `ATTACH` and key columns read as text) | host | Approved 2026-11-20 (a feature, no new crate) |
| `usvg` (already in the lock through iced) to check plugin icons | host | Approved 2026-11-20 as a direct dependency |
| `semver` | 1.0 (already in `Cargo.lock`) | Approved by the owner (2026-11-20): parsing and ordering release tags (an optional leading `v` is stripped first) and plugin versions |
| `serde`/`serde_json`, `jiff` in plugins | plugins | Plugin dependencies; official plugins use the versions in OTTY's lock |

## Out of scope / later

- Other extension points: commands, panels, block actions (the SDK and
  the host are made to add contracts, but none is designed here).
- A catalog or search of plugins, signed indexes.
- Plugins from hosts other than GitHub (local folders are for authors,
  and for the official plugins until they have repositories of their
  own; see "Local plugins").
- Plugins in other languages than Rust.
- A disk cache of per-file numbers.
- Moving the official plugins to their own repositories (the owner does
  it later) and publishing the SDK on crates.io (with its first stable
  version).
- Kimi Code, Moonshot, Kilo and Goose; Crush; desktop agents.
- Windows.

## Open questions for the owner

- Settled on 2026-11-21 (issue 23): `MODEL_MAKERS` is gone. The answer
  names who served it, and OTTY prices it by the row of that provider,
  else by the row of the plain model name; see "The plugin says who
  served an answer" in Decision 6.

The earlier question (how official plugins reach users before they have
their own repositories) was settled on 2026-11-20: all five are embedded,
and authors and the owner use **Add plugin from folder…**. It was settled
again on 2026-11-21 (issue 28): OTTY embeds none, and users install them
from `assets/plugins/<id>/` with **Add plugin from folder…**.
