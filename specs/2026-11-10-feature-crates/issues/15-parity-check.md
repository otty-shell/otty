# 15 Parity check

Status: needs-triage
Blocked by: 16

## What to do

Compare the new app with the old one before the owner removes the old one.

- Write a manual check list that walks through every mode, overlay and shortcut
  in both apps side by side. The owner runs it.
- Check the overlay rules from the spec ("Overlays") with each pair of overlays
  that can be open together.
- Check startup: shell integration files, host name, saved theme, saved
  launches and settings, including broken files.
- Search the new app for any `Task::done` sent from one feature to another and
  for I/O outside `effects/` and `subscription.rs`.
- Each difference found becomes its own issue in this folder.

The release still ships the old app. The steps for removing it are in the spec,
"When the old app is removed".

## Done when

- The check list is written and the owner has run it.
- Every difference is fixed or has its own issue.

## Comments
