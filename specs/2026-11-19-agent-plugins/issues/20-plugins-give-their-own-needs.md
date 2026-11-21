# 20 A plugin gives its own needs; OTTY keeps only the needs about plugins

Status: done
Blocked by: 19, and the "one estimate mark" change

## Why

Owner, 2026-11-21: "плагин должен сам предоставлять свой список действий
иначе можно написать плагин а пользовательские действия будут
интерпретированы через otty причем не всегда так как это нужно плагину".

Today OTTY writes every need of "Needs from you" itself. A part returns
only an error kind (`key-refused`) or a status (`not-logged-in`), and
OTTY guesses the title, the text and the button. Some of those guesses
are knowledge of particular agents and providers written in general
words: "the login expired, open the agent", "log in with
`login-command`", and above all the admin key, which is a notion of
Anthropic and OpenAI that the contract carries as `login-kind::admin-key`
and `read-bill(…, admin-key, …)`. A plugin whose agent wants something
else from the user cannot say so.

No compatibility with earlier versions: neither Agents mode nor the
plugin contracts were released, so the `0.1.0` contracts change in place.

## Who owns which need

OTTY keeps the needs that are about plugins themselves or about OTTY's
own permissions; a plugin can not tell them about itself:

- a plugin update waits for the user's OK (Review);
- a part of a plugin is off (Turn on);
- a key was found for a host no plugin serves (Add plugin);
- a plugin failed, ran out of time or was paused (Copy report);
- the format of an agent's files changed for the plugin (Copy report,
  Open plugin repository): the fix is a new plugin release, so the
  action is about the plugin, not about the agent;
- allow reading the system keyring.

Everything the user must do for an agent or a provider comes from the
part: not logged in, the login expired, a key is missing or refused, a
key of another kind is needed to read the bill, and anything a future
plugin needs.

## What to build

