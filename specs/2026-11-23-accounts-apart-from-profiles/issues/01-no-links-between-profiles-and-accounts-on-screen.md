# 01 Profiles and accounts stop pointing at each other on screen

Status: done
Blocked by: none

## Why

Decision 1: nothing links a profile to an account, so there is no "Pays
through", no "Used by" and no tokens by account. Decision 3: a profile's
money is its tokens at the API list prices, on a subscription too, always
marked "at API prices" and never disabled.

These are the parts of the spec that change only the screen and need no
change of the plugin contracts. Doing them first leaves less to move in
the later issues. See "Before → after" of the spec: Profile page, Project
page, Account page, All accounts, Usage block and statistics popup.

## What to build

All paths are under `crates/app/features/agents/core/src/` unless named
otherwise.

1. **No Pays through.** The profile page loses its Pays through chips
   (`model/profile_page.rs` `pays_through`, `view/profile_page.rs`) and
   the text `agents_pays_through`.
2. **No accounts on a project.** Project info keeps the Profiles chips
   only (`model/project.rs`, `view/project.rs`). The sessions table loses
   its Account column (`sessions_table.rs`, `model/session_account.rs`).
   The account chip (`model/account_chip.rs`, `AccountChipViewModel`,
   `view/chips.rs` `account_chips`) goes once nothing uses it. The
   narrow-width behaviour of the profile chips from issue 38 stays.
3. **No Used by.** The account page loses Used by (`model/used_by.rs`,
   `view/account_page.rs`, the text `agents_used_by`).
4. **No By account.** The usage tabs are By project | By model on the
   profile page, By profile | By model in Project statistics, and By
   model in Session statistics (`model/usage_groups.rs`,
   `model/usage_block.rs`, `model/stats_dialog.rs`, `usage.rs`). The
   Account line of Session statistics goes. The By account switch of
   issue 40 goes with its texts (`agents_by_account`,
   `tpl_agents_by_account_off`).
5. **No spend estimate of a key.** An account's money is only what its
   provider part read (money left, spent this month, key limit) and its
   bill. The estimate over the tokens of the profiles that pay through a
   key goes: `AccountMoney::Estimate` (`crates/agents/src/account_money.rs`),
   the key part of `crates/agents/src/money_estimate.rs` (`MoneyEstimate`,
   keep `MoneyTotal` and `MoneyTotal::of_models`), `Account::pays_for`
   (`crates/agents/src/account.rs`) and their use in
   `model/account_numbers.rs`. The Now column of All accounts shows the
   percent used, the money left or the money spent this month as the
   part read it, else "—".
6. **Money of a profile at API prices.** The tokens of every profile are
   priced per model at the LiteLLM list prices, on a subscription too.
   Money is never disabled in the usage block: the text
   `agents_money_subscription` goes, and so does what decides that a
   profile "pays per token" where it turns money off (such as
   `is_priced` of a usage source in `model/usage_groups.rs`, and the
   `billing` of a profile's connection where it is read for that).
7. **The mark "at API prices".** The mark "estimate, list prices"
   (`agents_estimate_mark`) becomes "at API prices" in all ten locales
   (`crates/app/i18n/src/locales/`), and its tooltip says the tokens are
   priced at the API list prices fetched at <time>.
   `MoneyKind::Estimate` and `EstimateMarkViewModel` (`model/money.rs`)
   are renamed for what they are now, for example `AtApiPrices` and
   `ApiPricesMarkViewModel`.
8. **A cost the agent writes** (OpenCode writes one per answer) stays as
   today: exact, with no mark. The spec leaves it open ("A cost the agent
   writes"), and this issue does not decide it.
9. The module docs of `crates/agents/src/lib.rs` and of the changed
   modules follow. The public docs are issue 10.

## Done when

- Tests first: a profile page has no Pays through, a project page has no
  account chips and its sessions table no Account column, an account
  page has no Used by, the usage block and the statistics popups offer
  no By account tab and Session statistics no Account line, money of a
  profile logged in with a subscription is shown and marked "at API
  prices", and the Now of a key account with no money read is "—".
- The checks of `AGENTS.md` pass.

## Comments

2026-11-23, done. The screens no longer link profiles and accounts: no
Pays through on a profile page, no account chips in Project info, no
Used by on an account page, no By account tab or switch, and no Account
line in Session statistics. The sessions table had no Account column
any more. The account chip, `used_by.rs`, `session_account.rs`, the By
account grouping, `AccountMoney`, `MoneyEstimate` and `Account::pays_for`
are gone, with their texts.

The money of every profile is its tokens at the LiteLLM list prices, on
a subscription too, marked "at API prices" in all ten locales
(`MoneyKind::AtApiPrices`, `ApiPricesMarkViewModel`). So the list prices
are now read whenever a profile is shown, not only with an API key
profile. A cost the agent writes (OpenCode) stays exact, with no mark.
The Cost of Project statistics always shows, with every profile priced.

An account's money is only what its provider part read: the bill, or the
money left, the key limit and what was spent this month. The Now of a
key account with nothing read is "—", and the Money block of an
organization whose bill failed shows only why.

Kept on purpose for later issues: the bill of an organization is still
read through an API key profile (`ProfileState::pays_through`,
`is_api_key`), and the "installed, not used" line of a profile still
looks at the logins found in it; issue 06 removes the logins from the
agent contract. Money stays off in the usage block only while no list
prices were read ("The list prices are not read yet"), since there is
nothing to count it with. The public docs are issue 10.
