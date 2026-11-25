# otty-app-plugins

Plugins mode of the OTTY app without I/O: the installed plugins in the
left panel with a search by name, and in the main area the page of the
selected plugin with a switch for each part, its permissions and health.
It adds plugins by a GitHub link or from a local folder, shows the
install dialog with the permissions of each part and the update dialog
with what changed, and runs the daily update check. Settings and the
links and needs of Agents mode open it.

It fills the left panel, the main area and the mode overlays. It must not
touch the network or the disk itself: `otty-app-plugins-io` installs,
updates and removes plugins. It must not depend on the agents feature or
`otty-agents`: it lists the parts of any domain.
