# 38 Account chips name an account by its login

Status: done
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

   **The project page shrinks like every other screen.** Owner: the main
   screen of Agents should do what the other screens do: shrink down to
   their minimum width, then scroll sideways, by the same code, not a
   second way. Every other screen of the main area (Agents' profile,
   account, All accounts and All profiles pages, Containers, Quick
   Launch, Plugins) is a `MainContent::Screen` in the kit's `ScreenArea`
   (`crates/ui/kit/src/components/screen_area.rs`, minimum
   `size::SCREEN_MIN`); the project page (`MainAreaViewModel::Project`
   in `crates/app/features/agents/core/src/lib.rs` `main_area`) is the
   only one that is `ScrollsOnItsOwn`, because its sessions table
   scrolls on its own, so its header (facts, Pays through chips) squeezes
   without limit. Make the project page keep the same minimum width and
   scroll sideways below it with the same mechanism as the other screens
   (reuse `ScreenArea` or what it is built on; read its module doc on why
   `responsive`/`DataTable` does not fit inside it, and how Containers
   puts its tables in screens, before choosing). Do not add a second
   minimum-width mechanism. The table keeps scrolling up and down, and
   one scroller per direction: nothing scrolls twice.
5. Docs that show these chips follow.
6. **No path under the title of a project.** Owner, 2026-11-22: on the
   Agents project page, the folder path shown right after the page
   title goes: Project info already shows the path, so it is a
   duplicate. The path stays in Project info.

## Done when

- Tests first: a "Pays through" chip of a profile page and of a project
  shows the login without the provider, a user's name as given, a shared
  account with its plugin, and the full name on hover; both build the chip
  with the one shared function; at a narrow width a chip's name is cut
  with an ellipsis on one line, its icon and number keep their place, and
  chips wrap to the next line.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-22, done. One chip of an account, `AccountChipViewModel`, built
by `model/account_chip.rs` `account_chip` and drawn by `view/chips.rs`
`account_chips`; "Pays through" of a profile and the accounts of a
project both use them (`PaysThroughViewModel` is gone). The chip names
the account with `short_account_name` (the name the user gave it, else
the login, with its plugin when two plugins read it); its tooltip is the
whole name with the provider ("OpenAI · you@example.com"), after the
user's name on a line of its own when there is one, and over the
tooltip of an estimate when the number has one. Of the other places
with `account_name(`, only the title of an account's page has the logo
beside it: it shows the short name too (its screen key keeps the whole
name, so two logins of two providers do not share a scroll position).
The All accounts table, the usage groups, the session account, the
needs and Found by OTTY have no logo and keep the whole name.

A chip never wraps its texts: the name is an `EllipsisText` that leaves
the number its width, the icon and the number keep their size, the
chip is at most as wide as its line, and the wrapping row moves a chip
to the next line before any chip is cut. Tested at 320 px.

The project page is a screen now: `MainContent::TableScreen` puts it in
the same `ScreenArea` with the same `size::SCREEN_MIN`, built with the
new `ScreenArea::filling_height`, whose scroller scrolls only sideways,
so the page gets the room's height and the sessions table fills the
rest and scrolls down on its own under its sticky header. Below the
minimum the page scrolls sideways as every screen. To keep one scroller
per direction, the minimum of the Title column went from 220 to 198 px:
the four minimums and the "⋯" column (524 px) now fit the smallest
screen less the page padding, so the table never scrolls sideways
itself. Containers' list table is still `ScrollsOnItsOwn` (out of scope):
it could move to `TableScreen` the same way.

Item 6: the title row shows the folder name only, the path stays in
Project info; the tests of the short path, its tooltip and its baseline
went with it. Docs: Overview.mdx.
