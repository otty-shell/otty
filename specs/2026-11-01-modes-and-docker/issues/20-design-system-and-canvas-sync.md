# 20 — Update the design system and the canvas to what was built

Status: resolved
Type: task
Class: —
Model: Opus 5.5 (claude-opus-5-5), owner review
Blocked by: 19
Spec: [../spec.md](../spec.md)

## What

- Add the new kit components to the design system artifact (https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3).
- Update the `Docker mode` boards on the canvas wherever the built app differs.
- Mark `V4ModeSwitch` and `V4ModesMany` as superseded, or change their Ctrl 1–5 labels to the Decision 6 table.
- The owner reviews before publishing.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Design system and canvas match the app; the owner has approved them.

## Answer

The owner approved publishing on 2026-11-03. Both are published:

- Design system: https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3, version 14 (id `1793707694-285e`).
- Canvas: https://claude.ai/artifact/5QkFQQUDrqMuqVSFyBZCh6, version 44 (id `1793707933-4418`), with design system version 14 installed.

### Canvas, first sync

Published to https://claude.ai/artifact/5QkFQQUDrqMuqVSFyBZCh6 as version 40 (id `1793620693-0e3a`). The owner had made no hand edits since the last version (the file hashes matched).

- `DockerMode`, and through it `DockerModesMenu`, `DockerImages`, `DockerVolumes`, `DockerNetworks`:
  - No mode name, "this computer" or engine chip. The tree has Containers, Images, Volumes and Networks only.
  - The header has two rows: the title with the compose chip, then the filter and icon buttons.
  - The tables sort as in the app, with the arrow after the title and running containers first. The Containers table has Compose and Ports links (`8080:80/tcp`). Every row has a "⋯" menu.
  - The inspector has icon actions with Remove in red, Overview links, keys above values, Logs with Follow and "N new lines", and Inspect as a tree with the two switches and Copy JSON.
  - Pressing a row opens a closed right panel.
  - A new `inspector` tweak (230, 320, 150) shows two columns and tabs as a select.
- `DockerRun`: `:` between ports, the SELinux switch on a folder volume, and Restart starting at No. A new `selinux` tweak hides the switch.
- `DockerNoEngine`, and through it `DockerEngineStopped`, `DockerPodmanOff`, `DockerNoAccess`: no mode name, and the "looked for" line as built.
- `V4ModeSwitch`, `V4ModesMany`: titles now say they are superseded and that their Ctrl 1–5 labels are old.

Not drawn or not matched:

- The boards are 1440 wide, so they do not show the toolbar collapsing or the narrow table and tree of ticket 36.
- Port links show their URL as a tooltip and open nothing. Prune, Refresh and Copy JSON do nothing.
- Prune dialogs are not drawn.
- The boards keep their sample containers, and two engines, instead of the screenshot fixtures.
- The result was not rendered, following the artifact's rule.

### Design system

The earlier draft was lost when the scratchpad was cleaned, so the update was rebuilt from the kit code and tickets 24 to 51. Version 13 (`1793707544-3c99`) holds the update; version 14 only makes `TagRow` chips mono, as in the kit.

- New cards: `ModeSwitcher`, `ModeMenu`, `StatusBadge`, `Notice`, `KeyValueList`, `InspectorHeader`, `InspectorTabs`, `LogView`, `CommandBox`, `ProgressBar`, `MappingField`, `Toolbar`, `DataTree`, `TagSearchField`, `TagRow`. `Split` had no card, so it is new too.
- Extended cards: `DataTable`, `IconButton`, `Inspector`, `Chip`, `ActionTile`, `KeyValueField`, `TreeRow`, `SessionGroup`, `SessionRow`, `PanelSection`, `Switch`, `Button`, and the danger item of the `Menu` component.
- `RailButton` and the `rail` and `rail-button` tokens stay, marked "not in the app", because older canvas screens use them.
- `README.md` has Modes, Docker mode, Formats, Wrapping and Tag search sections, the Decision 6 shortcuts, and the link and danger colors.
- `tokens.json`: `surface-danger-hover` (dark `#4a2a33`, light `#f0bcc2`, as in `palette.rs`), `table-header`, `table-row`, `mode-tab-width`, `mode-tab-height`, `mode-icon-tile`, `mode-menu`, `key-value-key`, `key-value-stack-below`, `inspector-tab`; `split-grab` is now 4.
- 17 icons uploaded to the Icons group: `mode-terminal`, `mode-docker`, `more`, `plus`, `pull`, `prune`, `refresh`, `start`, `stop`, `restart`, `pause`, `resume`, `shell`, `trash`, `copy`, `external-link`, `notice-error`.

### Canvas, second sync

The new design system copy was installed first, as its own change (versions 42 and 43). Version 44 then changed `DockerMode`, `project/docker-mode.css` and `project/otty-theme.css`; the other Docker boards import `DockerMode` and follow it.

- Tag search: title row; search field and icon buttons; the tags row with crosses and "Clear all", only with tags; then the table. No compose chip: the Compose link sets a `Compose` tag. Keys read as column headers (`Status:Running`).
- New tweaks on `DockerMode`: `search` opens the list with the typed text as the first row (`typed`, `values`), and `tags` shows the tags row (`some`, `long`). `long` adds one sample container, so its long tag wraps and rows stay in the table.
- Narrow inspector: Inspect switches stacked with the copy icon on its own row at the left under them, one-line labels. Everything fits at the `320` width of the `inspector` tweak, on one row.
- Inspect tree links by the ticket 37 rules, the copy icon, and `surface-danger-hover` on danger controls.

Not matched:

- The cards and the board match suggestions by substring; the app ranks them with a fuzzy match (ticket 46).
- `Toolbar` and `InspectorTabs` in the bundle take a prop for the narrow look; the kit measures the width. `KeyValueList` and `DataTree` stack by width in the cards too.
- The boards draw with the design system's stylesheet classes and do not mount its components, as before.
- The stylesheet rules for one-line chips and for notes that give way also reach older canvas screens, which link the same copy.
- The icons `folder.svg` and `folder-opened.svg` in the Icons group are no longer in `assets/svg/`. They stay. Icons older than this feature that the group lacks (`focus-mode`, `panel-left`, `panel-right`, `search`, `folder-closed`) were not added.
- Neither result was rendered, following the artifact's rule.

### Found on the way

The Prune dialog of Volumes always says "They and all data in them are deleted, named ones too" (`tpl_docker_prune_volumes_text`), even with "Also remove named volumes" off, when named volumes are kept. Not changed here; it needs its own fix in all ten locales.
