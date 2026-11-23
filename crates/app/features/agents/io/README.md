# otty-app-agents-io

The I/O of Agents mode: it reads and writes the saved profiles in
`agents.json` next to `modes.json`, finds whether each agent's program can be
started, checks each profile's login, reads its sessions, asks each provider
part for the accounts it finds and reads their numbers and bills (which may
ask the provider's server or start the agent), counts the tokens a profile
spent by the days of the user's time zone, which it asks from the system,
fetches the LiteLLM price table at most once a day and keeps it in OTTY's
cache folder, finds the folders the folder field of the New session and
profile dialogs offers and creates the folder of a new profile, all on a
thread of its own, and turns the answers into the agents feature's intents.
It also makes and removes the folders of OTTY's own logins under the data
folder, puts the command that continues a session, or its id, on the
clipboard, gives `otty-plugin-host` the way to start the programs and hidden
terminals of plugins, and loads the parts of the installed plugins that are
on into the agents, providers and registry of the mode. The only folders it
deletes are those of OTTY's own logins.

It is apart from `otty-app-agents` so that the feature itself never reads a
file or starts a program.
