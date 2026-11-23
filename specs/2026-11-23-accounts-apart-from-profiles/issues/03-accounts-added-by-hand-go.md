# 03 Accounts added by hand, by the place of their key, go

Status: ready
Blocked by: 01

## Why

Decision 10: the old Add account ("a provider and the place of its key,
a file or an environment variable") is removed, with Edit key… and the
`ask-key` need with no slot that replaced a refused key of such an
account (issue 26 of spec `2026-11-19-agent-plugins`). An account known
only by where its key lies can not be told apart from a copy of itself.
"Paste a key" comes later. An `ask-key` need with a slot, such as the
admin key of an Anthropic or OpenAI bill, stays as it is.

Removing these first leaves issue 05 one kind of account to rewrite: the
accounts a provider part finds.

## What to build

1. **The dialog goes.** The Add account dialog that takes a provider and
   a key place goes: `add_account_dialog.rs`, the key place part of
   `key_place_field.rs` (the field stays if the key dialog of an `ask-key`
   need with a slot uses it), `model/add_account_dialog.rs`,
   `view/add_account_dialog.rs`, their intents and texts (all under
   `crates/app/features/agents/core/src/`). The "+ Add account" entry of
   the left panel (`view/left_panel.rs`, `AgentsIntent::OpenAddAccount`)
   is not shown until issue 08 brings the new dialog.
2. **The menu.** Edit key… and the Remove of an account added by hand go
   from the account menu (`model/menus.rs`, `menu_actions.rs`, the
   account part of `model/remove_dialog.rs`).
3. **The model.** In `crates/agents/src/`: `Account::added`,
   `Account::added_key`, the added key of `AccountLogin`, the "added by
   you" place of `FoundIn` (and the text `agents_found_in_user`), reading
   an added key in `own_reads.rs` of the app and in `given_key.rs` (the
   `KeyRef` of a given key with a slot stays), and the added accounts of
   `saved_profiles/saved_accounts.rs`. In `agents.json` an account entry
   has no `key` any more (`KeyEntry` and the `key` of `AccountEntry` in
   `saved_profiles/file_content.rs`). No reader of the old shape
   (Decision 12).
4. **The refused key of an added account.** The `ask-key` need with no
   slot goes, in OTTY (the "replace the key where it sits" part of issue
   26) and in the official plugins that give it (`plugins/deepseek` and
   `plugins/openrouter` `needs.rs`, and any other found with
   `grep -n AskKey plugins/*/src`). The `ask-key` needs with a slot of
   `plugins/openai` and `plugins/anthropic` stay.
5. **The manifest.** `add_by_hand` of `[[agents.provider]]` goes
   (`ProviderPart::can_add_by_hand` in `crates/plugins/manifest`,
   `ProviderEntry::can_add_by_hand` and `with_add_by_hand` in
   `crates/agents/src/provider.rs`, `PluginProvider::with_add_by_hand`).
   The manifest refuses unknown fields, so every official manifest that
   has `add_by_hand` drops it in this issue. Bump each changed plugin's
   patch version and rebuild with `plugins/build.sh`.
6. **The host.** The doc comment of `given-in-otty` of the `key` resource
   (`crates/plugins/sdk/wit/host/0.1.0/host.wit`) no longer names an
   account added by hand: a key given in OTTY is one given for an
   `ask-key` slot. Doc only, no shape change.
7. Module docs follow. The public docs are issue 10.

## Done when

- Tests first: the account menu has no Edit key… and no Remove, the left
  panel offers no Add account, an `ask-key` need with a slot still asks
  for its key and the key reaches the next read under its slot, a
  manifest with `add_by_hand` is refused, and `agents.json` keeps names,
  Hide and given keys of accounts with no `key` field.
- The checks of `AGENTS.md` pass.

## Comments
