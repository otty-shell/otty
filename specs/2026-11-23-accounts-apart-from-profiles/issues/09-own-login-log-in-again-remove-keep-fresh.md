# 09 OTTY's own login: Log in again, Remove, and kept fresh by the agent

Status: done
Blocked by: 08

## Why

"Before → after", Account page: the menu is Rename…, Hide, Open usage
page, and for OTTY's own login **Log in again** and **Remove**. Remove
deletes the hidden profile folder after a question, and the account
leaves only when no other place holds its login.

Decision 9: the login need of an account with no profile shows only on
the account, as "Log in again".

Decision 7: OTTY never refreshes a token itself, in any folder. In a
hidden profile the provider part runs the agent program there to read
numbers, and the agent renews its own tokens: `codex app-server` with
`CODEX_HOME=<folder>`, or the `claude` `/usage` probe with
`CLAUDE_CONFIG_DIR=<folder>`. Renewing a token by hand can rotate it
away from the agent and log it out.

Prototype (website repo, read only):
`/home/kemokempo/projects/main/ottyrun/website/src/lite/prototype-accounts-apart/`
(`AccountsApartPrototype.tsx`), page
`/lite/prototype-accounts-apart/?page=account&id=claude-own&added=anthropic`,
`&open=menu` for the menu, `&open=relogin` for Log in again.

## What to build

1. **The account menu** (`crates/app/features/agents/core/src/model/menus.rs`,
   `menu_actions.rs`): Rename…, Hide, Open usage page for every account,
   and for an account with an own login also Log in again and Remove
   (danger), as the prototype shows. The page header of such an account
   also has a small Log in again button and the mark "OTTY's own login".
2. **Log in again** runs the part's `log-in` command (issue 08) in the
   same folder in a watched terminal tab. When the tab closes OTTY reads
   the accounts and their numbers again.
3. **"Log in again" need.** When the provider refuses an own login, the
   provider part gives a need on the account titled "Log in again", with
   the `log-in` command for that folder. It shows only on the account
   and in All accounts → Needs you (issue 07), and the fix of item 2
   makes it go.
4. **Remove** asks "Remove this account?" with a line that OTTY deletes
   its private folder `<folder>` and that the user's agents and their
   own logins stay as they are. Remove deletes only the own login's
   folder and its `own_logins` entry, never a profile folder. When the
   plugin still finds the account in a profile folder, the account stays
   with Source "Found in <folder>" only. Else it leaves the list, and
   its name and Hide entry in `agents.json` stay, as for a removed
   plugin.
