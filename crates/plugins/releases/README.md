# otty-plugin-releases

Installs, updates and removes OTTY plugins. A plugin comes from a GitHub
release of its author's repository, from a local folder its author built
it in, or from the copy built into OTTY; this crate finds the newest
release this OTTY can run, downloads and checks its files, keeps them in
the plugins folder, records them in `plugins.json`, and compares the
permissions of a new version with the ones the user granted.

It is a separate crate so that the network, the files on disk and the
user's choices stay apart from running plugins (`otty-plugin-host`) and
from the Plugins page. It holds the one constant with the name of the
official GitHub organization. It must not depend on `iced` or on any crate
under `crates/app/`.
