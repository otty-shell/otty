---
name: otty-pr-hardening
description: |
  Prepares or reworks Otty pull requests that touch terminal input, keyboard
  shortcuts, pane splitting, or maintainer review feedback. Use when an Otty
  PR adds key bindings, receives scope or platform comments, or needs final
  pre-review hardening.
targets: [claude, codex]
tags: [rust, pull-request, terminal, review]
metadata:
  version: 1.3.0
---

# Otty PR Hardening

## Critical Rules

1. Build the final diff around the issue, not around everything discovered
   while working. A real pre-existing bug belongs in a separate PR unless the
   requested behavior cannot pass without it.
2. Treat keyboard modifiers as platform semantics, not portable names.
   `COMMAND` aliases `CTRL` away from macOS; inspect the actual bit used by the
   physical key before registering a binding.
3. Match application shortcuts inside the focused terminal widget and route a
   typed action with the reporting terminal id. A global keyboard subscription
   can act on a terminal while settings or another widget owns focus.
4. An application binding prevents PTY output by matching before the text
   fallback. Keep unbound-key fallback behavior intact; do not globally discard
   every Logo-modified character to fix one shortcut.
5. No Git write, review reply, or PR update is part of this skill unless the
   user explicitly authorizes it.
6. A reviewer's direction label is not a new shortcut contract. Preserve the
   issue's visible geometry, and mark a request `CLARIFY` when it does not name
   both the intended layout and the key combination. Do not invent either.
7. Otty's verified host-action contract is `BindingAction<T = ()>` carried by
   `Event::BindingActionDispatched`. Keep the unit default for existing callers;
   do not substitute strings, `Infallible`, or the ambiguous `Event::Action`.

## Inputs

- The issue or PR number.
- The current branch and base branch.
- All inline review threads, including resolved and outdated threads.

## Workflow

### 1. Lock the evidence boundary

For a GitHub PR, collect the issue contract, current files, checks, and every
inline comment before editing:

```bash
gh issue view ISSUE -R otty-shell/otty
gh pr view PR -R otty-shell/otty --json baseRefName,headRefName,files,reviews,statusCheckRollup
gh api graphql -f query='query { repository(owner: "otty-shell", name: "otty") { pullRequest(number: PR) { reviewThreads(first: 100) { nodes { id isResolved isOutdated comments(first: 100) { nodes { databaseId author { login } body path line originalLine } } } } } } }'
gh pr diff PR -R otty-shell/otty --name-only
git status --short --branch
```

For a fork PR, resolve the pull request against the base repository
(`otty-shell/otty`), not the head fork. A PR number is repository-local. Use
GraphQL `reviewThreads` as the source of truth because the flat REST comment
list does not expose thread resolution state.

Write a disposition row for every comment: `ACCEPT`, `REJECT`, or `CLARIFY`,
with a file or command as evidence. Completion condition: comment count in the
matrix equals the root-thread count returned by GraphQL.

### 2. Enforce diff lineage

Classify every changed file before fixing review feedback:

| Class | Final-diff action |
|---|---|
| Required by the issue | Keep and test |
| Regression caused by the issue implementation | Keep with a reproducing test |
| Pre-existing lint, test, binding, policy, or documentation defect | Remove from this PR or move to a separately authorized PR |
| Generated plan or spec | Put it where the maintainer requests and verify it is trackable |

Do not keep a pre-existing fix merely because a strict local command exposes
it. Run the same command on the base revision when attribution is disputed and
record the result. Completion condition: every path in the final diff has one
lineage row and no row remains undecided.

### 3. Audit the complete keyboard event path

Trace these boundaries together:

```text
iced KeyPressed
  -> BindingsLayout exact modifier match
  -> BindingAction
  -> otty_ui_term Event
  -> TerminalWorkspaceIntent
  -> reporting terminal id to pane lookup
  -> split and explorer synchronization
```

Verify all of the following:

- The binding does not replace a terminal control character such as Ctrl+D.
- The binding reads `KeyPressed.modifiers`, not a cached modifier state that a
  newly created pane may never have received.
- Auto-repeat does not create one terminal and shell process per repeat event.
- A matched application action emits no `Write` event.
- An unbound key still follows the platform-provided text fallback.
- The action targets the pane belonging to the reporting terminal, not merely
  the active tab's currently focused pane.
- A shortcut-driven split triggers the same explorer synchronization as a
  context-menu split.
- Pane-width assertions use every layout parameter the application explicitly
  configures, including its real grid spacing. A zero-spacing helper can prove
  an ideal ratio while hiding the one-pixel rounding users actually see.
- Non-current platform mappings are testable through a pure selector or CI;
  `cfg`-gated tests on one developer machine are not cross-platform evidence.

Completion condition: each bullet has a behavioral test or an explicit
unverified-platform entry.

### 4. Keep the public API explicit and minimal

For host-defined terminal actions:

- Use `BindingAction<T = ()>` so existing embedders keep the verified default
  type.
- Name the emitted event by what happened, for example
  `BindingActionDispatched`, rather than reusing the ambiguous name `Action`.
- Keep feature modules private and re-export only the type an outer module
  genuinely imports.
- Add concise documentation to new public items.
- Compile the workspace examples after generic API changes; default type
  parameters can still fail at inference or conversion boundaries.

Completion condition: callers use the typed event, no string action names or
unknown-action fallback exists, and all examples compile.

