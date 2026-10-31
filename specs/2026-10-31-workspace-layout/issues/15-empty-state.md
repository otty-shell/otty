# 15 — Empty state

Status: ready-for-agent
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
