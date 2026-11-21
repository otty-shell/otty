# 20 A plugin gives its own needs; OTTY keeps only the needs about plugins

Status: ready-for-agent
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
5. Docs: "Needs from you" in `Overview.mdx` (who asks: OTTY or a
   plugin), `Write a Plugin.mdx` (how to give a need, the actions, the
   language), the official plugin pages, the SDK and template READMEs,
   `spec.md`.

## Done when

- Tests first: a part's need shows with its title, text, plugin label
  and button, and each `need-action` does what it says; a key given for
  an `ask-key` slot reaches the next `read-bill` as a handle with that
  slot; the same need from two reads shows once; OTTY's own needs (update,
  part off, add plugin, plugin failed, format changed, keyring) still show.
- No `admin-key`, `AddAdminKey`, `key-refused` or `login-command`
  left outside `specs/` and git history.
- The checks of `AGENTS.md` pass.

## Comments
