# Design System Adoption: Tokens, UI Kit and Restyled Screens

Created: 2026-10-31
Author: holland.deception@gmail.com
Agent: Claude Code
Status: IN PROGRESS
Approved: Yes
Worktree: No
Type: Build

## Summary

**Goal:** Make the current OTTY window look and behave like the OTTY design
system: every color, size, radius and text style in the app comes from one set
of design tokens, every widget is built from shared components in a new
`otty-ui-kit` crate, and the app has exactly two themes, OTTY Dark and OTTY
Light, picked in Settings.

**Reference:** the OTTY design system artifact
(https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3). Its `README.md` (including
the Themes section), `tokens.json` and `components/*/README.md` are the source
of truth for values and usage rules. Where this spec and the design system
disagree, the Decisions section below wins and the design system is updated
afterwards (ticket 15).

This spec covers restyling what the app already has. New features that the
design system draws (targets, agents, status bar, right panel, bottom dock,
command palette, Kubernetes) are listed under Follow-up specs.

### Current state (research, 2026-10-31)

| Fact | Location |
|---|---|
| Widgets read raw ANSI colors (`overlay`, `dim_black`, `dim_blue`, …) and tune alpha by hand; there are no role-based colors | `app/src/style.rs:36-45`, `app/src/widgets/**/view/*.rs` |
| 17 `*_style` functions spread across 8 files, 6 of them in the settings form | `app/src/widgets/settings/view/settings_form.rs:437-557` |
| Size constants are local to each widget and differ from the design system | `app/src/layout.rs`, `app/src/widgets/chrome/view/action_bar.rs:14-21`, `app/src/widgets/tabs/view/tab_bar.rs:13-22`, `app/src/widgets/explorer/view/sidebar_tree.rs:15-27` |
| UI font is `Font::default()` at 14px; terminal font is JetBrains Mono Nerd **Bold** at 14px | `app/src/fonts.rs` |
| Only `JetBrainsMonoNerdFontMono-Bold.ttf` and a 3270 font are bundled | `assets/fonts/` |
| Shared components: `icon_button`, `menu_item`, `resize_grips`, `sidebar_workspace_panel` | `app/src/components/primitive/` |
| Four dark presets (OTTY Dark, One Dark, Solarized Dark, Dracula) plus an editor for all 29 palette colors | `app/src/widgets/settings/types.rs:612-679`, `settings_form.rs:293-318` |
| Settings store the whole palette as 29 hex strings under `theme.palette`, not a theme name | `app/src/widgets/settings/types.rs:112, 239-242` |

### Gap in sizes

| Element | Now | Design system |
|---|---|---|
| Title bar | 30 | `titlebar` 38 |
| Tab strip | 24 | `tab` 34 |
| Widget rail | 52 | `rail` 48 |
| Tree rows | 24, indent 14, text 12 | `row` 30, indent 18, `body` 13 |
| Menu radius | 4 | `radius-md` 8 |
| UI text | default font, 14 | Noto Sans, `body` 13 |
| Terminal text | Bold, 14 | Regular 400, `term` 13 |

## Decisions

1. **Language.** The design system rule "the UI is in English" is read as a
   writing-style rule for English source strings (sentence case, verbs on
   buttons, Linux shortcut notation). All 10 locales in `app/src/i18n/locales`
   stay.
2. **Two themes, nothing else.** The app has OTTY Dark (default) and OTTY
   Light. One Dark, Solarized Dark, Dracula and the palette color editor are
   removed. Each theme is a fixed set of values: the terminal palette and the
   UI palette (`UiPalette`). Nothing is computed from colors at run time.
3. **Settings store the theme name.** The settings file keeps
   `theme.name` = `"otty-dark"` or `"otty-light"`. A missing or unknown name,
   and an old file that has only `theme.palette`, load as OTTY Dark. The old
   `theme.palette` key is ignored and not written back.
4. **Separate crate.** Tokens and components live in a new workspace crate
   `otty-ui-kit` at `crates/ui/kit`, following `crates/ui/tree`. Justification
   under AGENTS.md: every widget in `app` consumes it, it has a visual gallery
   example of its own, and it keeps styling out of widget business code.
