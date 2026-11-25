# 08 Menus, rename and hide

Status: ready-for-agent
Blocked by: 05, 06

## Why

Spec Decision 7: the user's own names and what they do not want to see.

## What to build

1. "⋯" on account and profile pages and a right click on their rows (left
   panel and the overview tables) open the same menu at the pointer;
   Escape, a click away or a pick closes it; the selection does not
   change.
2. Account: Rename…, Open usage page (the provider's page in the browser),
   Hide / Show again. Profile: Rename…, Open folder (in the Files panel),
   Hide / Show again. Overview "⋯": Show hidden … (N) / Hide hidden ….
3. Hidden items are not read and leave lists, chips and needs.
4. Names and hidden items are saved in `agents.json`.

## Done when

- Tests first: the menu entries per kind; hide stops reads; names and
  hidden items saved and read back; an `agents.json` without them still
  reads.
- The checks of `AGENTS.md` pass.

## Files

Feature core `menus.rs`, reducer, state; `crates/agents/src/saved_profiles.rs`; io save/load; i18n.

## Owner checks by hand

Rename both ChatGPT accounts; hide one; Show hidden; restart OTTY and see
the names kept.

## Comments
- 2026-11-19, coordinator: moved here from issues 03/04 — the "⋯" of All accounts also has "Show Found by OTTY" (opens the Found by OTTY window again); both overview pages get their "⋯" here (Show hidden accounts/profiles).
- 2026-11-19, issue 05: the account page has Refresh but no "⋯" yet; add it here with the account menu.

Built (2026-11-19):

- `otty-agents`: `agents.json` keeps the user's choices per account in
  `accounts` (provider and `account_id`, `organization_id` or `email`,
  with `name`, `hidden` and, for an organization, `admin_key`; one entry
  per account, an empty one is not written) and `hidden_profiles` (agent
  and config folder, so a profile OTTY found and a renamed profile stay
  hidden). A file without them reads as before; an old `hidden` flag in a
  profile entry is still ignored. `Account::can_be_saved` (not a key) and
  `Account::usage_page` (claude.ai or the Anthropic console, ChatGPT Codex
  usage or the OpenAI platform, DeepSeek, OpenRouter, Moonshot, Kilo).
- Menus at the pointer (`menus.rs`, `menu_actions.rs`, `model/menus.rs`,
  `view/menus.rs`): "⋯" of the account page and a right click on an
  account row (left panel, All accounts) open Rename…, Open usage page,
  Admin key… (an organization whose bill OTTY reads; it opens the profile
  dialog with the admin key field, as the need does), Hide / Show again.
  The profile menu (page "⋯", right click in the left panel and in All
  profiles) has Rename…, Open folder, Edit, Log in, Hide / Show again,
  Remove. The "⋯" of All accounts: Show hidden accounts (N) / Hide hidden
  accounts, Show Found by OTTY (closing it then writes nothing); of All
  profiles: Show hidden profiles (N) / Hide hidden profiles. A right click
  does not change what is shown.
- Rename dialog (`rename_dialog.rs`, its model and view; overlay
  `AgentsRenameDialog` in the binary, Esc cancels): an account's name is
  the user's own, empty or its own name gives back "Provider · login"; a
  profile's name is checked like the profile dialog's and saved as an edit
  of the profile, and its page stays open under the new name. The name
  shows everywhere an account is named (lists, pages, chips, needs, usage
  By account, session Account line).
- Hidden (`hiding.rs`, `profiles.rs`): the change shows at once and is
  saved. A hidden profile is not checked or read at all (io keeps it apart
  in `LoadedProfiles::hidden_profiles`), its sessions leave the projects;
  no limits or bill are asked for a hidden account. Both leave the lists,
  chips and needs; with Show hidden they are listed with "hidden" (a
  neutral badge in the tables) and their page says "Hidden: OTTY does not
  read …" with only what needs no read. Hiding the open page goes to All
  accounts / All profiles.
- Open folder: Terminal mode, the Files panel shown at the profile folder
  (binary `ShowInFiles`); Open usage page: io `open_page` (`xdg-open` /
  `open`).
- i18n: 19 new keys in the 10 languages.

Decided differently:

- An account known only by its key (fingerprint, FNV-1a) is never saved:
  its Rename… and Hide are off in its menu with the line "A key can not be
  renamed or hidden yet: OTTY keeps nothing taken from a key." Waiting for
  the owner's choice of a cryptographic hash.
- The profile menu keeps Edit, Log in and Remove next to the new items: Edit
  is still the only way to move a profile's folder, Remove the only way to
  drop an added profile.
- Admin key… reuses the profile dialog's admin key field (as the need Add
  admin key does) instead of a dialog of its own; issue 09 can give it one.
- Open usage page addresses are best guesses of each provider's page; they
  live in one place, `Account::usage_page`.
