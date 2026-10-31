# 11 — Title bar

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 04, 07
Spec: [../spec.md](../spec.md)

## What

`widgets/chrome/view/action_bar.rs` as in `V4Workspace`.

- Left: left panel toggle. Center: `TitleSearch` with the active tab title (it opens the palette after ticket 17; until then it emits nothing). Right: files panel toggle, focus mode toggle, Settings, gap, fullscreen, minimize, close.
- Toggles use the panel icons from the design system Iconography section, `accent` when on; every button has a tooltip with its shortcut label from ticket 06.
- The logo is removed from the title bar. Window drag keeps working on empty title bar space.
- Chrome intents map to sidebar intents through `app/src/events/chrome.rs`, like `ToggleSidebarVisibility` today.

## Done when

- `iced_test`: each button emits its intent; tooltips show the shortcut labels; screenshots in both themes.
