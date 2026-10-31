# 02 — Shells report exit code and millisecond times

Status: needs-triage
Type: task
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

Send an `exit` event with the exit code when a command finishes, keep the
block's other metadata, and send times in milliseconds (spec question 5).

## Details

- `assets/shell-integrations/otty.zsh` and `otty.bash`: in the precmd hook, read `$?` as the very first statement (before any other code can change it), and if a command block is open (`preexec` ran since the last prompt) emit `{"v":2,"id":"<that cmd id>","phase":"exit","exit_code":N,"time":<ms>}` before the `precmd` event. Nothing is sent after an empty Enter. Times: `EPOCHREALTIME` → milliseconds, fallback seconds × 1000; all events use `"v":2`.
- Check how other precmd hooks (prompt frameworks, bash-preexec `__bp_last_ret_value`) affect `$?`; register the hook so it runs first, and write down what was verified.
- `crates/surface/src/block.rs`: on `BlockPhase::Exit`, merge into the existing block: set `exit_code` and `finished_at`, keep `cmd`, `cwd`, `started_at`, `shell`. `begin_block` must not overwrite a `finished_at` already set by `exit`.
- `crates/escape/src/dcs/block.rs`: accept `"v":2`; document that `time` is milliseconds.

## Tests first

- Surface: `exit` after `preexec` keeps `cmd`/`cwd`/`started_at` and sets `exit_code`/`finished_at`; `exit` for an unknown id changes nothing; a `precmd` after `exit` does not change `finished_at`.
- Escape: parse an `exit` payload with `exit_code` and `v` 2.

## Done when

- Spec AC2, including the manual checks listed there, in zsh and bash.
