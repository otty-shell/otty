# 38 Account chips name an account by its login

Status: ready
Blocked by: 34

## Why

Owner, 2026-11-22: the "Pays through" chips of Agents mode still show the
provider name ("Anthropic · you@example.com"), though the chip already
shows the provider's logo. As in the left panel since issue 34, the logo
and the account are enough.

## What to build

1. The account chips (`crates/app/features/agents/core/src/view/chips.rs`,
   built in `model/profile_page.rs` `pays_through` and in
   `model/project.rs`) name the account with `short_account_name`
   (`model/account_name.rs`): the name the user gave it, else its login
   alone, with its plugin when two plugins read it. The provider stays in
   the chip's tooltip if the chip has one, else add it as the tooltip
   ("Anthropic · you@example.com"), so it can still be read on hover.
2. Look for other places where an account is shown next to its provider
   logo with the full name (grep `account_name(` in the agents feature,
   e.g. usage groups, session account) and use the short name there too
   only where the logo is shown beside it. Places with no logo keep the
   full name.
3. **One account chip, made in one place.** Owner: every chip of an
   account (profile page, project, any other) must look the same and be
   changed in one place in the code, with no duplication. Today
   `pays_through` (`model/profile_page.rs`) and `model/project.rs` each
   build `PaysThroughViewModel` by hand (icon, name, number). Make one
   function that builds the chip of an account from the state (and name
   the type for what it is, e.g. `AccountChipViewModel`), and make every
   place that shows an account chip use it and `view/chips.rs`
   `account_chips`. Projects show the same chip as profiles.
4. **Chips stay readable in a narrow window.** Owner: when the window is
   made narrow the chips become unreadable. The row already wraps
   (`view/chips.rs` `view`), but a chip's own texts wrap inside the chip,
   letter by letter. A chip never wraps its texts: its icon and its
   number keep their size, the name is cut with an ellipsis
   (`otty-ui-kit` `EllipsisText`) when the chip is wider than the space
   it has, and the chip is at most as wide as that space. Chips move to
   the next line before any of them is cut. The full name stays readable
   in the tooltip. Check the profile page and the project page at a
   narrow width (e.g. the page column at 320 px).
5. Docs that show these chips follow.

## Done when

- Tests first: a "Pays through" chip of a profile page and of a project
  shows the login without the provider, a user's name as given, a shared
  account with its plugin, and the full name on hover; both build the chip
  with the one shared function; at a narrow width a chip's name is cut
  with an ellipsis on one line, its icon and number keep their place, and
  chips wrap to the next line.
- The checks of `AGENTS.md` pass.

## Comments
