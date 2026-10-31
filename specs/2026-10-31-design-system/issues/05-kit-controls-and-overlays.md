# 05 — Kit components: controls and overlays

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 02
Spec: [../spec.md](../spec.md)

## What

The first set of shared components in `otty-ui-kit`, each one file, each
styled only with `UiPalette` and kit tokens, each following its
`components/<Name>/README.md` in the design system.

## Components

| Component | Replaces | Notes |
|---|---|---|
| `Button` | `button_style` in `settings_form.rs`, `wizard_form.rs`, `secret_prompt.rs` | Kinds `primary`, `secondary`, `ghost`, `danger`; sizes 34 (forms) and 28 (`sm`); optional shortcut shown as `Kbd` |
| `IconButton` | `app/src/components/primitive/icon_button.rs` | Required label used as tooltip; idle `text-muted`, hover `surface-raised`; sizes 28 and 24 |
| `Kbd` | — | Shortcut hint, `kbd` text style, `radius-xs`, `surface-raised` |
| `Menu` + `MenuItem` | `menu_item.rs`, `menu_panel_style` in `app/src/style.rs` | `surface-overlay`, `shadow-popover`, `radius-md`, item height `row`, hovered item `surface-hover`, shortcut on the right, separators |
| `Switch` | `checkbox_style` in `settings_form.rs` | On/off with the label |
| `Select` | `pick_list_style` / `pick_list_menu_style` | Its dropdown uses the Menu look |
| `Segmented` | the button rows for Type and After it exits in `wizard_form.rs` | Selected segment `surface-hover` inside a `radius-md` frame |

Focus for every interactive component: 2px `focus` outline with 1px offset,
as far as iced 0.14 allows; note any gap in the ticket's Comments.

- Add `examples/gallery.rs`: each component in every state, and a switch between OTTY Dark and OTTY Light (AC5).
- No unit tests for pure view code (AGENTS.md). If a component holds logic (for example, mapping a shortcut to `Ctrl Shift C` text), test that function.

## Done when

- The gallery shows all components of this ticket in both themes; they match the design system previews.

## Comments

Focus ring gap (iced 0.14): `button`, `pick_list` and `container` have no
focus state and no way to draw an outline (`button::Status` is only Active,
Hovered, Pressed, Disabled; buttons are not reachable with Tab). The 2px
`focus` outline with 1px offset is therefore not drawn on any component of
this ticket. Closing it needs a custom widget (or a wrapper that tracks focus
through `iced::advanced`), which is out of scope here.

Other gaps:
- IconButton: the icon tint changes to `text-strong` only while the pointer is
  over the 15px icon itself, not the whole 28px button (the `svg` widget
  reports hover for its own bounds). The `surface-raised` background follows
  the whole button.
- Kbd uses the regular mono font; the bundled mono has no 500 weight.
- Select: item height follows the 34px field (iced's pick list menu reuses the
  field padding), not the 30px `row` height. Dot and note per option, and
  group headings, are not supported by `pick_list`.
- Switch is built from a button and containers (34x20 track, 14px knob) since
  iced's `toggler` is always twice as wide as it is high. The knob moves
  without animation.
