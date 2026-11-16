# otty-app-shell

The shell that new terminals start, and the OTTY shell integration: small zsh
and bash scripts that make the shell tell the terminal its commands and
current folder.

It is a separate crate because several parts of the app need it: the
terminal for new shell tabs, quick launch for launches of a plain zsh or
bash, and Agents mode, which starts a coding agent through the user's login
shell so a program installed by a version manager is found.
