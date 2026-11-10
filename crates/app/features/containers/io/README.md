# otty-app-containers-io

The I/O of Containers mode: it finds the Docker or Podman engines, calls
them, watches their events and follows container logs, and turns the answers
into the containers feature's intents. It also opens the browser and the
clipboard for that feature.

It is apart from `otty-app-containers` so that the feature itself never
calls an engine.
