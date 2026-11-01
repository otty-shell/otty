# 11 — Title bar

Status: resolved
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

## Answer

The title bar (`widgets/chrome/view/action_bar.rs`) now follows `V4Workspace`: left panel toggle, `TitleSearch` with the active tab title, then files toggle, focus mode toggle, Settings, a gap, full screen, minimize and close. The logo is gone.

- Intents: `ChromeIntent::ToggleSidebarVisibility` is renamed `ToggleLeftPanel`; new `ToggleFocusMode`. `events/chrome.rs` maps them to `SidebarIntent::ToggleLeft` and `ToggleFocusMode` (files panel keeps `ToggleRight`).
- On/off look: the three toggles use the `accent` tone while on. `view.rs` passes `is_left_open`, `is_right_open` and `is_focus_mode` from the sidebar view model; no new state.
- Tooltips: "Name (shortcut)", with the label from `AppShortcut` (`tooltip_label`). Full screen, minimize and close have plain tooltips.
- Search button: shows `AppShortcut::CommandPalette.label()`. It has no press message until ticket 17; a click on it falls through to the window drag.
- Window drag: unchanged. The drag area sits under the bar, so empty space (and the search button for now) still drags; double click toggles full screen.
- Icons: new SVGs `panel-left`, `panel-right`, `focus-mode`, `search` in `assets/svg` and `icons.rs`; removed the unused `FILES_PANEL` and `LOGO_SMALL`. New i18n keys `window_left_panel`, `window_focus_mode`, `window_close` in all 10 locales.

### Tests

- Routing tests (reducer and `events/chrome.rs`) were written first and failed to compile before the code.
- `iced_test` view tests click each button at its position and check the intent, check the drag on empty space, the title text, and the tooltip text. They were added together with the view rewrite, not before it.
- Not done: screenshots in both themes (the agent cannot view the GUI); tooltip hover is not simulated (only the label function is tested).

## Comments

Regression checklist steps the owner must walk: 19 (Settings, now the gear on the right of the title bar), 22 (hide and show the side panel, now the left toggle instead of the logo; also try the files toggle and the focus toggle), 23 (resize, full screen and minimize from the title bar). Also check by eye: accent color on the toggles in both themes, drag by empty bar space, and the tooltip text with shortcuts.
