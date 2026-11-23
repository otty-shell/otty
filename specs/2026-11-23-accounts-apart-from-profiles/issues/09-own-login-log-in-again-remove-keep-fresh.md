# 09 OTTY's own login: Log in again, Remove, and kept fresh by the agent

Status: ready
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
