# 19 Remove "Where the numbers come from"; a part reads its limits its own way

Status: ready-for-agent
Blocked by: 18

## Why

Owner, 2026-11-20: where a number comes from is the plugin's business,
not the user's. Today the agent contract exports `limit-sources` and
OTTY walks those sources one by one through `read-limits(…, source)`,
then shows every source with its state in a table at the bottom of the
account and profile pages. That puts a plugin's inner order into OTTY
and on screen. The user needs to know only whether to trust a number and
what to do, and the page already says both.

No compatibility with earlier versions: neither Agents mode nor the
plugin contracts were released, so `0.1.0` changes in place.

## What to build

1. Contract `otty:agent@0.1.0`: remove `limit-sources`; `read-limits`
   becomes `read-limits(profile, login)`. The part tries its own
   program, files or servers in its own order and returns the limits or
   one `read-error`. Update the SDK trait (`limit_sources` goes, the
   default of `read_limits` stays "not supported"), the SDK test host,
   the template and the four plugins that read limits (anthropic,
   openai, opencode if it does, and any other). Rebuild the embedded
   copies with `plugins/embed.sh`. Check the provider contract for the
   same pattern and remove it there too if it exists.
2. OTTY keeps one order rule, because parts can come from different
   plugins (OpenCode with DeepSeek): for each number it asks the
   provider part first, then the agent part; the first answer wins. It
   keeps the walk internal and does not show it.
3. Remove the table "Where the numbers come from" from the account page
   and the profile page: the model (`model/sources.rs`), the view
   (`view/sources.rs`), their uses, its i18n keys in all locales, and
   types that only fed it (source names given by parts, `SourceId`
   variants, states such as "not needed") when nothing else needs them.
4. What stays on the pages, each already present or moved, not new:
   - the time of the data, and the last good numbers greyed with their
     time when a read fails;
   - "Needs from you" for anything the user must do (login expired →
     open the agent; format changed → update the plugin). Every failure
     the table showed that needs an action must still reach the user as
     a need or as the tooltip of the greyed number; check this case by
     case and say in the comments where each went;
   - money that is an estimate from LiteLLM list prices, not the
     provider's bill, says so with a short mark next to the sum (for
     example "estimate, list prices"), with a tooltip;
   - "Read by" in Account info and Profile info.
5. Plugins page, card of a part: the last failed or refused call of the
   part, with its time and kind (never a value, key or session text),
   and "Copy report" there if the report exists elsewhere. This is where
   plugin authors look now.
6. Docs: remove the section "Where the numbers come from" and its
   diagram from `docs/public/Agents/Overview.mdx`, the table mentions in
   the pages table, and the lists of sources on the official plugin
   pages; mention the last failure on the part card in `Plugins.mdx`;
   update `Write a Plugin.mdx`, the SDK and template READMEs for the new
   `read_limits`. Update `spec.md` of this spec (contract listing, UI
   changes, the Decisions that describe the table) so it matches.

## Done when

- Tests first: a read that fails with an expired login still shows a
  need and greyed last numbers; a provider part answer wins over the
  agent part; an estimated sum carries the estimate mark; the part card
  shows the last failure.
- No "Where the numbers come from", `limit-sources` or `limit_sources`
  left outside `specs/` history comments and git history.
- The checks of `AGENTS.md` pass.

