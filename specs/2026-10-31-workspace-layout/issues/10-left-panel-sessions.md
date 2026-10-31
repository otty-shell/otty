# 10 — Left panel: Sessions replaces the tab strip

Status: ready-for-agent
Type: task
Class: B
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 03, 07
Spec: [../spec.md](../spec.md)

## What

Open tabs become `SessionRow`s in the upper `PanelSection` "Sessions" (spec
Decision 2). `TabsState` and its reducer do not change.

- Enable the `hostname` feature of `nix` (spec Decision 10) and read the host name once at start; fall back to "This computer".
- View model: each tab gets a machine: SSH host for tabs started by an SSH launch, this computer otherwise. Find where a tab's launch is known (quick launch services / tab open effects) and pass the machine in the view model; if that needs a new field on `TabItem`, it is set only when the tab is opened. Groups keep first-open order; rows keep tab order.
- Row title = tab title; state word for Settings ("settings"), launch editor ("editor"), error tab ("failed"). Click activates (`ActivateTab`), middle click or the close button closes (`CloseTab`), any tab strip action that exists today (check `tab_bar.rs` before removing it) stays reachable from the row.
- Header action: New session (plus) opens a terminal tab; ticket 16 turns it into a split button.
- Remove `widgets/tabs/view/tab_bar.rs` and the tab strip from `view.rs`. Delete `TabStrip` from the kit if nothing else uses it, and its gallery section.

## Done when

- View model tests: grouping by machine, order, state words, active row.
- `iced_test`: row press activates, close closes.
- Tab flows of the regression checklist pass.
