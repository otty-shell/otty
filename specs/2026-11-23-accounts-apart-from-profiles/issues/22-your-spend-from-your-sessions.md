# 22 A key's account shows what you spent through it, counted by its plugin

Status: done
Blocked by: 21

## Why

Owner, 2026-11-25: the Semrush Inc. organization (an Anthropic API key,
the owner is an employee, not an admin) shows no money since issue 01,
and an admin key will never come. The owner wants a number on the
account, done "purely as a change in the provider": the Anthropic plugin
is both Claude Code and the Anthropic provider, so its provider part can
count what went through the key in Claude Code's own sessions and price
it, with no profile↔account link in OTTY (spec Decision 1). The "Needs
from you" admin-key need stays as it is.

## What to build

1. **Prices for a provider part.** The host's `price-of` (today an
   import of the agent contract only, answered during a read of answers,
   `prices.wit` of `otty:agents-agent`) is open to a provider part too,
   during its `numbers` call, so a provider can price tokens the same way
   an agent part does. Change the WIT in place, SDK, host.
2. **Your spend this month.** The Anthropic provider part, for an account
   known by a key (an organization or a key account found in Claude Code
   folders), reads the Claude Code sessions in the folders it was found
   in (its `[[agents.provider.logins]]` entry gets the `read` globs it
   needs, such as `projects/**/*.jsonl`), sums this calendar month's
   tokens of the answers that went to Anthropic, prices each with
   `price-of` (`cost_at`, by kind), and returns it as the account's
   money: "Your spend this month ~$42.10, at API prices, from your Claude
   Code sessions". Use or extend the existing account money shape (the
   "spent this month" figure of issue 01) with the source and the "at
   API prices" mark, so OTTY shows it with the mark and its tooltip by
   kind (issue 16). An account with an admin key keeps the real bill
   first; this figure is for accounts without one. A subscription login
   shows its limits, not this.
3. **The OpenAI plugin** does the same for a key found in Codex folders,
   if its sessions say which provider served each answer; if they can
   not tell a key's answers apart, leave it and say so in the comment.
4. Words in all ten languages, the plugin pages say what the figure is
   and that it only counts this computer's sessions of that agent. Bump
   and rebuild changed plugins and the host test plugins.

## Done when

- Tests first: a provider part can price tokens during `numbers`; the
  Anthropic provider gives a key account this month's spend from the
  Claude Code sessions of its folders, priced by kind, marked at API
  prices; answers of another month or another provider are left out; an
  account with an admin key still shows its bill; the account page shows
  the figure with the mark and the tooltip by kind.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-25, owner decisions, added to this issue:
  1. Keep the contract change as small as it can be: no new function
     and no new interface. The provider world imports the `prices`
     interface the agent parts already use. The figure comes back in
     the existing `account-numbers.spent-this-month` money, with one
     optional field that tells it apart from a real bill. To make that
     field possible, `priced-cost` moves as it is from `agent` into the
     shared `prices` interface (option A).
  2. OpenAI: yes. A key found in Codex folders gets the same figure. A
     folder's answers count for the account that pays for that folder
     now, and the plugin page says so.
  3. "This month" is the user's calendar month, not UTC. The host
     `clock` interface gets one function that gives the local offset
     from UTC at a time, the SDK wraps it, and the plugin works out the
     start of the month with it. Tests use a fixed offset.

