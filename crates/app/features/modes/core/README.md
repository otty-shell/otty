# otty-app-modes

The modes feature of the OTTY app without I/O: which mode is active, the
pinned modes of the mode switcher and the All modes menu. Reading and writing
the pinned modes file lives in `otty-app-modes-io`.

It fills the mode switcher above the left panel content (its own method) and
the overlays (the All modes menu). It must not touch files itself.
