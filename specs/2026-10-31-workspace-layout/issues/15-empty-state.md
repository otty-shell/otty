# 15 — Empty state

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 04, 10
Spec: [../spec.md](../spec.md)

## What

With no tabs, the main area shows a kit `EmptyCard` as in `V4Empty`: logo,
"Start a session", "Open a terminal on this computer or run one of your
launches.", and two `ActionTile`s: New terminal (with its shortcut) and New
launch…. "Terminal in a folder…" and recent launches are out of scope. The
Sessions section shows the hint "No open sessions."

- New terminal → open terminal tab; New launch… → the same intent as the Launches header plus.
- Replaces the empty container at the end of `view_tab_content` in `view.rs`.

## Done when

- `iced_test`: both tiles emit the existing intents; strings in all 10 locales; screenshots in both themes (spec AC6).

## Answer

With no tabs the main area now shows the kit `EmptyCard` from `V4Empty`: the
logo, "Start a session", "Open a terminal on this computer or run one of your
launches.", and two `ActionTile`s. The Sessions section shows "No open
sessions." while it has no rows.

- `app/src/widgets/tabs/view/empty_state.rs` (new): `view(EmptyStateProps)`,
  generic over the message type. New terminal (accent plus icon, shortcut text
  from `AppShortcut::NewTerminal.label()`) and New launch… (play icon). The
  logo is `assets/logo/logo-small.png` (`icons::LOGO`), passed as an
  `image` element, so it is never recolored. Not the SVG: that file only
  wraps a large raster image, which resvg shrinks into a grainy logo.
- `app/src/view.rs`: only the fallback branch of `view_tab_content` changed.
  New terminal sends `TabsIntent::OpenTerminalTab` (shell name as title, as
  the Sessions "+" does). New launch… sends
  `QuickLaunchIntent::HeaderCreateCommand`, the same intent as the Launches
  header "+".
- `session_list.rs`: `empty_hint` shows "No open sessions." when
  `session_count == 0`.
- i18n: five keys (`empty_title`, `empty_description`, `empty_new_terminal`,
  `empty_new_launch`, `no_open_sessions`) in all 10 locales.
- Not drawn, on purpose (Out of Scope): "Terminal in a folder…", recent
  launches, Alt N hints.

### Tests (written first; first run failed to compile)

- `empty_state.rs` (`iced_test`): title and both tiles show, no "Terminal in
  a folder…" tile; New terminal sends the terminal message; New launch…
  sends the launch message.
- `session_list.rs`: hint shows with no sessions, hidden with sessions.
- Existing i18n tests cover all 10 locales for the new keys.

### Not verified

The visual look and the screenshots in both themes (the app was not run).

## Comments

Regression checklist steps for the owner:

1. Close every tab: the card is centered, logo not recolored, in dark and light themes.
2. New terminal: a terminal tab opens; the Sessions row appears and the hint disappears.
3. Close all tabs again, then New launch…: the launch editor tab opens, same as the Launches "+".
4. The shortcut text on the tile matches the Sessions "+" tooltip.
5. Sessions section: "No open sessions." with no tabs; collapsed, the count shows 0.
6. Switch to a non-English language: card text and hint are translated.
7. Narrow the window: the card stays readable (460 px wide card, centered).
