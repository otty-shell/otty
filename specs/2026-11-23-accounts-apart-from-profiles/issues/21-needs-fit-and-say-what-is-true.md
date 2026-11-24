# 21 A need fits any width and says what is true now

Status: ready
Blocked by: 20

## Why

Owner, 2026-11-25, with a screenshot of an account's "Needs from you":

```
Anthropic (Semrush Inc. (Claude API Platform)), the real bill needs an admin   Anthropic
key                                                                            plugin  [Admin key…]
OTTY shows an estimate from the tokens. The bill of the whole organization
needs an admin key, and no agent keeps one.
```

"This behaviour must not be: work on the layout, if it does not fit we
must do something about it."

- The source label ("Anthropic plugin") wraps onto two lines and the
  button overlaps it; the heading wraps mid-phrase.
- The heading nests parentheses: the organization's own name from the
  plugin holds "(Claude API Platform)" inside "Anthropic (…)".
- The text is no longer true: since issue 01 OTTY shows no estimate from
  the tokens on an account; an account shows only the money its provider
  part reads. The owner asked why the money of the Semrush Inc. account
  stopped showing: it was that estimate. Without an admin key the
  organization has no money to show.

## What to build

1. **A need row that fits.** The need row (Agents needs view, and any
   other place that draws a need: account page, profile page, All
   accounts and All profiles "Needs you") lays out its parts so nothing
   overlaps or splits oddly at any width: the heading and text take the
   width, the source label ("Anthropic plugin") never wraps (one line, cut
   with "…" if it must, whole in a tooltip), and the actions stay whole.
   When the row is too narrow for text, label and actions side by side,
   the label and actions go to their own line under the text, aligned to
   the right (or the start), instead of squeezing. Check with pictures at
   a wide, a normal and a narrow width, and pin the geometry in tests
   (no overlap between label and button, the label on one line).
2. **No nested parentheses in names.** The Anthropic and OpenAI plugins
   name an organization without parentheses inside ("Semrush Inc., Claude
   API Platform" or just "Semrush Inc." with the kind elsewhere, choose
   what reads best and say why), so the full name is "Anthropic (Semrush
   Inc., Claude API Platform)". Check every name a plugin builds.
3. **True words.** The admin-key need of an organization says what is
   true now: OTTY shows no money for the organization until it has an
   admin key, which reads the real bill (no "estimate from the tokens").
   Fix it in the Anthropic and OpenAI plugins (all their languages) and
   anywhere else a text still promises the removed estimate (grep the
   locales and plugins for "estimate"). Bump and rebuild changed plugins.
4. Docs follow where they show a need or the organization's money.

## Done when

- Tests first: a need row at a narrow width has its source label on one
  line and no overlap with its button, and moves label and actions under
  the text when it must; at a wide width all sit on one line; an
  organization's full name has no nested parentheses; the admin-key need
  text no longer speaks of an estimate.
- The checks of `AGENTS.md` pass.

## Comments
