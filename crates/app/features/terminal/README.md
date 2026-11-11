# otty-app-terminal

The terminal feature of the OTTY app: the live terminal sessions of the tabs,
their split panes and the pane menu.

It fills the main area and the overlays (the pane menu). It stays one crate
without an `io` crate because it owns its running sessions.
