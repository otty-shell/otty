# otty-app-notifications

The notifications feature of the OTTY app: toasts at the top of the window
for errors and finished actions, and the list of recent ones under the bell
in the header.

It fills the overlays (the list under the bell) and draws the toasts with its
own method. It has no I/O, so it is one crate.
