# 22 — FormPage component

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 21
Spec: [../spec.md](../spec.md)

## What

Settings and the quick launch editor each build a scrolling form page with a
720 px max width, and with different paddings. Make one component.

## Details

- `crates/ui/kit/src/size.rs`: `FORM: f32 = 720.0` — "Maximum width of a form page."
- `crates/ui/kit/src/components/form_page.rs`: `FormPage::new(palette)` with `.section(element)` (repeatable, `spacing::SPACE_6` apart), `.footer(element)` (optional); renders a vertical scrollable (kit `scrollbar` style and direction) filling the area, content `max_width(size::FORM)`, padding `SPACE_7` top, sides and bottom (design system: form page top padding `space-7`).
- Use it in `app/src/widgets/settings/view/settings_form.rs` and `app/src/widgets/quick_launch/view/wizard_form.rs`; remove both `FORM_MAX_WIDTH`. The wizard's form-level error text stays a section above the footer.
- Add it to the gallery (`settings` part).
- Visual change: the quick launch editor gets the settings paddings (`SPACE_7`), its content moves a few pixels.

## Answer

- Added `size::FORM` and `components/form_page.rs` (`FormPage` with `.section` and `.footer`), plus a FormPage example in the gallery `settings` part.
- Settings and the quick launch editor use it; both `FORM_MAX_WIDTH` constants are gone. The wizard's form-level error is a section above the footer.
- The settings page title is a row with the Reset and Save buttons, not plain text, so it is passed with `.section(..)` and not `.title(..)`.
- The quick launch editor now has `SPACE_7` padding; the old extra right padding for the scrollbar is dropped.
- `.title()` was dropped in review: no caller used it (the settings title row carries buttons and is passed as the first section).
