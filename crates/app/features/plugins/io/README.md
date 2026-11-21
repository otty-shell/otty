# otty-app-plugins-io

The I/O of the Plugins page: it opens the installed plugins in
`plugins.json`, finds, installs, updates, reloads and removes plugins
through `otty-plugin-releases` on a thread of its own, and turns the
answers into the page's intents. OTTY installs no plugin by itself.

It is apart from `otty-app-plugins` so that the page itself never touches
the network or the disk.
