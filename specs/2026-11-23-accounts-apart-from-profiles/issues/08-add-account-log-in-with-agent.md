# 08 Add account: "Log in with <agent>" makes OTTY's own login

Status: done
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

- 2026-11-23, owner answer: the Claude login is `claude auth login
  --claudeai`.

- 2026-11-23, done. Add account → "Log in with <agent>" makes OTTY's own
  login, in a folder of OTTY's own, and the account found there shows
  with Source "OTTY's own login".

  Contract (changed in place, Decision 12): `login-folder` has a second
  case, `own-login(tuple<string, string>)` (own login id, folder); `own`
  is a WIT keyword, so the case is `own-login` (`LoginFolder::OwnLogin`).
  `log-in(provider, folder) -> option<command>` gives the agent's login
  command pointed at the folder: an option, so a part with no `log_in`
  (DeepSeek, OpenRouter) needs no code, the SDK default is none. The SDK
  trait has `Provider::log_in`; `TestHost::as_provider_in` takes own
  login folders. In the host, `ProviderCalls::log_in(folder)` is a quick
  call with no folder permissions, and the folders of a call map an own
  login folder to `otty_plugin_manifest::LoginFolder::own(folder)`.

  Manifest: a `[[agents.provider.logins]]` entry may say `log_in = true`;
  two such entries in one part are refused. `LoginRule::offers_log_in`,
  `ProviderPart::log_in`. Inside an own login folder the part reads what
  the entry that offers `log_in` declares, and its `folder_env` may point
  there; a part with no such entry reads nothing there.

  `otty-agents`: `OwnLogin` (id, plugin, provider part, agent part) with
  `folder_in(data_dir)` = `<data dir>/accounts/<id>`, `FoundIn::OwnLogin
  { id, folder }` (never a profile folder: `is_found_in` is false for it,
  `is_found_in_own_login(id)`), `LogInWith` on the registry entry of a
  part that offers it (agent id and name), `log_in_command` and
  `own_login_accounts` (the part that made the login, asked about its
  folder alone), `Unavailable::NoLogIn`. `find_accounts` takes the own
  logins and the data dir; `PluginProvider` passes each part its own
  logins next to its profile folders, so a part with only an own login is
  asked too. `agents.json` has `own_logins` (`id`, `plugin`, `provider`,
  `agent`, no folder); an entry whose id is no plain name (letters,
  digits, `-`, `_`), or that names no part, is not read, so a hand-edited
  id can never point the folder elsewhere. Removing a plugin keeps the
  entries, as for accounts.

  Screen: the `+` of the Accounts section (on once `agents.json` was read)
  opens Add account: the intro of the prototype, a Provider select of
  every provider part that is on with its logo, name and "<Plugin>
  plugin" (the kit `NoteSelectOption` got `with_icon`), then How: "Log in
  with <agent>" with the line that names the part's command once the part
  gave it (program name and arguments, no variable), and "Paste a key"
  drawn off with the kit tooltip "Coming later" on hover (a row of the
  dialog, not a kit option, so it is always visible). A provider with no
  agent login says it and Continue is off. With no provider part on, the
  dialog says so and offers Manage plugins… (Plugins mode). Picking a
  provider that offers it asks the part at once for the command of a new
  own login with a fresh id (`PrepareOwnLogin`, the id is 16 hex digits
  from the std random hasher seed and the time, no new dependency);
  nothing is made on disk until Continue. Continue makes the empty folder
  (`MakeOwnLoginFolder`, a folder already there is refused and the error
  stays in the dialog), then saves the own login in `agents.json` and
  opens the login tab (`OpenLoginTab`, titled "Log in with <agent>", in
  the home folder), which the feature watches (`LoginTabOpened`). When it
  closes, the part is asked about that folder alone (`CheckOwnLogin`): an
  account there reads the profiles and accounts again; none removes the
  entry and the folder (`RemoveOwnLoginFolder`, only that folder) and says
  "The login did not finish"; a part that could not be asked keeps both
  and says why. The data dir of `app.rs` is now kept in `App` and given to
  the reads. Fifteen texts in all ten locales.

  Plugins: `openai` 0.2.10 -> 0.2.11 (`log_in = true`, `codex login` with
  `CODEX_HOME`, an own folder read as a Codex folder, so one ChatGPT
  account in `~/.codex` and in an own login is one account with both
  places), `anthropic` 0.2.10 -> 0.2.11 (`log_in = true`, `claude auth
  login --claudeai` with `CLAUDE_CONFIG_DIR`, the own folder's
  `.claude.json` is inside it), template `example` 0.1.5 -> 0.1.6
  (`example-agent login` with `EXAMPLE_AGENT_HOME`, README says how
  `log_in` works), `deepseek` and `openrouter` 0.1.4 -> 0.1.5 (rebuilt
  for the new export, no `log_in`). All five rebuilt into
  `assets/plugins/`; `template.wasm` and the test plugins of the host were
  rebuilt (`programs` matches the new case).

  Tests first, each seen failing: manifest (an entry offers `log_in`, two
  are refused, an own folder reads as the offering entry), SDK test host
  (own folder), host over the template (account found in an own folder,
  the login command; the old `template.wasm` failed every provider test
  until it was rebuilt), template, OpenAI and Anthropic (command for a
  folder, account in an own folder), `otty-agents` (folder, saved own
  logins and refused ids, own folders passed to the part, the login
  command, the accounts of the folder alone), the core dialog, model and
  view (providers with logo and plugin, Log in with only for a part that
  says so, Paste a key off with "Coming later", Plugins mode with no
  part, Continue makes the folder then saves and opens a watched tab, a
  closed tab checks the folder, found is read again, none removes folder
  and entry and says "The login did not finish"), the io reads (the own
  login's account found, its folder never a profile, folder made, made
  only once, removed alone, checked with the real Anthropic plugin) and
  the app (the login tab is watched until it closes). The kit test of
  the option logo was written with `with_icon` and only seen passing.

  Left for issue 09: Log in again and Remove in the account menu and the
  page header mark, the "Log in again" need of a refused own login, and
  keeping the own login fresh by running the agent in its folder (the
  Anthropic probe's write permissions cover only `~/.claude*` and
  `CLAUDE_CONFIG_DIR`, not the own folder under the data dir). The
  macOS Keychain question stays open: both parts offer "Log in with" on
  every system for now. The public docs are issue 10.