5. **Fonts are approved.** The owner approved adding Noto Sans and JetBrains
   Mono Regular to `assets/fonts` (2026-10-31).
6. **OTTY Light terminal palette.** The design system fixes 9 of the 29 colors;
   the other 20 are set in ticket 03 (table there) and go back into the design
   system in ticket 15.

## Acceptance Criteria

- [ ] AC1: `crates/ui/kit` is a workspace member named `otty-ui-kit`; `app` depends on it.
- [ ] AC2: For OTTY Dark and OTTY Light, every color token of `UiPalette` equals the hex value in the design system's `tokens.json` for that theme (table-driven unit test).
- [ ] AC3: For each theme, the tokens the design system defines as ANSI aliases (`surface-canvas`, `text-body`, `accent`, `focus`, `status-*`, `target-*`) equal the matching colors of that theme's terminal palette (unit test).
- [ ] AC4: In both themes, `text-strong`, `text-body` and `text-muted` reach 4.5:1 on `surface-chrome`, `surface-panel`, `surface-canvas` and `surface-overlay`; `focus` reaches 3:1 on every surface; `on-accent` reaches 4.5:1 on `accent` (unit test computing WCAG contrast).
- [ ] AC5: Settings loading: `theme.name` `"otty-light"` loads OTTY Light; a missing name, an unknown name and an old file with only `theme.palette` load OTTY Dark; saving writes `theme.name` and no `theme.palette` (unit tests).
- [ ] AC6: `cargo run -p otty-ui-kit --example gallery` opens a window with every kit component from tickets 05 and 06 in all its states, with a switch between OTTY Dark and OTTY Light.
- [ ] AC7: No file under `app/src/widgets` or `app/src/components` calls `iced_palette()` or reads `IcedColorPalette` fields; colors come only from `UiPalette` (checked with `grep`).
- [ ] AC8: Heights, paddings, gaps, radii and font sizes in `app/src/widgets` and `app/src/components` come from `otty-ui-kit` tokens. Local `const …: f32` remain only for values with no token (dialog and menu widths, resize grip thickness), and each such constant has a doc comment.
- [ ] AC9: The title bar is 38px, the tab strip 34px, the rail 48px, tree rows 30px with 18px indent (measured in the running app or asserted on layout constants).
- [ ] AC10: UI text renders in Noto Sans, terminal text in JetBrains Mono Nerd Font Mono Regular 400 at 13px.
- [ ] AC11: Settings › Appearance shows exactly two themes, OTTY Dark and OTTY Light, and no palette color fields; switching restyles the whole window and the terminal without a restart.
- [ ] AC12: Screenshots of the main window, the quick launch editor, the settings screen and an open context menu, each in OTTY Dark and OTTY Light, are attached to the PR next to the design system's Themes card and component previews.
- [ ] AC13: `cargo +nightly fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` and `cargo deny check` pass; line coverage from `cargo llvm-cov --workspace --all-features` is strictly greater than on the merge base.

## Out of Scope

- **Custom themes** and computing UI colors from an arbitrary terminal palette. The design system README still describes this (principle "Colors come from the active theme", "Custom themes follow the same split … `UiPalette::derive`"); ticket 15 marks it as future work there.
- **Settings that apply right away without a Save button.** The design system asks for it (SettingRow, Switch), but it changes behavior and storage, not looks. Separate spec.
- **CommandBlock styling of failed commands.** The terminal widget does not track an exit code per block yet (`crates/ui/terminal/src/term.rs:77` has only the session exit status).
- **Components with no current consumer:** Banner, TargetChip, StatusBar, CommandPalette, DataTable, Inspector. They arrive with the features that need them.
- New window slots and features, listed below.

### Follow-up specs

| Spec | Brings |
|---|---|
| Layout slots | Status bar, right panel, bottom dock |
| Targets | Target model (this computer, SSH, cluster), TargetChip, `target-*` dots, protected targets with Banner |
| Command palette | Ctrl K palette over sessions, launches, targets and actions |
| Agent sessions | Agent states (working, needs you, done, failed) in SessionRow and TabStrip |
| Side session list | Sessions as SessionRows grouped by target, Settings › Layout switch |
| Kubernetes | Clusters tree, DataTable of pods, Inspector |
| Custom themes | User themes and UI colors computed from them |

