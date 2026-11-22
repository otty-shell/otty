# 29 A provider reads its accounts every way it can; duplicates are fine

Status: in progress
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
