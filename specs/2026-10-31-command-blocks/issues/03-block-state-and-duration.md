# 03 — Block state and duration format

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 02
Spec: [../spec.md](../spec.md)

## Details

- In the terminal crate, one function derives a block's state from its metadata: `Running` (command, not finished), `Succeeded` (exit 0), `Failed(code)` (non-zero), `Unknown` (no exit code and finished, e.g. SSH or no hooks). Prompt and full-screen blocks have no state.
- One function formats a duration in milliseconds: `0.4 s` under a minute (one decimal), `3 min 12 s` under an hour, `1 h 05 min` above.

## Tests first

- Table tests for every state and for the duration format (spec AC3, AC7), including 0 ms, 59 999 ms, 60 000 ms, 3 600 000 ms.
