# otty-app-quick-launch

The quick launch feature of the OTTY app without I/O: its state, decisions
and views. It keeps the tree of saved launches, the launch editor and the
prompt for an SSH password or key passphrase. The launches file and the check
before a launch runs live in `otty-app-quick-launch-io`.

It fills the left panel (the Launches section), the main area (the launch
editor tab) and the overlays (the tree menu and the SSH prompt). It must not
touch files or start programs itself.
