# otty-containers

Finds the local Docker or Podman engine and talks to it. It is a separate
crate so that `bollard`, the Docker client, lives in one place in OTTY.

With the default `client` feature turned off it holds only the data types
(containers, images, volumes, networks), so a crate can show engines without
being able to call one.
