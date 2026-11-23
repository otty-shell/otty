# 08 Add account: "Log in with <agent>" makes OTTY's own login

Status: ready
Blocked by: 03, 05, 07

## Why

Decision 6: a user who wants to see a Claude balance should not need a
Claude Code profile they work in. Add account offers, for each provider
part that can, **Log in with <agent>**. OTTY makes an empty folder of
its own (a hidden profile), runs the agent's official login command
there in a terminal tab, then asks the provider part for accounts in
that folder. OTTY never handles a password or an OAuth flow itself.
Decision 5: the hidden profile is the provider part's second place to
find accounts. The hidden profile never shows as a profile, and its
projects and sessions are never read or shown. "Paste a key" is not
offered yet (Decision 10, Later).

The prototype in OTTY Lite shows the dialog (website repo, read only):
`/home/kemokempo/projects/main/ottyrun/website/src/lite/prototype-accounts-apart/`
(`AddAccountDialog.tsx`), page `/lite/prototype-accounts-apart/?open=add`
(`&list=1` opens the provider list, `&provider=deepseek` shows a
provider with no agent login).

## What to build

1. **The contract** (`agents-provider/0.1.0/provider.wit`, Decision 12:
   in place):
   - `login-folder` gets the case `own(tuple<string, string>)` (own login
     id, folder).
   - `log-in(provider, folder) -> command`: the agent's official login
     command pointed at `folder`, such as `codex login` with
     `CODEX_HOME=<folder>`.
   - The SDK and `ProviderCalls` follow. The folders of an own login are
     login folders of a call as in issue 04.
2. **The manifest.** A `logins` entry of a provider part (issue 04) may
   say it offers "Log in with" (such as `log_in = true`). The label is
   "Log in with <agent part's name>". At most one entry of a part offers
   it.
3. **Own logins in OTTY** (`crates/agents/src/`):
   - `agents.json` gets `own_logins`: `id` (a new random id), `plugin`,
     `provider`, `agent`, as in "What OTTY stores". The folder is always
     `<data dir>/accounts/<id>/` and is not stored. The data dir is the
     one of `crates/app/otty/src/app.rs` `data_dir`, such as
     `~/.local/share/otty`, given to the agents io crate.
   - OTTY's word for these in code is "own login" (`OwnLogin`), since
     `agents.json` already has `hidden_profiles` for the profiles the
     user hid (`saved_profiles/hidden_profiles.rs`).
   - `PluginProvider` passes each part its own logins as `own` folders
     next to its profile folders. An own login's folder is never a
     profile: no session, project, token or version is read there.
   - Removing a plugin keeps its `own_logins` and their folders, so
     reinstalling it brings the account back.
4. **The dialog** (`crates/app/features/agents/core/src/`, new
   `add_account_dialog` files, the "+ Add account" entry of the left
   panel comes back):
   - "Pick the provider of the account. The list comes from your
     installed plugins." and a Provider select of every provider part
     that is on, each with its logo, its name and its plugin's name.
   - Under "How": **Log in with <agent>** when the part offers it, with
     the line "OTTY keeps this login in its own private folder and runs
     the official `<command>` there, your profiles stay as they are."
     **Paste a key** is shown disabled, with the tooltip "Coming later".
     A provider with no agent login says "<Provider> has no agent login,
     so it can be added only with a key, and keys come later." and
     Continue is disabled.
   - With no provider part on, the dialog says so and offers Plugins
     mode.
   - Continue makes the folder, adds the own login to `agents.json`, and
     runs the part's `log-in` command in a visible terminal tab that the
     feature watches (as the tabs of needs).
5. **After the tab closes**, OTTY asks the part for accounts in that
   folder. An account found there is shown, with Source "OTTY's own
   login" (and "Found in ~/.codex" too when the plugin finds the same
   account there, Decision 8). No account found removes the folder and
   the `own_logins` entry, and says "The login did not finish".
6. **The plugins.** Bump each changed plugin's patch version and rebuild
   with `plugins/build.sh`.
   - OpenAI: `log-in` gives `codex login` with `CODEX_HOME`, and finds
     accounts in `own` folders as in profile folders.
   - Anthropic: `log-in` gives Claude Code's login with
     `CLAUDE_CONFIG_DIR` (`claude auth login --claudeai`, as the
     prototype shows and CodexBar runs, unless the owner answers the
     open question "Claude login command" otherwise).
   - Template: the example provider offers it with the example agent.
   - DeepSeek, OpenRouter: no `log_in`.
7. Words in all ten locales. The public docs are issue 10.

## Done when

- Tests first: the dialog lists every provider part that is on with its
  logo and plugin, offers Log in with only for a part that says so,
  shows Paste a key disabled with "Coming later", and with no provider
  part offers Plugins mode. Continue makes `<data dir>/accounts/<id>/`,
  saves the own login and opens a watched tab with the part's command
  and the folder variable. After the tab closes, a found account is
  shown with Source "OTTY's own login", none found removes the folder
  and the entry and says "The login did not finish". An own login's
  folder is never listed as a profile and none of its sessions is read.
  (Plugins): OpenAI and Anthropic give their login command for a
  folder, and find an account in an `own` folder.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-23, written with the issues. Where the prototype and the spec
  differ, this issue follows the spec, ask the owner if unsure: the
  prototype runs the login inside the dialog with a log view and "OTTY
  notices the login by itself", the spec runs it in a terminal tab and
  asks for accounts when the tab closes (a tab also lets the user type
  a code the login may ask for). The prototype's folder is
  `~/.local/share/otty/logins/openai-1`, the spec's
  `<data dir>/accounts/<id>/`.
- Open question of the spec, "macOS Keychain": Claude Code on macOS
  keeps its login in the Keychain. Until checked on a Mac, ask the owner
  whether "Log in with Claude Code" is offered on macOS. OTTY must not
  check the agent itself (no agent-specific code), so the Anthropic
  plugin would decide.
