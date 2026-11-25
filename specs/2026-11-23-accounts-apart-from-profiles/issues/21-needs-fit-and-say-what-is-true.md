# 21 A need fits any width and says what is true now

Status: done
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

2026-11-25, done. The need row (`view/needs.rs`, used by the account
and profile pages, All accounts, All profiles and Found by OTTY) lays out
for the width it gets: the heading and the line take the width and wrap,
then the plugin's label on one line and the buttons whole at their
right. When the text would keep less than 320 px beside them, the label
and the buttons go to their own line under the text, at the right, and
there a label that still does not fit ends with "…" and its tooltip
shows it whole. The kit `Button` got `width()`, the width it takes, so
the row can tell what fits. Tests pin the geometry at 900, 360 and
200 px. Pictures before and after at 900, 620 and 360 px were checked.

Names: the Anthropic plugin sets off each part of an organization's name
in parentheses with a comma, "Example Org (Claude API Platform)" becomes
"Example Org, Claude API Platform", for a Console organization and after
the email of a subscription. Every word stays, so two organizations that
differ only in that part stay apart, and the plugin needs no list of the
kinds Anthropic writes there. The OpenAI plugin builds no name from a
provider's text (an email, "ChatGPT ••••…" or "key ••••…"), so it needed
no change.

Words: the admin-key need of the Anthropic, OpenAI and template plugins
now says "OTTY shows no money for this organization until it has an
admin key, which reads the real bill. No agent keeps one." in all ten
languages. No locale of OTTY spoke of the estimate. Anthropic 0.2.17,
OpenAI 0.2.16 and the template 0.1.10 are rebuilt into `assets/plugins`.
The plugin pages, Overview (Needs, The money) and the Anthropic README
follow.
