# 14 — Focus mode note

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 04, 07
Spec: [../spec.md](../spec.md)

## What

In focus mode a kit `FloatingNote` sits at the top center of the main area:
"Focus mode · Panels are hidden · <shortcut> to show them", as in `V4Focus`.
It is an overlay layer in `view.rs` and does not take mouse input from the
terminal outside its own bounds.

## Done when

- Visible only in focus mode; strings in all 10 locales; screenshot in both themes (spec AC5).