5. **Kept fresh by the agent.** In an own folder the provider parts read
   numbers by running the agent there, so the agent renews its own
   tokens:
   - OpenAI: `codex app-server` with `CODEX_HOME=<folder>`
     (`plugins/openai/src/app_server.rs`), first for an own folder. Its
     session files are not read there.
   - Anthropic: the `/usage` probe with `CLAUDE_CONFIG_DIR=<folder>`
     (`plugins/anthropic/src/usage_probe.rs`), when the token in the
     folder is past its `expiresAt` or the server refused it. The probe's
     session files stay cleaned up as today.
   - The parts' manifests allow these runs only with the folder variable
     pointing at the call's folders (issue 04). Bump each changed
     plugin's patch version and rebuild with `plugins/build.sh`.
   - OTTY reads own logins on the same schedule as other accounts while
     it runs. The spec leaves open how often to read when Agents mode is
     not opened for weeks ("How often OTTY runs the agent in a hidden
     profile"): ask the owner before adding a background read.
6. Words in all ten locales. The public docs are issue 10.

## Done when

- Tests first: the menu of an own login has Log in again and Remove, the
  menu of a found account has neither, Log in again opens a watched tab
  with the command and the folder, a refused own login shows "Log in
  again" on the account only, Remove asks first, deletes the own folder
  and entry and nothing else, an account also found in `~/.codex` stays
  with "Found in ~/.codex". (Plugins): a numbers read in an own folder
  runs the agent there with the folder variable, and never refreshes a
  token itself.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-23, owner answer: OTTY reads its own logins on the usual
  schedule while it runs, and runs nothing in the background when it is
  closed.

- 2026-11-23, done. An account with OTTY's own login can log in again
  and be removed, and the provider parts keep the own login fresh by
  running the agent in its folder.

  Screen: the account menu is Rename…, Log in again (own login only),
  Open usage page, a line, Hide or Show again, and Remove (own login
  only, danger), as the prototype orders it. The page header of such an
  account has the mark "OTTY's own login" after the name and a small
  Log in again button before Refresh (`overview::header_with`), and its
  row in the left panel the tag "OTTY" (the kit `TreeRow` got `tag`).
  Whether an account has an own login is `own_login_actions::has_own_login`:
  an `own_logins` entry of `agents.json` made by the part that found the
  account and named in its `found-in`. Log in again (menu or header,
  `AccountAction::LogInAgain`, `AgentsIntent::LogInAgain`) asks the part
  for its `log-in` command of that folder (`PrepareLogInAgain`, io
  `log_in_again`, answered with `LogInAgainPrepared`) and runs it in a
  watched tab titled "Log in with <agent>" (`OpenFixTab`), so its close
  reads the accounts and their numbers again. Remove asks "Remove this
  account?" with "OTTY deletes its private folder <folder>. Your agents
  and their own logins stay as they are." (the remove dialog of profiles,
  now for both). Confirm drops the own login entries of the account from
  `agents.json` and removes their folders (`RemoveOwnLoginFolder`), and
  nothing else: the account's name and Hide stay. The save reads again,
  so an account still found in `~/.codex` stays with Source "Found in
  ~/.codex" only, else it leaves the list. Four new texts in all ten
  locales (`agents_log_in_again`, `agents_remove_account_title`,
  `tpl_agents_remove_account_text`, `plugins_place_may_change`); the mark
  reuses "OTTY's own login". The "Log in again" need of a part shows on
  the account and in All accounts → Needs you, never on a profile, as
  any need of an account did.

  Manifest and host: a `[[agents.provider.logins]]` entry may declare
  `write` globs inside the folder (`LoginRule::write`); in a call with
  login folders the part may write and delete them in each folder, an own
  login folder too, never a key file and never outside the call.
  Plugins mode shows them in the group of those profiles noted "may write
  and delete" (`PermissionLine::WritesInProfiles`).

  Plugins: `openai` 0.2.11 -> 0.2.12: the numbers of an account with an
  own login are read first with `codex app-server` started there with
  `CODEX_HOME` set to it, even past the token's time, so Codex renews its
  own login; the session files of an own folder are never read; an
  expired token there is no need; a token the server refused there needs
  "Log in again" (`codex login` in that folder). `anthropic` 0.2.11 ->
  0.2.12: the own login is read first; a token past its `expiresAt` is
  not sent and the `/usage` probe runs Claude Code there with
  `CLAUDE_CONFIG_DIR`, as after a refused token; a need there, or a login
  that can not be renewed, is "Log in again" (`claude auth login
  --claudeai` in that folder). The probe's plain `write` permission of
  `~/.claude*` and `CLAUDE_CONFIG_DIR` became `write` of its `logins`
  entry, so it cleans up its session files inside the folders of the call
  only, an own login folder too. Neither part renews a token itself. Both
  rebuilt into `assets/plugins/`; the template and the host test plugins
  did not change.

  Schedule: an own login is read with the other accounts, as the owner
  answered; nothing new runs in the background, and with Agents mode away
  the parts start no program, as before.

  Tests first, each seen failing: manifest (`write` inside login folders
  only, refused outside the folder, the plain words), Plugins mode (the
  place and its note), the kit tag of a tree row, OpenAI (own login read
  by `codex app-server` with `CODEX_HOME`, nothing sent, no need for an
  expired own token, no session files read there, a refused own login
  needs "Log in again", the need in ten languages), Anthropic (own login
  first, past its time nothing sent, the probe in the own folder and its
  session file removed there, a refused own login and a lapsed one need
  "Log in again", the need in ten languages), the core (menu of an own
  login has Log in again and Remove, a found account has neither, the
  header mark and button, the row tag, Log in again asks for the command
  and opens a watched tab that reads again on close, a part with no
  command is told, Remove asks first, Cancel keeps all, Confirm removes
  only the own folder and entry, an account also in `~/.codex` stays
  with "Found in ~/.codex"). These passed at once, since the code was
  there: the "Log in again" need on the account only, and the own login
  read on the usual schedule. The app dispatch test of `PrepareLogInAgain`
  was written with the wiring.

  Left: the public docs are issue 10. The macOS Keychain question stays
  open.
