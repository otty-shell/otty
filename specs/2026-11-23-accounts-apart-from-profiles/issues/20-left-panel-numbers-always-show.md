# 20 In the Agents left panel the numbers always show, the name gives way

Status: done
Blocked by: 19

## Why

Owner, 2026-11-25: in the Accounts section of the left panel the number
after an email ("42 %", "$12.40") does not show on a normal screen: the
emails are long and the row collapses, which reads poorly. The projects'
numbers look fine, but even there the owner would always draw the number
and cut the name step by step instead: a name can be found through the
search or by pressing the row, a number can not.

Cause: the kit `TreeRow` (`crates/ui/kit/src/components/tree_row.rs`)
gives the label the room it needs first and cuts the note on the right
("The label takes the room it needs first; the note on the right takes
what is left and ends with …").

## What to build

1. **A row that keeps its note.** `TreeRow` gets a way to say its note
   comes first (a builder method such as `keep_note()`): the note is never
   cut and keeps its width at the right, and the label takes what is left
   and ends with "…" when that is too little. Rows that do not ask keep
   today's rule (the Files tree, Quick Launch and others are unchanged).
2. **The Agents left panel uses it** for every row that has a number:
   accounts, profiles and projects. A cut name shows the whole name in
   the kit tooltip on hover (the account's whole name has its provider, as
   the chips' tooltip since issue 38 of spec 2026-11-19-agent-plugins).
3. The logo or icon and any tag (such as "OTTY" of an own login) stay
   whole; only the name gives way.

## Done when

- Tests first: in a narrow panel an account row with a long email shows
  its whole number at the right and its email cut with "…", the whole
  email in the tooltip; the same for a project and a profile; a Files
  tree row still cuts its note first; a row whose label fits shows both
  whole.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-25, done. The kit `TreeRow` got `keep_note()`: the state word
and the count keep their width at the right edge, the label takes what
is left and ends with "…", and a tag such as "OTTY" stays whole after
it. It also got `whole_label(..)`: while the pointer is over the row and
its label is cut, the kit tooltip shows the whole label; a label that
fits shows none. The row tells a cut label by the same room its layout
gives the label, checked against the drawn row at many widths. Rows that
do not ask keep the old rule, so the Files tree and Quick Launch are
unchanged. The Agents left panel uses both for accounts, profiles and
projects. An account's tooltip is its provider with its login, such as
"OpenAI (you@example.com)", after the name the user gave it, if any. A
row with a broken plugin icon keeps one tooltip: the whole name, then
the plugin note under it. `TooltipText` got `line(..)` for that.