1. Contract `otty:host/types@0.1.0`: a `need` record and a `need-action`
   variant.
   - `need`: `id` (stable within the part, so the same need is not
     shown twice and does not flicker between reads), `title`, `text`,
     `action`.
   - `need-action`, the short list of what OTTY can do on the button:
     - `run(command)`: run a command in a terminal tab, with the same
       rules as `launch` and `login-command` today (for example
       `codex login`, or the agent itself so it renews its login);
     - `ask-key(key-ask)`: show a field for a key. `key-ask` has a
       `slot` (the part's own name for the key, such as `admin`), a
       label, a hint, an optional prefix the key starts with (such as
       `sk-ant-admin`) and an optional page where to get it. OTTY keeps
       the given key as it keeps given keys today and never shows it to
       the plugin, only a handle;
     - `open-page(url)`: `https` only;
     - `copy-report(text)`.
   - Texts come in the user's language: the host tells the part the
     UI language (`env`), and the part answers in it or in English.
     The contract carries no translation tables.
2. Where a part gives needs:
   - `read-error` gets `needs-user(need)`: the part can not read until
     the user does this. `key-refused` goes; a part that wants to say
     "the key was refused" says it as a need with its own button.
   - `profile-status::not-logged-in` carries a need. `login-command`
     goes: the need's `run` replaces it.
   - Provider part: `read-bill` and `numbers` get the keys the user gave
     for the account, each with its `slot`, in place of the `admin-key`
     argument. A part that needs a key it does not have returns
     `needs-user` with `ask-key`. `login-kind::admin-key` goes.
3. OTTY: `NeedAction::{LogIn, AddAdminKey, OpenAgent}`, the login
   expired, not logged in and admin key needs and every admin key word
   leave the core (types, model, view, i18n in all locales, the profile
   dialog's admin key field, `agents.json`). OTTY shows a part's need
   as it is, with a small label naming the plugin ("OpenAI plugin"), so
   the user sees who asks; OTTY's own needs carry no plugin label. The
   button runs the action; `ask-key` opens a key field named by the
   part's label, with its hint and its page.
4. SDK and plugins: SDK types and helpers for `Need` and the language;
   the test host; the template (one example need). Official plugins:
   - OpenAI: Codex not logged in → `run(codex login)`; Codex token
     expired → `run(codex)` so it renews; bill → `ask-key` slot `admin`,
     prefix `sk-admin-`, its page.
   - Anthropic: the same for Claude Code and the Anthropic admin key.
   - OpenCode, DeepSeek, OpenRouter: their logins and keys as they need.
   - Texts in all ten OTTY languages for the official plugins (copy the
     translations OTTY has now). Rebuild the embedded copies with
     `plugins/embed.sh`.
5. The language receiver (owner, 2026-11-21): OTTY passes the
   language of its interface; a plugin may have translations or not.
   - `env.language` gives the language OTTY shows now at every call,
     not the one of the load.
   - An optional contract of its own, `otty:language@0.1.0`, at the
     root of the SDK and not under agent or provider: one function,
     `language-changed(tag) -> bool`, true when the plugin's texts
     depend on the language. The SDK trait is `Language`, exported with
     `export!(T with agent, provider, language)`; the manifest names it
     as `[api] language = "0.1"`.
   - When the user changes the language, OTTY calls it on every plugin
     that exports it. When one answers true, OTTY reads Agents mode
     again at once, so its needs show in the new language without a
     restart. A plugin that does not export it is not called.
   - The template and the five official plugins export it.
6. A refused login goes to the agent part that holds it (owner,
   2026-11-21). A provider part does not know which agent keeps a
   subscription login, so it does not write that need.
   - `read-error::login-refused`: the provider refused the login or it
     expired. `needs-user` stays for keys the provider part itself asks
     for, such as an admin key or a DeepSeek key.
   - The agent contract gets `login-refused(profile, login) ->
     option<need>`: what the user must do for that login, such as
     `run(codex)` in the profile's folder or `run(opencode auth login)`.
   - OTTY asks the agent part of the profile whose login was read and
     shows its need with that plugin's label. Needs are the same when
     they have the same part, profile and id, so a profile shows one.
   - When that part says nothing, failed or ran out of time, OTTY shows
     its own need with no plugin label and no button: the provider
     refused the login of the profile.
   - The provider parts of OpenAI and Anthropic no longer give a
     `login-expired` need; the agent parts keep theirs.
7. Docs: "Needs from you" in `Overview.mdx` (who asks: OTTY or a
   plugin), `Write a Plugin.mdx` (how to give a need, the actions, the
   language), the official plugin pages, the SDK and template READMEs,
   `spec.md`.

## Done when

- Tests first: a part's need shows with its title, text, plugin label
  and button, and each `need-action` does what it says; a key given for
  an `ask-key` slot reaches the next `read-bill` as a handle with that
  slot; the same need from two reads shows once; OTTY's own needs (update,
  part off, add plugin, plugin failed, format changed, keyring) still show.
- Tests first: a language change reaches a plugin that exports the
  receiver and its needs are read again in the new language; a plugin
  without it is not called; `env.language` gives the new language
  between two calls.
- Tests first: a ChatGPT login kept only by OpenCode and refused by the
  `openai` provider part shows OpenCode's `run(opencode auth login)`
  need, not `codex`; a login kept by Codex shows one need, not two;
  with the agent part failing OTTY shows its own need.
- No form of `admin-key`, `AddAdminKey`, `key-refused` or
  `login-command` left outside `specs/` and git history.
- The checks of `AGENTS.md` pass.

## Comments
- 2026-11-21, stopped for the day (owner), work in progress on branch `wip/issue-20-plugin-needs` (one `--no-verify` wip commit on top of `agents` `889ea0fe`; not compiled or tested at the stop, the subagent was in the middle of an edit). Done: the contracts (`host.wit`: `need`, `need-action` with `run` / `ask-key(key-ask)` / `open-page` / `copy-report`, `read-error::needs-user`, `env.language`; `admin-key`, `key-refused` and `login-command` gone from the WIT), the SDK (including the test host), the manifest crate, the host and its rebuilt test plugins, most of `otty-agents`, most of the app (54 files under `crates/app/features`, i18n), and the template plugin with an example need (`plugins/template/src/needs.rs`). In progress: `crates/app/features/agents/io/src/bill.rs` and `account_numbers.rs` (given keys with their slot reaching `read-bill`). Not started: the needs of the five official plugins and their texts in ten languages, their tests, `plugins/embed.sh` for each, the docs and `spec.md`, the comments on design choices, and the full checks. Next: build the workspace, fix what the half-done edit broke, then go on from "In progress".
- 2026-11-21, step 3 (agent): the language receiver and refused logins.
  Choices made:
  - A refused login that its agent part says nothing about (the part
    gives no need, failed or ran out of time) is OTTY's own need: "The
    login was refused", naming the provider and the profile, with no
    button. No Run: OTTY does not know how that agent logs in again. No
    Copy report: the plugin did nothing wrong, the provider really
    refused the login, so a report to the plugin's author would mislead.
  - The agent function is `login-refused(profile, login) -> option<need>`:
    it also gets the login, so a part that keeps several (OpenCode) can
    tell which one; `none` is the SDK's default and leads to OTTY's own
    need.
  - OTTY asks the agent part of the profile whose login the provider read
    (the limits of that profile, the account's own numbers, the bill).
    Each profile that holds the login and was read gives its own need;
    two reads of the same profile give one, since needs are the same by
    part, profile and id.
  - When the language changes and at least one plugin answers true, OTTY
    reads Agents mode again as the Refresh button does, not only the
    parts of that plugin: the reads go by profile and account, not by
    plugin, and a change of language is rare. The receiver runs with no
    permissions: it may only answer.
- 2026-11-21, step 4 (agent): the docs are done. "Needs from you" in
  `Overview.mdx` says who asks (OTTY or a plugin, with its label), how a
  refused login goes to the agent that keeps it, and the language;
  `Write a Plugin.mdx` shows how to give a need, its four actions, given
  keys, `login_refused` and the language receiver; each official plugin
  page lists what it asks for; `spec.md` matches the code and records the
  owner's decision on prices and the open question on `MODEL_MAKERS`.
