# 13 — Restyle quick launch editor, secret prompt and error tab

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 05, 06
Spec: [../spec.md](../spec.md)

## Details

- `wizard_form.rs`: `FormSection`s Connection / Command; fields `TextField` (Host and Port share a row, Port narrow; Identity file with Browse if a file picker exists, otherwise without action); Type and After it exits become `Segmented`; Arguments and Extra arguments `TokenField`; Environment `KeyValueField`.
- Footer: Cancel (`ghost`), Save (`primary`) last on the right.
- `secret_prompt.rs`: dialog on `surface-overlay` with `shadow-popover` and `radius-lg`; field `TextField`; buttons kit `Button`. Keep the backdrop.
- `error_tab.rs`: `title` text, message in `body`, error detail in `term-small`.
- Field errors use the `error` slot of `TextField` with `status-error` and the wording "what is wrong, how to fix it".

## Done when

- Screenshots of the editor (custom and SSH), the secret prompt and the error tab in both themes.
