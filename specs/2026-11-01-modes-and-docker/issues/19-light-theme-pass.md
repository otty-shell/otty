# 19 — OTTY Light pass and screenshots

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 10–18
Spec: [../spec.md](../spec.md)

## What

Check every new screen in OTTY Light and fix contrast or token issues in the
kit. Take the screenshots of spec AC12 in both themes and attach them to the
PR next to the `Docker mode` canvas boards.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Screenshots of: Terminal mode with the switcher, the open All modes menu, each Docker table, each inspector tab, the Run form, each no-engine case — Dark and Light.

## Answer

### Fixed

- Neutral badge dot: it is now `text-muted`, the color of its word
  (`StatusTone::dot_color`). `text-faint` was 2.99:1 on `surface-selected` in
  Light and 3.00:1 in Dark, and 2.67:1 on `surface-hover` in Dark. No surface
  changed. A new test in `palette.rs` checks the neutral dot at 3:1 on panel,
  canvas, raised, hover and selected, in both themes.
- Overview tab: the scrollable uses the thin kit scrollbar with 12 px of right
  padding, like the Inspect tree (test: values stay clear of the bar at 260 px).
- Status bar: while the Run form is open it says "Run container" next to the
  engine instead of the table counts (`status.rs`).
- "Looked for" line: with no probes it reads "OTTY looked for docker and podman
  on PATH and in the usual install folders." (new key `DockerLookedForCommands`
  in the ten locales; `Key::ALL` is 408).
- Found in the screenshots: the Logs hint ("Last 500 lines, new lines appear as
  they come.") sat in a row with the "N new lines" count and the Follow switch
  and wrapped to one word per line in the 260 px panel. The hint now has its own
  line and the count and switch share the row below it.

### Screenshots

Taken with a throwaway `iced_test` (not committed) that drove the real root
view with fixture data at 1440x900 (narrow: 900x900), software renderer, in
OTTY Dark and OTTY Light. Folder:
`/tmp/claude-1000/-home-kemokempo-projects-main-ottyrun-otty/01492649-6afb-4868-b239-d18c47b01579/scratchpad/shots19/`.
Files, each as `-dark.png` and `-light.png`: `01-terminal-switcher`,
`02-all-modes-menu`, `03-containers`, `04-images`, `05-volumes`,
`06-networks`, `07-compose-filtered`, `08-row-menu`, `09-inspector-overview`,
`10-inspector-logs-follow-off`, `11-inspector-inspect`, `12-narrow-overview`,
`13-run-form-ready`, `14-run-form-errors`, `15-run-form-pulling`,
`16-no-engine-not-installed`, `17-no-engine-docker-off`,
`18-no-engine-podman-off`.

### Not done or left

- The no-access card: a `Discovery` for it cannot be built outside the docker
  crate (`SocketCandidate::new` is private), so it has no screenshot.
- Narrow window: the left panel cuts "Containers" to "Co" next to "2 running"
  (`TreeRow` has no ellipsis; same as ticket 09). Table cells are cut with an
  ellipsis ("otty-fx-runni...", "1..."), which is the designed behavior.
- Light looked right elsewhere: row, hover and selected surfaces, status words,
  links, badges, form errors and cards all read clearly.
- Not run against a live engine; see `regression-checklist.md`.
- The overview scrollbar test was written with the fix, not seen failing first.

### Checklist

`specs/2026-11-01-modes-and-docker/regression-checklist.md` (23 steps).

Coverage (lines): 87.53% on `2d62317` -> 87.54%.
