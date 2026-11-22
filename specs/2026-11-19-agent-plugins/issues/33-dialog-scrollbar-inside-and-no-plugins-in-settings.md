# 33 A dialog scrolls inside its card; Settings has no Plugins section

Status: open
Blocked by: 32

## Why

Owner, 2026-11-22:

- In Settings, and maybe in every dialog, the scrollbar is far outside
  the dialog, which looks odd. Cause: `otty-ui-kit`'s `Dialog`
  (`crates/ui/kit/src/components/dialog.rs`) wraps the whole card, with
  `SHADOW_ROOM` around it, in one `scrollable` that fills the window, so
  the bar is drawn at the edge of that area, not at the card.
- Settings still has a Plugins section ("Installed plugins", **Manage
  plugins…**). Since issue 30 Plugins is a mode of its own; the section is
  not needed.

## What to build

1. **The body scrolls inside the card.** A dialog's title (with its
   cross) and its row of actions stay in place; only the body scrolls,
   with the bar inside the card along its right edge, in the kit's
   scrollbar style. The card is never taller than the window minus a
   margin; a body that fits shows no bar. A window narrower than the card
   still works (the card may shrink to the window, or scroll sideways as
   a last resort — choose the simpler that keeps the bar on the card).
   This is one change in the kit's `Dialog`, so every dialog gets it;
   check the dialogs that hold their own `scrollable` today (grep
   `scrollable` in the app's dialog views) so none scrolls twice.
2. **No Plugins section in Settings.** Remove `plugins_section`,
   `SettingsIntent::OpenPlugins` / `SettingsOutcome::OpenPlugins` and their
   dispatch, and the words only it uses (`SectionPlugins`,
   `FieldInstalledPlugins`, `DescPlugins`, and `ButtonManagePlugins` if
   nothing else uses it — Agents mode's "No agent plugin" card does; keep
   the key while it does) in all ten locales. Docs and spec that say
   "Settings → Manage plugins…" say Plugins mode.

## Done when

- Tests first: a dialog whose body is taller than the window keeps its
  title and actions in the window and scrolls the body inside the card
  (the bar's bounds are inside the card's); a short body shows no bar;
  Settings has no Plugins section.
- The checks of `AGENTS.md` pass.

## Comments
