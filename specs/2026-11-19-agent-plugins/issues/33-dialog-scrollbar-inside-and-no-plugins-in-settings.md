# 33 A dialog scrolls inside its card; Settings has no Plugins section

Status: done
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

- 2026-11-22, issue 33, done. The kit's `Dialog` lays its card out with
  `DialogColumn` (`crates/ui/kit/src/components/dialog_column.rs`): the
  title and the actions are measured first, the body gets the height
  they leave and scrolls up and down in it, with the kit's thin bar
  beside the body, `space-2` from it, inside the card; the bar takes its
  room only while it shows. The area around the card no longer scrolls:
  the card keeps 48 px from each edge of the window and, in a window
  narrower than it, shrinks to it (the simpler choice; below about
  300 px a row of two buttons may not fit). The log of a plugin was the
  only dialog with a `scrollable` of its own; it is now plain text in
  the body. The tables of the found report are as tall as their rows and
  only scroll sideways, so nothing scrolls twice. Settings has no Plugins
  section: `plugins_section`, `SettingsIntent::OpenPlugins`,
  `SettingsOutcome::OpenPlugins`, their dispatch and guard, and the keys
  `SectionPlugins`, `FieldInstalledPlugins`, `DescPlugins` in all ten
  locales are gone; `ButtonManagePlugins` stays for the "No agent plugin"
  card of Agents mode. Plugins, Write a Plugin and the spec say Plugins
  mode. Tests: a long body keeps the title and the action in the window
  and scrolls inside the card, a short body shows no bar, only the body
  scrolls, a narrow window shrinks the card (kit); Settings has no
  Plugins section, its fields scroll while Save stays (settings); a long
  plugin log scrolls once (plugins); the app's Settings scrolling tests
  now wheel its body. Follow-up of issue 32: the official plugins give
  an install command, `sh -c` with the vendors' commands confirmed on
  2026-11-22: OpenCode 0.1.2 `curl -fsSL https://opencode.ai/v2/install
  | bash`, Anthropic 0.2.3 `curl -fsSL https://claude.ai/install.sh |
  bash` with the page `https://code.claude.com/docs/en/setup`, OpenAI
  0.2.3 `curl -fsSL https://chatgpt.com/codex/install.sh | sh` (still no
  program place: its README names no folder); rebuilt with
  `plugins/build.sh`; plugin pages and READMEs follow.