## Comments
- 2026-11-20, issue 19: done. Contract `otty:agent@0.1.0` changed in place: `limit-sources` is gone and `read-limits(profile, login)` lets the part pick its own sources. The SDK trait has only `read_limits(profile, login)` (default "not supported"). OpenAI's Codex part tries `codex app-server` (not started for an expired token), then its session files, and gives the `app-server` error when neither answers; Anthropic's Claude Code part reads `/usage`. Every plugin under `plugins/` and the host's test plugins were rebuilt; DeepSeek and OpenRouter have only a provider part, so their embedded copies did not change. The provider contract had no such list.
- 2026-11-20, issue 19: OTTY keeps one order rule (`read_profile_limits`): the provider part of the profile's subscription login first, then the agent part; the first answer wins. An agent part that answers "not supported" is no failure, so a profile nothing reads limits for still draws no limits. `SourceId` now has `AgentPart` and `ProviderPart` (was `Plugin` and `Named`); `SourceName`, `SessionFilesAtListPrices`, `CostWrittenByAgent`, `Agent::limit_source_ids`, the per-profile money estimate (`ProfileState::money`) and the list of "agents that read limits" went with the table. `Agent::limit_sources` is `Agent::limit_source` (one read, `None` for an agent with no part); `own_source_of` is `own_reader_of` (the provider entry). The host paces a slow reading per part and profile (no source any more).
- 2026-11-20, issue 19, where each failure the table showed now reaches the user. Limits: "login expired" (not logged in) → need "The login expired" [Open agent] and the greyed last numbers with why under them (account page; the profile page has no limits block); a refused key, a network error, a program not found, a stopped read, a key gone, "nothing of its own" → the line under the greyed limits ("Could not read the limits again: Plugin: …"); a plugin that failed or was paused → need "<Plugin> could not read limits" [Copy report] and the part's last failure on the Plugins page; every part failing with an unexpected answer → the notice "Could not read the usage of …" as before; "not needed" and "answered" → dropped, the time stays as "Updated … ago" and "recorded … ago". Tokens: format changed → need [Copy report, Open plugin repository] with the greyed usage block; any other count failure → the line of the usage block, and an unreadable folder → the notice. Money: no admin key → need "The real bill needs an admin key"; a bill that failed with an admin key (refused, file or variable missing or empty, network, plugin) → new line under the estimate on the account page, "Could not read the bill: …"; the estimate and its LiteLLM time → the new mark "estimate, list prices" with that time in its tooltip; no prices → the note "no prices" and a dash, as before; "from the bill" and the cost the agent wrote → dropped (the sum has no "~" and no mark); a key's own read (Left, Key limit) → the line "<Provider>: …" under the Money block, and a failed plugin → need [Copy report]. Prices: a failed fetch with a kept copy → dropped (no action); a broken price file → the notice, as before.
- 2026-11-20, issue 19: the Plugins page card of a part shows its last failed or refused call (`PartHealth::last_failure`): "Last failure 5 min ago: read-limits, the plugin did not finish in time" (a "could not read" adds the reason tag, such as "host error: not allowed"), and Copy report copies `PluginFailure::report`. Pausing does not replace the failure that paused it. A refused login key (`key-refused`) is not kept there: it is the user's to fix and shows as a need.
- 2026-11-20, issue 19, left for the owner: (1) the bill failure is only a line, not a need with **Admin key…**; (2) the usage block of an API key profile keeps its note "An estimate: the tokens at the list prices." and the left panel its "~", with no new mark; (3) a key refused by the agent part (Codex's expired token) is not taken as "login expired"; the provider part's refusal of the same login already is; (4) a subscription login found in a profile of a known agent is now read through that profile (provider part first), so an OpenCode profile with a ChatGPT login reads it there instead of by its own read; (5) the age on the part card counts from when the page was drawn; (6) a read of an agent part that stopped (thread panic) now counts as a failure once, even for a part that reads no limits.
- 2026-11-21, issue 19, owner's point (2), one mark everywhere: money estimated at the LiteLLM list prices has no "~" and no note "An estimate: the tokens at the list prices." anywhere; it carries the one mark "estimate, list prices" (`agents_estimate_mark`) with the one tooltip "Tokens from the session files at the LiteLLM prices updated {time}, an estimate" (`tpl_agents_money_estimate`). Under the sum: the Money block of the account page (as before) and the usage block of a profile page and of the session and project popups in Money (the mark under the table, where the note was). Where there is no room under the sum, the short form follows it, "$42.10 · estimate, list prices" (`tpl_agents_estimated_amount`, "{amount} · {mark}", which drops "this month"): the left panel, the Now column of All accounts, the Now column of Found by OTTY and the account chips of the profile and project pages, each with the same tooltip; and the Cost fact of a project popup, which has no tooltip because the kit's fact list has none. The keys `tpl_agents_about_amount`, `tpl_agents_about_this_month` and `agents_usage_money_note` are gone from all locales. Money from the bill or the cost the agent wrote has no mark.
- 2026-11-21, issue 19, one money type (owner): every shown sum is a `ShownMoney` (`model/money.rs`): the sum written and its `MoneyKind`, `Exact` (a bill, the provider's count, the cost the agent wrote, a dash with no prices) or `Estimate` with the mark and its tooltip (the time of the LiteLLM prices). It is made in one place, `ShownMoney::exact` or `ShownMoney::of_total` (estimate when `MoneyTotal::is_estimate`), through `account_numbers::shown_total` from the state's prices; the usage block takes `MoneyKind::at_prices`. Its forms: full, `view::money::mark` under the sums (account page Money block, usage block); tight, `short_text` / `month_short_text` and `tip`, drawn with `view::money::with_tip` (left panel rows, account chips) or `cell_with_tip` (All accounts, Found by OTTY). The short number of an account (`AccountNumber`) holds a word or the `ShownMoney` of the month and gives `text()`, `tip()` and `tone()`; the left panel, All accounts, Found by OTTY and the chips hold it as is. Gone: the per-place `estimate_tip` fields, `MoneyViewModel::estimate` (now `spent: Option<ShownMoney>`) and `UsageBlockViewModel::estimate` (now `money_kind`). All Profiles shows tokens only, so it has no money.
