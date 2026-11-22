# 29 A provider reads its accounts every way it can; duplicates are fine

Status: done
Blocked by: 28

## Why

Owner, 2026-11-21/22, after the "host error: not found" failure of the
Anthropic part on `~/.claude-semrush` (its `.credentials.json` is `{}`,
Claude Code still works; issue 13). The model the owner wants:

- The profile is the main thing. The agent part of a plugin lists the
  accounts of a profile (whose they are, where their key sits, what else
  the files tell). OpenCode's plugin does only that.
- For each account OTTY asks the provider parts that serve it, and a
  provider part gets the numbers (limits, money, credits, bill) any way it
  can: from the service with the key, or by asking the agent itself, such
  as `claude /usage` in the profile the account was found in, or the
  limits Codex writes in its files. When no way works it answers
  "nothing", not an error.
- No fight over names. If two plugins serve the same service, both read
  it and both accounts are shown, each with its plugin's label; the user
  hides the one they do not want. An account found in two profiles stays
  one account when the provider and the identity match (as today); when
  they do not, it is two accounts, and that is fine.

## What to build

1. **The provider reads every way.** The agent part's `read-limits`
   goes away, and so does OTTY's "provider first, then the agent's
   limits" merge. A provider part is told where each account was found:
   the agent part (plugin id and part id) and the profile (its config
   folder and what `launch`/hidden terminals need to run the agent in it),
   for every profile the account was found in. The provider world gets
   the host operations this needs (files, programs, hidden terminals),
   under the provider part's own declared permissions.
   - Anthropic: the `/usage` probe moves from the `claude-code` part to
     the `anthropic` part: token there → the server; else, found in a
     Claude Code profile → `/usage` in that profile; else nothing.
   - OpenAI: the limits Codex writes in its files move to the `openai`
     part the same way.
   - The permissions move with the code in the manifests; the docs and
     the permission words follow.
2. **"Nothing" is not an error.** A key that is not at its place is a
   normal case a part can see and test: the SDK's `TestHost` can make a
   handle to a key that is not there (the real host reads keys lazily,
   the test host today refuses such a handle, so plugin tests can not
   express the Semrush case). The Anthropic and OpenAI parts answer
   "nothing" for it, and use what else they have (a key the user gave
   under `admin` still reads the bill).
3. **Revert `93d8e65e`** ("a provider part gets no handle to a login key
   that is not there"): OTTY no longer skips a provider part on its own;
   the part decides.
4. **Every provider part that serves an account reads it.** No "which
   plugin reads X" choice at install or later: the conflict choice of the
   Plugins page and `choices` in `plugins.json` go. Each read result is
   its own account row, labelled with its plugin. Joining by identity
   inside one provider id stays.
5. **An account no part reads is shown**, with one line saying no
   installed plugin reads it (its sessions and their estimated cost are
   still counted).
6. **Hidden accounts are left out of every total** (money, spend,
   bills). Check every total and pin it with tests.
7. Spec (Decisions on parts, limits, conflicts), public docs
   (`docs/public/Agents/*.mdx`, Write a Plugin) and plugin READMEs follow.
   Rebuild the changed plugins with `plugins/build.sh`.

No backward compatibility: Agents mode was never released.

## Done when

- Tests first: Semrush case (no token, found in a Claude Code profile →
  `/usage` limits, no failure); no token and no profile to probe →
  nothing, no failure; admin key given → bill read without the token;
  two provider parts for one service → two accounts; an unread account
  is shown; hidden accounts out of totals.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-22, done:

1. The agent part's `read-limits` is gone, and so are OTTY's
   `read_profile_limits`, `LimitSource` and the "provider first, then the
   agent's limits" merge. `login-facts` gets `found-in: list<found-profile>`:
   for every shown profile the account was found in, the plugin and the
   agent part that found it, the folder, and `launch` (the command the
   agent part gives for a new session there). `numbers` and `read-bill`
   take `found` too, `account-numbers` gets `recorded-at-ms`, and the
   provider world imports `files`, `programs` and `terminal`. A provider
   part may declare `read`, `write`, `env`, `run` and `terminal` (not
   `sqlite`, `keys`, `keyring`). The pace of a slow reading (5 minutes)
   moved to `numbers`, kept per part and found folders. The Anthropic
   part reads token → server; no token, or a failing server → `/usage`
   in the first Claude Code profile of `found-in`; else nothing. The
   OpenAI part reads token → `chatgpt.com`; no token or failing server →
   `codex app-server`, then the session files, in the first Codex
   profile; else nothing. Permissions moved in both manifests; the Codex
   and Claude Code parts no longer run anything but `codex --version`.
2. `TestHost::key_at(place)` makes a handle read at each use, so a
   `{}` credentials file is a handle whose use is `not-found`. Every
   official provider part (and the template) answers nothing for it; an
   admin key given under `admin` still reads the bill.
3. 93d8e65e is reverted (its own commit).
4. The conflict choice, `Conflict`, `Served`, `Winner`, `Choice` and
   `choices` in `plugins.json` are gone; every part installs on. The
   registry keeps every provider part (one agent part per agent id, the
   first). `merge_accounts` gives one copy of an account per plugin that
   reads it (`Account::read_by`); added accounts only to parts that offer
   Add account. Saved names, hidden and given keys of a found account
   are kept with `"plugin"` in `agents.json`. While two plugins read a
   provider, the name of each account says its plugin.
5. An account no plugin reads keeps its row and the need "No provider
   plugin", whose line now says no installed plugin reads it (all ten
   languages).
6. Hidden accounts: no read (also when every subscription copy of a
   profile is hidden), not in the totals by account (their tokens go to
   a shown copy of the same login, else to no account); a profile shows
   the limits and the bill of its first shown copy.
7. Spec (Decisions 4, 5, WIT sketch, install flow, UI table), public
   docs (Plugins, Write a Plugin, Overview, What OTTY Reads and Sends,
   Anthropic, OpenAI), plugin, template and SDK READMEs follow. All six
   plugins are rebuilt into `assets/plugins/`, the host fixtures with
   `scripts/build-test-plugins.sh`. `url` was updated to 2.5.9 for
   RUSTSEC-2026-0592, which `cargo deny` reported.

Left open for the owner:

- The probe asks only the first Claude Code (or Codex) profile of
  `found-in`; it does not try the next one when that fails.
- The OpenAI part also falls back on `codex app-server` and the session
  files when `chatgpt.com` fails with a token there, as the old order
  did; the issue named only "no token".
- A probe that can not start while Agents mode is away is a failure of
  the read (as before), shown greyed until the read on screen.
- Two agent parts with one agent id: first installed wins, no choice;
  the user turns the other off. Hiding one copy of an added account
  hides every copy (one entry).
- A host two plugins serve under different provider ids becomes the id
  of the first; the second does not read it.
