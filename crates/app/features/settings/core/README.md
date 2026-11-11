# otty-app-settings

The settings feature of the OTTY app without I/O: its state, decisions and
views. It is the Settings dialog for the language, the terminal shell and
editor, how panes split, and the theme. Reading and writing the settings file
lives in `otty-app-settings-io`.

It fills the overlays. It must not touch files itself.
