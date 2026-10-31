# 23 — Dialog component and scrim token

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 22
Spec: [../spec.md](../spec.md)

## What

Move the modal dialog of the SSH secret prompt into the kit. The backdrop
color gets its own token instead of reusing the shadow color.

## Details

- `crates/ui/kit/src/palette.rs`: new token `scrim` — "Dims the window behind a modal dialog." Values equal today's backdrop, so nothing changes on screen: OTTY Dark `rgba(0, 0, 0, 0.5)`, OTTY Light `rgba(17, 20, 28, 0.16)`. Getter `scrim()`.
- `crates/ui/kit/src/size.rs`: `DIALOG: f32 = 420.0` — "Width of a modal dialog."
- `crates/ui/kit/src/components/dialog.rs`: `Dialog::new(title, palette)`, `.body(element)`, `.action(element)` (repeatable; actions sit right-aligned in one row, `SPACE_3` apart, in the order added), `.into()` → `opaque` full-window container with `scrim` background, centered card: width `size::DIALOG`, padding `SPACE_5`, `surface_overlay` background, `RADIUS_LG`, `shadow_popover`, title in `typography::HEADING` `text_strong`, parts `SPACE_5` apart. Same look as `secret_prompt.rs` today.
- `app/src/widgets/quick_launch/view/secret_prompt.rs` uses `Dialog`; remove `DIALOG_WIDTH` and the hand-made backdrop.
- Add a dialog example to the gallery (shown on a button press, closed by its Cancel action).

## Tests first

- In the palette tests: the `scrim` value of each theme is in the expected token table.
- No other tests (styling).
