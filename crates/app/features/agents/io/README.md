# otty-app-agents-io

The I/O of Agents mode: it reads the saved profiles in `agents.json` next to
`modes.json`, finds whether each agent's program can be started, checks each
profile's login and reads its sessions, all on a thread of its own, and
turns the answers into the agents feature's intents.

It is apart from `otty-app-agents` so that the feature itself never reads a
file or starts a program.
