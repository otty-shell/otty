# otty-app-agents-io

The I/O of Agents mode: it reads and writes the saved profiles in
`agents.json` next to `modes.json`, finds whether each agent's program can be
started, checks each profile's login, reads its sessions and its limits (which may
ask the agent's server), counts the tokens it spent by the days of the user's
time zone, which it asks from the system, fetches the LiteLLM price
table at most once a day and keeps it in OTTY's cache folder, finds the folders
the folder field of the New session and profile dialogs offers and creates the folder of a new profile, all on a
thread of its own, and turns the answers into the agents feature's intents.
It also puts the command that continues a session, or its id, on the
clipboard, gives `otty-plugin-host` the way to start the programs and
hidden terminals of plugins, and loads the parts of the installed plugins
that are on into the agents, providers and registry of the mode. It never
deletes a folder.

It is apart from `otty-app-agents` so that the feature itself never reads a
file or starts a program.
