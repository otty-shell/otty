# otty-ui-kit

`otty-ui-kit` is the OTTY design system for `iced`: the design tokens (colors,
sizes, gaps, corner radii, text styles and fonts) and the ready-made controls
built from them, such as buttons, fields, menus, tables and dialogs. The app
builds its screens from these pieces, so every mode looks and behaves the same.

The kit knows nothing of the app: it does not depend on the app crates and
has no translations, so every label comes from the caller.
