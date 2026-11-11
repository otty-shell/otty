# otty-app-domain

The plain types the OTTY app crates share: the modes, the saved launches, the
app commands, the shortcut names, and what one feature hands to another.

It has no `iced`, no I/O and no `tokio`, so the binary, every feature and
every helper crate may depend on it.
