# Command Blocks: Token Colors, Exit Codes and the CommandBlock Look

Created: 2026-10-31
Author: holland.deception@gmail.com
Agent: Claude Code
Status: DRAFT
Approved: No
Worktree: No
Type: Build

## Summary

**Goal:** Command blocks in the terminal look and behave like the design
system's `CommandBlock`: their colors come from the theme's tokens, each
finished command shows whether it succeeded, its exit code and how long it
ran, and a failed command stands out.

**Background:** The terminal splits its history into blocks: one block per
command with its output, plus the prompt block at the bottom. The design
system spec (`specs/2026-10-31-design-system`) left block styling out of scope
because the UI had no exit code per block. Research below shows the data path
exists; only the shells do not send the exit code yet.

**Reference:** design system `components/CommandBlock` (README and preview),
tokens `surface-selected`, `surface-header`, `surface-error`, `line`,
`line-width`, `status-ok`, `status-error`, `text-muted`, `shadow-sticky`.

### Current state (research, 2026-10-31)

| Fact | Location |
|---|---|
| Block colors are computed in the terminal crate, not taken from tokens: the selected block fill and the line between blocks are `bright_foreground` at 10% opacity | `crates/ui/terminal/src/view.rs:584-587`; `block_highlight` set from `bright_foreground` in `app/src/theme.rs:148` |
| The hovered action button background is drawn with alpha 0, so it is invisible | `crates/ui/terminal/src/view.rs:740-741` |
| The block protocol already carries `cmd`, `cwd`, `started_at`, `finished_at` and `exit_code`, and has an `exit` phase | `crates/escape/src/dcs/block.rs` (`BlockMeta`, `BlockPhase::Exit`) |
| The shell hooks send only `preexec` (cmd, cwd, time) and `precmd` (cwd, time); no `exit` event, so `exit_code` is always empty | `assets/shell-integrations/otty.zsh:90-117`, `otty.bash:260-276` |
| On an `exit` event the surface replaces the block's whole metadata, so an `exit` payload without `cmd`/`cwd` would wipe them | `crates/surface/src/block.rs:519` (`end_block_by_id`) |
| Times are whole seconds (`EPOCHSECONDS`, `date +%s`); the design shows tenths (`0.4 s`) | same shell files |
| A block has no header row; the shell's own prompt line is the first line of a block. Block actions are an inline button on the right edge plus the pane context menu (Copy prompt, Copy command, …) | `crates/ui/terminal/src/block_controls.rs`, `app/src/widgets/terminal_workspace/view/pane_context_menu.rs` |
| The terminal widget can leave block chrome to the host (`BlockUiMode::ExternalOverlay`) | `crates/ui/terminal/src/term.rs:54` |
| Shell hooks are installed only for local zsh and bash sessions; SSH sessions have no hooks | `app/src/widgets/terminal_workspace/services.rs` |

## Questions for the owner

1. **Header or badge.** The design system draws a header row above each block
   (status mark, command, cwd, duration). Our blocks start with the shell's own
   prompt line, which already shows the command and usually the cwd. Options:
   - **(Recommended) Badge:** a small meta badge right-aligned on the block's
     first line: `✓ 0.4 s` or `✕ exit 1 · 8.4 s`. No change to terminal
     geometry; text selection and copy stay as they are.
   - **Header row:** a real extra row per block. Changes line math in the
     terminal grid (scrolling, selection, search, copy) — a much larger change.
2. **How the terminal crate gets token colors.**
   - **(Recommended)** Through its existing theme settings: the app passes
     block colors (selected, hover, line, error fill, error edge, meta text,
     ok, error) taken from `UiPalette`. The terminal crate stays independent
     of the kit, as a reusable widget.
   - The terminal crate depends on `otty-ui-kit` and reads `UiPalette` itself.
3. **Actions on hover.** The design shows Copy command, Copy output, Rerun,
   Ask agent. Recommended: Copy command and Copy output now (they exist as
   block commands); Rerun as its own ticket (writes the command and Enter to
   the session only when the shell is at an idle prompt); Ask agent waits for
   the agent sessions spec.
4. **Sticky header** (header pinned to the top with `shadow-sticky` while long
   output scrolls). Recommended: out of scope, because with the badge
   (question 1) there is no header to pin. Revisit with a header row.
5. **Duration precision.** Recommended: shells send milliseconds
   (`EPOCHREALTIME` in zsh and bash 5+, fallback to seconds × 1000). The hooks
   ship inside the app, so the parser and the scripts change together.

## Decisions

Filled in after the owner answers the questions above.

## Acceptance Criteria

Written for the recommended answers; adjusted after review.

- [ ] AC1: The line between blocks is `line`, `line-width` thick; the selected block fill is `surface-selected`; the hovered block fill is `surface-header`. Colors switch with the theme without a restart. No color in `crates/ui/terminal` is computed by changing alpha of a palette color for block chrome.
- [ ] AC2: In zsh and bash sessions, after a command finishes, its block has `exit_code`, `started_at` and `finished_at` (milliseconds), and keeps its `cmd` and `cwd` (unit tests on the surface merge; manual check with `true`, `false`, `exit 3` in a subshell, an empty Enter and Ctrl C).
- [ ] AC3: Block state is derived in one tested function: running, succeeded, failed with code, or unknown (no hooks, e.g. SSH). Unknown blocks show no badge and no error styling.
- [ ] AC4: A failed block has `surface-error` fill and a 2 px `status-error` edge on the left; text on it keeps 4.5:1 (contrast test with the theme's foreground).
- [ ] AC5: Every finished command block shows a badge on its first line: `✓` and duration in `status-ok` / `text-muted`, or `✕ exit N` and duration in `status-error` / `text-muted`, in `term-small`. The state is never shown by color alone.
- [ ] AC6: On hover the badge is replaced by actions Copy command and Copy output; the hovered action has a visible background (`surface-hover` is not used for status text, per the design system rule).
- [ ] AC7: Durations format as `0.4 s`, `48.2 s`, `3 min 12 s`, `1 h 05 min` (table test).
- [ ] AC8: `cargo +nightly fmt --check`, clippy with `-D warnings`, `cargo test --workspace --all-features`, `cargo deny check` pass; line coverage is strictly greater than on the merge base.

## Out of Scope

- Header row and sticky header (question 1 and 4).
- Ask agent to fix (needs agent sessions).
- Exit codes for SSH sessions and for shells other than zsh and bash.
- The `source` link from the preview (`reducer.rs:212`): needs output parsing.

## Progress Tracking

- [ ] 01 — Block colors from tokens · Sonnet
- [ ] 02 — Shells report exit code and millisecond times · Opus
- [ ] 03 — Block state and duration format · Sonnet
- [ ] 04 — Failed block look · Sonnet
- [ ] 05 — Meta badge and hover actions · Opus
- [ ] 06 — Rerun action · Sonnet
- [ ] 07 — Update the design system CommandBlock · Opus, owner review

Order: 01 → 02 → 03 → 04 → 05 → 06 → 07, one at a time (01, 04 and 05 touch the same drawing code). Each ticket names its model on its `Model:` line; one subagent and one commit per ticket.

## Round Log

- Draft (2026-10-31): written after the design system spec; waiting for answers to the questions above.
