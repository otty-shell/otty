# 12 — Restyle settings screen

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 03, 05, 06
Spec: [../spec.md](../spec.md)

## Details

- `app/src/widgets/settings/view/settings_form.rs`: rows become `SettingRow`; language `Select`; theme `Segmented` with OTTY Dark and OTTY Light (two options, so Segmented per the design system); equalize panes `Switch`; shell and editor `TextField`. The palette color fields are already gone (ticket 03).
- Section navigation rows use `TreeRow`.
- Buttons use kit `Button`. Save stays (instant apply is out of scope).
- Remove the six local `*_style` functions and the hard-coded `Color::from_rgba(1.0, 1.0, 1.0, 0.2)` (`settings_form.rs:338`).
- Page layout: title `title`, sections `space-6` apart, top padding `space-7`.

## Done when

- Screenshots of each settings section in both themes.
