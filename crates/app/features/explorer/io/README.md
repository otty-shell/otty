# otty-app-explorer-io

The I/O of the explorer: it reads the folders the explorer asks for,
watches them for changes, and creates, moves and deletes the files the user
asks for in Files, and turns the answers into the explorer's intents.

It is apart from `otty-app-explorer` so that the feature itself stays free of
disk access.
