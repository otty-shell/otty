# 11 Prices load before the first count; no profile↔account link inside OTTY

Status: done
Blocked by: 07

## Why

Two leftovers of issue 06 (see its comment):

- At start the first token count runs before the price list is loaded,
  so once prices arrive every shown profile is counted again: the
  session files are read twice at start-up.
- With the connection gone from the agent contract, OTTY still decides
  that a profile "pays with an API key" when the accounts found in it are
  only keys or organizations. Nothing on screen shows it, but it is a
  profile↔account link inside OTTY, against Decision 1 of the spec ("two
  independent slots, no link between them").

## What to build

1. **Prices before the first count.** At start OTTY loads the price list
   (from its cache when there is one) before it starts the first token
   count, so a start reads the session files once. A count started with
   no prices at all (first run, offline, empty cache) still works and is
   redone once prices arrive, as today.
2. **No inner link.** Find every place in `otty-agents` and the agents
   app feature that still reads which accounts were found in a profile
   to decide something about the profile (the "pays with an API key"
   check, and anything else of the kind, e.g. how an organization bill
   is read), and remove it or move the decision to the provider part,
   which owns its accounts and their bills. Internal scheduling that
   reads an account's numbers through a folder it was found in may stay
   (the account knows its own folders), but no profile decision may
   depend on accounts.

## Done when

- Tests first: a start with cached prices counts each shown profile once;
  a start with no prices counts once and once more when prices arrive; no
  profile state or view model depends on the accounts found in it (a test
  that a profile's page model is the same with and without accounts).
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-23, done. The first count waits for the first price read, and
  a profile no longer knows the accounts found in its folder.

  Prices before the first count: while the first read of the list prices
  is on its way (none answered yet), a count of the tokens is not
  started; the profile waits for the prices
  (`ListPrices::is_first_read_on_its_way`,
  `ProfileSpend::wait_for_prices`) and is counted once they answer, with
  prices or without. Later new prices count again, as before. So a start
  reads the session files once; a start with no prices at all counts
  once, and once more when prices arrive. The start no longer waits for
  the limits before the prices and the bills (`StartWait` is gone): the
  profiles come first, then the sessions, the prices, the numbers of the
  accounts and the bills together.

  Inner links found and what became of them:

  1. "Pays with an API key" (`ProfileState::accounts_say_api_key`,
     `is_api_key`, `billing_kind`, `reads_limits`): gone, with
     `BillingKind` and `Billing::kind` of `otty-agents`, which only it
     used.
  2. The limits of a profile: a profile read the limits of the first
     subscription found in it (`limits_account`, `LoadLimits`,
     `LoadLimitsInBackground`, `LimitsLoaded`, `load_limits` of the io
     crate) and a profile not logged in read none. Now every account a
     provider part reads has its numbers read by itself
     (`AccountNumbersLoaded`, `account_reads.rs`, which replaces
     `own_reads.rs`), in the folders it was found in, whatever the status
     of a profile. The wait for the agent's program (a read while the
     mode was away that gave nothing is read again on screen) moved with
     it.
  3. The bill of an organization: it was read through a profile that
     "pays with an API key" and kept on the profile (`organization`,
     `pays_through`, `bill`). Now each organization a part reads has its
     bill read once, keyed by the organization (`LoadBill` and
     `LoadedBill` carry the organization, no profile). Whether an account
     is an organization with a bill is the provider part's call. An
     account whose given keys change is read again.
  4. The profile page said "installed, not used yet" only when no
     account was found in the folder; now only the agent part's "not
     logged in" decides.
  5. A profile showed "the plugin could not read the limits" from the
     limits read through it; now the failure of an account's read shows
     on the account only, and the text `agents_plugin_read_limits` is
     gone from the ten locales.
  6. `ProfileState` held the accounts found in its folder and
     `Account::is_found_in` existed to pick them: both are gone.

  "Every limit source failed with an unexpected answer" is now told for
  the read of an account's numbers, named after the account.

  Tests first, each seen failing: a start with kept prices counts each
  shown profile once, a start with no prices counts once and once more
  when prices arrive, a profile page is the same with and without
  accounts (logged in or not), a subscription is read as the numbers of
  its account, an account that waits for its program is read again on
  screen, the bill of an organization is read once whatever its folders
  hold, and a part that could not read an account asks on the account
  only. The tests of the removed profile reads were moved to the
  accounts or removed with the concept. No plugin changed.
