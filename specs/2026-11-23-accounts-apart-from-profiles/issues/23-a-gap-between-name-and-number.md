# 23 A left panel row keeps a gap between its name and its number

Status: ready
Blocked by: 22

## Why

Owner, 2026-11-25: in the Agents left panel, on a small screen the
numbers of the accounts start to cover the email, right against it. At
any width there must be at least some space between the text and the
numbers.

Since issue 20 the kit `TreeRow` with `keep_note()` keeps the note whole
at the right and cuts the label with "…", but nothing keeps a gap between
the cut label (and its tag) and the note.

## What to build

1. In `TreeRow` (`crates/ui/kit/src/components/tree_row.rs`), with and
   without `keep_note()`, the label (or the tag after it) and the note
   always have at least `spacing::SPACE_3` between them, at any width: the
   label is cut earlier to leave it, and nothing draws over anything.
   `cuts_label` (which decides the tooltip) counts the same gap.
2. Check with pictures at a few narrow widths (throwaway test, not
   committed) and pin the geometry.

## Done when

- Tests first: at widths from very narrow to wide, an account row's label
  (or tag) ends at least `SPACE_3` before its note begins and they never
  overlap; a Files tree row too; `cuts_label` agrees with the drawn row.
- The checks of `AGENTS.md` pass.

## Comments
