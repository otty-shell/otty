# 10 Tabs

Status: needs-triage
Blocked by: 09

## What to do

Move `app/src/widgets/tabs` into `otty-app-tabs`, following "Moving a feature"
in the spec. It adds the `LeftPanel` trait to `otty-app-slots`.

- The Sessions section draws only from the tabs state. The host name arrives
  as an intent (`HostNameRead`) instead of being passed to the view.
- `TabMachine` and `WizardTabInit` move to `otty-app-domain`, since tabs and
  quick launch both read them.
- Opening, activating and closing a tab return outcomes that carry what other
  features need, for example `Closed { tab_id, new_active_id }`. The decisions
  about what the terminal does then belong to the terminal feature (issue 11),
  not to the binary.
- Tabs are only terminal sessions and the launch editor; settings is not a tab.

## Done when

- The Sessions section, the new session menu and the tab list grouped by machine
  work as in the old app, except what needs the terminal.
- `otty-app-tabs` depends on no other feature.
- The steps and checks in "Moving a feature" are done.

## Comments
