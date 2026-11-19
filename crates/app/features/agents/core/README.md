# otty-app-agents

Agents mode of the OTTY app without I/O: the profiles of the coding agents
(Claude Code, Codex), the projects their sessions ran in, the sessions of
the selected project, and "Continue" and "New session", which ask for a
terminal tab that runs the agent in the project folder.

It fills the left panel (the accounts, the profiles and the projects), the
main area and the left side of the status bar. It must not read files or start programs
itself: `otty-app-agents-io` reads the profiles and the sessions, and the
binary opens the tab.
