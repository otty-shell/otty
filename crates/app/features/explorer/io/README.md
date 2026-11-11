# otty-app-explorer-io

The I/O of the explorer: it reads the folders the explorer asks for and
watches them for changes, and turns the answers into the explorer's intents.

It is apart from `otty-app-explorer` so that the feature itself stays free of
disk access.
