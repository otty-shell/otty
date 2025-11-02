# otty-pty

`otty-pty` provides a thin, async-friendly abstraction over pseudo-terminals
that powers the `otty` shell project. It exposes a single `Session` trait with
two interchangeable concrete implementations:

- **Unix PTY** – spawns local processes on Unix-like systems and manages their
  controlling terminal.
- **SSH PTY** – establishes an SSH connection (via `libssh2`) and interacts with
  a remote pseudo-terminal as if it were local.

The crate is designed to integrate with `mio` runtime.

## Requirements

- A Unix-like platform with PTY support.
- `libssh2` at build/run time for the SSH backend (this is managed
  automatically by the `ssh2` crate, but the native library must be available).

## Using the Library

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
otty-pty = { path = "../otty-pty" } # adjust the path or use the published crate
```

Create a session by picking the appropriate builder and then drive it through
the `Session` trait:

```rust
use otty_pty::{unix, ssh, Session, PtySize, SSHAuth};

fn spawn_local_shell() -> anyhow::Result<()> {
    let mut session = unix("/bin/sh")
        .with_arg("-i")
        .with_size(PtySize::default())
        .spawn()?;

    session.write(b"echo hello from otty-pty\n")?;
    let mut buf = [0u8; 4096];
    let read = session.read(&mut buf)?;
    println!("{}", String::from_utf8_lossy(&buf[..read]));

    session.close()?;
    Ok(())
}

fn spawn_remote_shell() -> anyhow::Result<()> {
    let mut session = ssh()
        .with_host("example.com:22")
        .with_user("user")
        .with_auth(SSHAuth::Password("secret".into()))
        .spawn()?;

    session.write(b"uname -a\n")?;
    let mut buf = [0u8; 4096];
    let read = session.read(&mut buf)?;
    println!("{}", String::from_utf8_lossy(&buf[..read]));

    session.close()?;
    Ok(())
}
```

Both session types also implement `Pollable`, so they can be registered with a
`mio::Registry` for event-driven workloads.

## Examples

Two runnable examples live in the `examples/` directory:

- `run_unix_shell.rs` – spawns an interactive `/bin/sh`, issues a command, and
  prints the output.
- `ssh_shell.rs` – opens an SSH session to a remote host and runs a command.

Run them via:

```bash
cargo run --package otty-pty --example run_unix_shell
cargo run --package otty-pty --example ssh_shell
```

> **Note:** `ssh_shell` requires you to export `SSH_EXAMPLE_TARGET` (and
> optionally `SSH_EXAMPLE_USER` / `SSH_EXAMPLE_PASSWORD`) to point at a reachable
> SSH server.
