# 13 — Restyle quick launch editor, secret prompt and error tab

Status: resolved
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

## Answer

- Wizard uses kit FormSection, TextField, Segmented, TokenField and KeyValueField; secret prompt is a secure TextField on surface-overlay with shadow and radius-lg; error tab uses title and term-small; `menu_panel_style` removed.
- Arguments and extra arguments are kit `TokenList`s: new `WizardArgInput/Submit/Backspace` intents (and extra-arg twins) replace add/update; Save turns pending typed text into an argument. Stored data unchanged. Reducer tests written first.
- Kit: TextField gained `.secure()` and `.id()`; `segmented` borrows options only while building.
- Known debt: `WizardEditorState` keeps one untyped error string, so the view matches it against the validation messages to put it under the right field. A typed field error would remove this string match; left for a follow-up.
- Not checked visually: the wizard, secret prompt and error tab open only through input. Check them in the final review.