- 2026-11-25, done. A key or an organization found in Claude Code or
  Codex folders shows "Your spend this month ~$42.10", marked "at API
  prices", with its money by kind on hover and the line "From your
  Claude Code sessions on this computer".

  What changed in the WIT, and why:
  - `prices.wit`: the record `priced-cost` moved here as it is from the
    `agent` interface. `agent.wit` now uses it with
    `use prices.{provider-ref, priced-cost}`. Why: a provider part can
    not use a type of the `agent` interface, which only an agent part
    exports, so the money by kind had to sit in an interface both
    worlds import.
  - `provider.wit`: the world `provider-part` imports
    `otty:agents-agent/prices@0.1.0` (one line), so a provider part may
    call `price-of` during `numbers`. The record `money` gets one field,
    `by-kind: option<priced-cost>`. Its presence means "at API prices",
    and its parts give the tooltip. Money the provider told has none.
    The interface uses `priced-cost` from `prices`. No new function,
    record or interface.
  - `host.wit`: the `clock` interface gets
    `local-offset-minutes: func(at-ms: s64) -> s32`, the user's offset
    from UTC at a time, so the month starts at midnight of its first day
    on the user's clock (owner decision 3).

  SDK: `agents::PricedCost` now comes from `prices`, and the provider
  bindings map `prices` to the agent bindings, so `agents::price_of`
  works in a provider part. `clock::local_offset_minutes` wraps the
  host call. A new `agents::MonthSpend` adds up the answers of the
  user's month kind by kind, each identity once
  (`this_month`, `since`, `since_ms`, `add`, `cost`, `money`). It
  counts no written cost, and with answers but no price at all it is
  "not known". `TestHost::with_local_offset_minutes` sets the clock of a
  test. It uses `BTreeSet`, so no plugin imports `wasi:random`.

  Host: `ProviderCalls::numbers` takes the app's `PriceList` and answers
  `prices.price-of` for that call only, as `read_answers` does. The
  clock offset comes from `jiff`'s system time zone, the crate and
  features `otty-app-agents-io` already uses: `jiff` is now a dependency
  of `otty-plugin-host` too, but `Cargo.lock` gets no new crate. With no
  readable time zone the offset is 0.

  `otty-agents`: `read_account_numbers` and `PluginProvider::read` take
  `&Arc<ApiPrices>`. `Balance::spent_by_kind()` (a `MoneyByKind`, boxed
  so the app's messages stay small) is set from `by-kind`. A
  subscription ignores it.

  Screen: the Money block of an account page shows the bill first, then
  the money the provider counted, and only then "Your spend this month"
  (`SpentAtApiPricesViewModel`): "~$42.10" with its kinds on hover, the
  mark "at API prices" with its tooltip, and the source line named after
  the agent of the first profile the account was found in. Before the
  list prices are read it is not shown. The Now of All accounts shows
  "~$42.10 this month". When the list prices change, the numbers of
  every account paid per token are read again. A read still on its way
  at that time is read again once it ends. Two new texts in all ten
  languages: `agents_your_spend_this_month` and
  `tpl_agents_spent_from_sessions`.

  Plugins:
  - `anthropic` 0.2.17 -> 0.2.18. The `logins` entry reads
    `projects/*/**/*.jsonl`. Its session file reading
    (`spend_file::read_answers` and the agent part's glob, now
    `claude_code::session_files_glob`) is reused, so there is no second
    parser. `logins::paying_account` picks the account that pays for a
    folder now:
    - the key of `settings.json`, else the Console organization of the
      login, else the Console key;
    - none for a Claude subscription, an admin key, a folder whose
      `ANTHROPIC_BASE_URL` points to another server, and OTTY's own
      login.

    So the Console key of an organization login shows nothing, and the
    organization shows the spend. When the server read of an account
    paid per token fails without a need, it still gives its spend.
  - `openai` 0.2.16 -> 0.2.17. Codex sessions name the provider of each
    session (`model_provider`), but not the login that served it, so a
    folder's answers served by OpenAI count for the API key that folder
    logs in with now (owner decision 2). It reuses
    `spend_file::read_profile_answers` and `session_file_globs`, which
    count plain, compressed and archived copies once. Admin keys and
    ChatGPT logins get nothing.
  - Template `example` 0.1.10 -> 0.1.11: its key gets this month's
    spend from `sessions/*.jsonl`, as the example for plugin authors.
  - `opencode` 0.1.11 -> 0.1.12, `deepseek` and `openrouter` 0.1.7 ->
    0.1.8: rebuilt for the changed types (`by_kind: None`).

  All of them were rebuilt into `assets/plugins/`, and the host test
  plugins and `template.wasm` were rebuilt.

  Docs:
  - The Anthropic and OpenAI pages have a section "Your spend this
    month": what it counts, that it counts only this computer's sessions
    of that agent, the folder rule, and that it is an estimate.
  - The versions of all official pages, Write a Plugin (`by_kind`,
    `MonthSpend`, `price_of` during `numbers`, `clock` offset, template
    0.1.11), Overview, the SDK README and the READMEs of the anthropic,
    openai and template plugins.

  Tests first: they were committed before the code (`wip: issue 22
  tests`), and they failed to build: there was no `MonthSpend`,
  `local_offset_minutes`, `with_local_offset_minutes`, `by_kind`,
  `spent_by_kind`, `SpentAtApiPricesViewModel` or new key yet, and
  `numbers` took no prices. They cover:
  - SDK: the month start in UTC, Moscow and New York and on a leap day;
    kinds adding up; another month and a copied answer left out; no
    price is not known.
  - Host: the system offset, also in summer time; the template prices
    its sessions during `numbers` at the price list given, and has no
    spend with no price.
  - `otty-agents`: by kind kept from `by-kind`, none for the provider's
    own count, the prices reaching the part.
  - Anthropic, with made-up session files: a settings key's month with a
    copy, a helper agent and October left out; an organization counting
    only the folder that goes to Anthropic, its Console key nothing; a
    subscription, an own login and no price nothing; the numbers of a
    key carrying it.
  - OpenAI: a key's month with an archived copy, October and a local
    model left out; subscription, admin key and own login nothing.
  - Core: the account page figure with the mark, kinds and source; the
    bill coming first; the Now "~"; the account page and the tooltip
    drawn; new prices reading accounts paid per token again.

  Two older plugin tests changed after the code: "an API key has no
  numbers" now expects its spend of $0.00.

  Left as it is: the figure counts a folder's answers for its current
  payer, also those from before a login change this month. The
  per-model price of a request is the part's; OTTY only adds.