## Known Token Issues

Found while checking contrast (WCAG 2.1 relative luminance). Ticket 02
recorded them in tests as known exceptions; on 2026-10-31 the owner decided to
fix them, and ticket 16 changes the tokens so every pair below reaches 4.5:1.
Ticket 15 then publishes the new values to the design system.

| Pair | Dark | Light |
|---|---|---|
| `text-muted` on `surface-hover` (shortcut in a hovered menu item) | 4.16 | 4.47 |
| `status-ok` / `status-error` on `surface-chrome` | passes | 4.39 / 4.39 |
| `status-ok` / `status-error` on `surface-raised` | passes | 4.23 / 4.23 |

The design system's Themes section says the light status colors pass 4.5:1 on
`surface-chrome`; the numbers above show they do not.

## Progress Tracking

- [x] 01 — Create `otty-ui-kit` crate with size, spacing, radius and type tokens · Opus
- [x] 02 — `UiPalette` for OTTY Dark and OTTY Light, contrast tests · Opus
- [x] 03 — Two themes: add OTTY Light, remove other presets and the palette editor, store the theme name · Opus
- [x] 04 — Bundle fonts and switch UI and terminal text styles · Sonnet
- [x] 05 — Kit components: controls and overlays · Sonnet
- [x] 06 — Kit components: rows, tabs and form fields · Sonnet
- [x] 07 — Restyle title bar and pane dividers · Sonnet
- [x] 08 — Restyle widget rail · Sonnet
- [x] 09 — Restyle tab strip · Sonnet
- [x] 10 — Restyle sidebar trees · Sonnet
- [x] 11 — Restyle menus and dropdowns · Sonnet
- [x] 12 — Restyle settings screen · Sonnet
- [x] 13 — Restyle quick launch editor, secret prompt and error tab · Sonnet
- [x] 14 — Remove old styling code · Opus
- [x] 16 — Fix the known contrast issues · Sonnet
- [x] 17 — Status colors pass 4.5:1 on every surface they can sit on · Sonnet
- [x] 18 — Divider component and the 1px line token · Sonnet
- [x] 19 — One split style for the sidebar and terminal panes · Sonnet
- [x] 20 — Surface container · Sonnet
- [x] 21 — Bold mono font in the kit and small token fixes · Sonnet
- [x] 22 — FormPage component · Sonnet
- [x] 23 — Dialog component and scrim token · Sonnet
- [ ] 24 — Update the design system with the new kit parts · Opus, owner review
- [x] 15 — Update the design system to match the code · owner

Tickets live in `issues/`. Each ticket names the model that runs it on its `Model:` line; a subagent started for a ticket is launched with that model. Order: 01 → (02, 04) → (03, 05, 06) → 07–13 in parallel → 14 → 16 → 15 → 17 → 18 → … → 24 (one at a time; 18–23 touch the same view files).

## Round Log

- Round 1 (2026-10-31, owner review): no dark/light detection, only two fixed themes; One Dark, Solarized Dark, Dracula and the palette editor removed; fonts approved; OTTY Light terminal colors completed in ticket 03 because the updated design system still fixes only 9 of 29.
- Round 2 (2026-10-31, owner review): spec approved; OTTY Light terminal colors in ticket 03 approved; implementation runs ticket by ticket, one subagent per ticket with the model from its `Model:` line, one commit per ticket.
- Ticket 02 (2026-10-31): `text-muted` on `surface-selected` (4.68 / 4.54) passes 4.5:1 and was wrongly listed as a known issue; removed from the table and tested as a passing pair.
- Round 3 (2026-10-31, owner review of the ticket 15 draft): fix the known contrast issues before publishing (ticket 16); publish the design system after that.
- Round 4 (2026-10-31, owner): fix OTTY Light `status-warn` contrast (ticket 17); status colors never sit on `surface-hover`, so OTTY Dark `status-error` on it stays; `target-*` dots need 3:1.
- Round 5 (2026-10-31, owner): after a review of what is left in the app, move dividers, split style, surface containers, the bold mono font, small tokens, the form page and the modal dialog into the kit (tickets 18–23); split grab zone 8 px for all splits; the dialog backdrop gets a `scrim` token with today's values.
