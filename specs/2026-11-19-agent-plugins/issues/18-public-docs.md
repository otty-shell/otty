# 18 Public docs: Agents mode and how to write a plugin

Status: ready-for-agent
Blocked by: 09, 17

## Why

Spec Decision 14: issue 15 of `2026-11-19-agents-accounts` moved here.
The docs must say exactly what OTTY and each official plugin read and
send, and how anyone can write a plugin.

## What to build

1. `docs/public/Agents/`: the Overview explains accounts and profiles,
   the needs, hiding, Add account, the keyring permission, "format
   changed" (now naming the plugin) and plugins (official, community,
   built in, local). Remove the budget. Everything of the old issue 15 and its
   comments still holds where the code still does it; Kilo, Kilo
   Gateway, Moonshot and Goose are gone and are not described.
2. One page per official plugin (rename the `Providers/` group to fit
   plugins): Anthropic, OpenAI, OpenCode, DeepSeek, OpenRouter. Each page
   lists its parts and, per part, every file read and written, every
   request and to whom, every program run, keyring entries; requests
   sent with logins found in another agent (a ChatGPT login found in
   OpenCode, read by the `openai` part). Generated or checked against
   each plugin's manifest permissions.
3. A Plugins page: opened from Settings; install by URL, plugins and
   their parts, what permissions mean and that they apply per part, the
   choice when another plugin already serves an id, turning parts on and
   off, automatic updates and when an update waits (the permission
   diff), "Needs a newer OTTY", the five official plugins built into
   OTTY, remove, what happens when a plugin or a
   part is missing (the spec table, in plain words).
4. "How to write a plugin": the template, the SDK traits, parts and the
   manifest (`[[agent]]`, `[[provider]]`, `[api]`, the permissions of
   each part), `export!` with the traits, the host operations and their
   permissions, the test host, depending on the SDK by git tag, trying a
   build with **Add plugin from folder…** and **Reload**, building
   for `wasm32-wasip2` with `opt-level = 3`, releasing with a semver tag,
   contract versions, the safety rules of official plugins as advice.
5. Check each claim against the code and the manifests.

## Done when

- Every page matches the code and the manifests; the website builds the
  docs (`make dev` in `../website`).
- No Rust change, so no cargo checks.

## Comments
- 2026-11-20, issue 10: tell plugin authors not to use `std::collections::HashMap`/`HashSet` (their random seed makes the `.wasm` import `wasi:random`, which OTTY refuses at install) — use `BTreeMap`/`BTreeSet`. Also add this warning to the SDK README and the template README.
