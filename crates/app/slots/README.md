# otty-app-slots

The socket every OTTY app feature plugs into. It writes down, as traits, what
a feature gives the window frame: how it changes its state and which places
it fills (left panel, main area, right panel, status bar, overlays).

It is a separate crate so that the features and the binary agree on one
contract that the compiler checks, without depending on each other.