### 5. Use behavioral red-green tests

Write the smallest failing test before each behavioral change. For shortcut
work, include these mutation checks:

- Change the split axis: a routing test must fail.
- Use cached modifiers: the newly created pane test must fail.
- Allow repeat events: the action-count test must fail.
- Remove the application binding: the no-PTY-write test must fail.
- Replace a safe non-macOS modifier with `CTRL`: the collision test must fail.

Do not add source-text assertions or tests for a private constant. Completion
condition: the new test failed for the intended behavior, then passed after one
production change.

### 6. Reconcile repository and documentation constraints

- Otty repository prose is English, and Chinese text is forbidden everywhere,
  including test data. A non-Chinese wide character, such as Japanese kana,
  may be used only when the payload property itself is under test.
- A payload-only edit still needs issue lineage; do not rewrite test data just
  to satisfy an unrelated policy change.
- Maintainer-requested specs use `specs/YYYY-MM-DD-slug/spec.md` when that path
  is requested. Check it with `git check-ignore -v` before deleting the old
  tracked file.
- If the requested spec path is ignored, add the narrowest `.gitignore`
  exception. Do not force-add the file.
- Update stale event names, platform shortcuts, and verification limits in the
  spec in the same change.

Completion condition: the new document appears in `git status`, the old path is
deleted, and exact-name searches return no stale current-behavior references.

### 7. Run the Otty gates

Run every required command fresh:

```bash
cargo +nightly fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features --fail-under-lines 80
```

For a desktop shortcut, also build and run Otty on the target platform. Record
the exact result for macOS, Linux, and Windows separately; one platform does
not prove another. Completion condition: every command has an exit code and
every unavailable platform is listed under `Not Verified`.

When a required test fails because it reads host state such as `$SHELL`, keep
the original command's failure as the gate result. A controlled environment
rerun can attribute the failure, but cannot replace or hide the default result.

### 8. Reply to review threads only after the pushed diff exists

This step is conditional on explicit authorization for both push and external
review replies.

- Push the verified commit before replying so every response can point to code
  that reviewers can inspect.
- Reply to each root review comment in its own thread. Include the implemented
  change or the technical reason for rejecting or clarifying it.
- Include resolved and outdated threads when the user asks for every review.
- Do not resolve threads unless the user explicitly asks for resolution.
- Re-query GraphQL after replying and verify that every root thread has a reply
  by the authenticated account.

Completion condition: pushed head SHA equals the local head, and the number of
root threads with an authenticated-author reply equals the root-thread count.

## Red Flags

- The PR body needs a section explaining unrelated changes.
- A platform check uses `COMMAND` without proving its bits on every target.
- A newly split pane depends on a prior `ModifiersChanged` event.
- A shortcut is handled both globally and inside the terminal widget.
- A held key can allocate panes or spawn shells repeatedly.
- A moved spec disappears from `git status`.
- Green unit tests are presented as hardware or cross-platform proof.

Any red flag returns the workflow to the corresponding step.

## Excuse to Reality

| Excuse | Reality |
|---|---|
| "The unrelated bug is real, so it can stay." | Correctness does not create issue lineage; separate it. |
| "COMMAND means Command everywhere." | Away from macOS it can mean Ctrl and replace EOF bindings. |
| "The event has text, so filtering Logo is safest." | A host binding should consume its shortcut; blanket filtering changes reusable terminal behavior. |
| "The active pane is the one that fired." | Global subscriptions do not prove which widget held focus. |
| "The non-macOS test is cfg-gated, so it is covered." | It is not executed on a macOS-only development run. |
| "The moved file exists on disk." | An ignored file will not enter the PR. |
| "The reviewer said horizontal, so add a shortcut." | An axis word alone does not define visible geometry or a key; preserve the issue contract and request both details. |

## Common Issues

### Ctrl+D sends EOF and splits a pane

Cause: a non-macOS `COMMAND` binding aliases `CTRL` and replaces the terminal
binding. Use a non-control combination chosen by the product owner and test the
exact modifier bits.

### A second split leaks a character into the new PTY

Cause: the new terminal view started with an empty modifier cache. Match against
the modifiers carried by the `KeyPressed` event.

### Holding the shortcut spawns many shells

Cause: repeated key events are dispatched as application actions. Reject repeat
events at the widget boundary where a binding becomes an action.

### The spec move looks like deletion only

Cause: `specs/` is ignored. Add a narrow exception and confirm the new path is
visible in `git status`; do not use force-add.

### Equal-pane tests pass but rendered widths still differ

Cause: the test called `pane_regions` with synthetic zero spacing instead of
the application's grid configuration. Measure with production spacing and
state an explicit pixel tolerance; do not claim sub-pixel equality from ideal
split ratios alone.

### A fork PR lookup says the PR does not exist

Cause: the lookup used the head fork, but pull request numbers belong to the
base repository. Query `otty-shell/otty` and keep the head repository only as
branch metadata.

## Example

User says: "Rework the pane-split PR after the maintainer's platform and scope
comments."

Result: a complete comment disposition matrix, an issue-lineage-only diff,
red-green shortcut tests, platform-separated verification, and a trackable spec
at the maintainer-requested path.

## When Not to Use

- A Rust change that does not touch a PR, terminal input, shortcuts, or pane
  behavior.
- A general code review outside the Otty repository.
- Writing a new product requirement before an issue contract exists.
