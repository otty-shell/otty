# otty-app

The OTTY app you run: the window frame (header, side panels, status bar) and
the code that links the features together. Each mode decides which feature
fills which place of the window, and this crate passes what one feature
reports on to the next feature or to its `io` crate.

It is the only app crate that may depend on every other one; no crate
depends on it.
