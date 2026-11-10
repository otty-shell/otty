# otty-app-containers

The containers feature of the OTTY app without I/O: its state, decisions and
views for Containers mode (engines, containers, images, volumes, networks,
the Run form, the inspector and the dialogs). The Docker calls live in
`otty-app-containers-io`.

It fills the left panel (the engine tree), the main area, the right panel
(the inspector), the status bar and the overlays. It must not call an engine
itself: it uses `otty-containers` without its Docker client.
