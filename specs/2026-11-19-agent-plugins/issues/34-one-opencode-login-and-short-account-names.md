# 34 OpenCode gives the login it uses; the left panel names an account by its login

Status: done
Blocked by: 33

## Why

Owner, 2026-11-22: two accounts "OpenAI · login" come from one OpenCode
folder. It is not clear why they are named so, nor why there are two.

What the owner's folder holds (read-only check, no key values printed):
the `credential` table of `~/.local/share/opencode/opencode.db` has two
rows with `integration_id = openai`, both ChatGPT logins by
`chatgpt-token-sharing`: label `OpenAI`, `active = 0`, and label
`OpenAI 2`, `active = 1`, made 38 s apart. The owner logged in twice; OpenCode
kept both and uses the active one. The access tokens of both have the same
`sub` claim (the same person), but their account and email sit in
`encrypted_auth_metadata`, so the plugin can not tell them apart and gives
two logins. With no email and no plan, OTTY names each by the fallback
word "login".

The owner also wants the left panel to show an account by its logo and
login only: the provider name ("Anthropic", "OpenAI", …) takes room there,
the logo already says it, and the account's page names it.

## What to build

1. **OpenCode gives the login it uses.** When the `credential` table has
   an `active` column, the plugin reads only the active row of each
   `integration_id` (OpenCode's own choice of login); with no `active`
   column, or no active row for a provider, it reads all rows of it, as
   today. `auth.json` is read as today.
2. **One person is one login.** Two ChatGPT logins whose access tokens
   have the same `sub` claim are the same login, also when they name no
   account and no email (the rule of "a login found in both places is
   given once" in `plugins/opencode/src/logins.rs`). `sub` is only
   compared inside the plugin; it is not shown and not added to the WIT
   contract.
3. Bump OpenCode to 0.1.4, rebuild with `plugins/build.sh` into
   `assets/plugins/opencode`; its plugin page and README follow.
4. **"unknown".** An account whose login names no email, no key and no
   plan is named by the word "unknown" (all ten locales), not "login":
   "OpenAI · unknown". Remove `AgentsAccountLogin` if nothing else uses it.
5. **The left panel names an account by its login.** In the left panel of
   Agents mode (`model/left_panel.rs`) an account's row is its logo and
   its login only, such as "you@example.com", "key ••••a1f3" or
   "unknown", with the plugin still following when two plugins read it
   ("you@example.com (Example Fork)"). A name the user gave the account is
   shown as given. Every other place (account page, needs, session
   account, found report, usage groups, rename dialog default) keeps the
   full name with the provider; the account's page shows its provider.
   Docs that show left panel rows follow.

## Done when

- Tests first: two credential rows of one provider, one active → one
  login, the active one; no `active` column → all rows; two logins with
  one `sub` and no account or email → one login; a login with nothing
  known is "Provider · unknown"; the left panel row is the login without
  the provider, a user's name as given, a shared account with its plugin.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-22, issue 34, done. OpenCode 0.1.4: `Database::credentials`
  (`plugins/opencode/src/database.rs`) checks the `credential` table for
  an `active` column with `pragma_table_info`; when it is there, a
  provider with an active row gives only that row, any other provider
  all of its rows; with no such column every row is read, as before. In
  `logins.rs` the access token's `sub` claim is kept beside each login
  inside the plugin (`ReadLogin`), never in the WIT contract: two ChatGPT
  logins are the same when their account ids match, else their emails,
  else, when neither pair is there to compare, their `sub`. The plugin
  was rebuilt into `assets/plugins/opencode`; its page and README
  follow. In the app, `AgentsAccountLogin` became `AgentsAccountUnknown`
  ("unknown" in all ten locales), and `short_account_name`
  (`model/account_name.rs`) names a left panel row by its login, a
  user's name as given, with the plugin after it when two plugins read
  it. Every other place keeps the full name; the account page already
  shows the provider in its info. The Overview and Plugins pages say how
  the left panel names an account. Tests: active row only / no `active`
  column (database), the active login of two / one person by `sub`
  (logins), "OpenAI · unknown" (account name), login only, user's name
  and shared plugin (left panel); the tests of other left panel rows
  were changed to the short names.
