# otty-app-plugins

The Plugins page of the OTTY app without I/O: the installed plugins with
their parts and a switch for each, Add plugin by a GitHub link or from a
local folder, the install dialog with the permissions of each part, the
update dialog with what changed, and the daily update check. It opens from
Settings and from the links and needs of Agents mode.

It fills the overlays. It must not touch the network or the disk itself:
`otty-app-plugins-io` installs, updates and removes plugins.
