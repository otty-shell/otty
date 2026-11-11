# otty-app-shell

The shell that new terminals start, and the OTTY shell integration: small zsh
and bash scripts that make the shell tell the terminal its commands and
current folder.

It is a separate crate because two features need it: the terminal for new
shell tabs, and quick launch for launches of a plain zsh or bash.
