# 03 Accounts added by hand, by the place of their key, go

Status: done
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

2026-11-23, done. Accounts can no longer be added by the place of their
key. The Add account dialog, Edit key…, the Remove of an account and the
`+` of the Accounts section are gone, with `OpenAddAccount`,
`AccountAction::EditKey` and `AccountAction::Remove`. The section header
keeps only Refresh until issue 08 brings the new Add account.

The dialog of an `ask-key` need with a slot stays, as the owner decided. It is
now its own small dialog, `key_dialog.rs` (`KeyDialog`,
`KeyDialogIntent`, `AgentsIntent::KeyDialog`, the overlay `KeyDialog`),
in place of `add_account_dialog.rs`. It shows the part's label, hint,
prefix and page, the provider of the account, and the key place field,
and saves the place under the part's slot. `key_place_field.rs` stays
whole, since this dialog uses it.

In `crates/agents`: `Account::added`, `Account::added_key`,
`AccountIdentity::AddedKey`, `AccountLogin::KeyPlace`,
`FoundIn::AddedByUser`, `ProviderEntry::can_add_by_hand` and
`with_add_by_hand`, `Registry::addable_providers`, `own_reader_of` (now
just `Registry::reader_of`), `PluginProvider::read_given` and
`read_given_bill`, `KeyAsk::for_account_key`, the `KeyPlace` enum of
`read_account_numbers` (it takes the profiles the account was found in)
and `AccountNumbers::Bill` (only an added account read its bill this
way; the bill of an organization is still read through its profile) are
gone. `SavedProfiles` lost `added_accounts`, `with_account_added`,
`with_added_key` and `without_added_account`; an account entry of
`agents.json` has no `key`, and an old entry known only by a key is not
read. `FoundIn` keeps one variant, `ProfileFolder`, until issue 05 adds
OTTY's own logins.

`KeyAsk::slot` is a `String` now. A need whose `key-ask` has no slot is
not shown. The WIT shape stays (`slot: option<string>`, Decision 12, no
version bump); only the doc comments of `key-ask` and `given-in-otty`
changed. A later shape cleanup could make the slot required.

The manifest refuses `add_by_hand` as an unknown field. Plugins:
`openai` 0.2.6 → 0.2.7 and `anthropic` 0.2.6 → 0.2.7 drop `add_by_hand`
and always ask for a refused admin key under `admin`; `deepseek` 0.1.1 →
0.1.2 and `openrouter` 0.1.1 → 0.1.2 drop `add_by_hand` and the
`replace-key` need, so a refused key always opens the keys page; the
template `example` 0.1.1 → 0.1.2 does the same. All five were rebuilt
into `assets/plugins/`. The template copy the host tests load,
`crates/plugins/host/tests/fixtures/template.wasm`, was rebuilt too: it
was older than issue 02, so its host test now expects only the default
folder. A spend test reads the template's version from its manifest
instead of writing it.

Texts gone from all ten locales: `agents_found_in_user`,
`agents_edit_key_item`, `agents_add_account_title`,
`agents_edit_key_title`, `agents_add_account_text`,
`agents_error_account_added`. The public docs are issue 10.
