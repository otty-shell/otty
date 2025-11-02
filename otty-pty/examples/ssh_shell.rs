//! Demonstrate connecting to a remote shell over SSH and executing commands.
//!
//! Required environment:
//! - `SSH_EXAMPLE_TARGET` must contain `host:port`.
//! - `SSH_EXAMPLE_USER` sets the username (defaults to `root`).
//! - `SSH_EXAMPLE_PASSWORD` optionally provides a password; if omitted, ssh-agent
//!   identities are tried first.
//!
//! Run with:
//! `cargo run --package otty-pty --example ssh_shell`

use std::env;
use std::error::Error;
use std::io::ErrorKind;
use std::thread;
use std::time::Duration;

use otty_pty::{SSHAuth, Session, SessionError, ssh};

fn main() -> Result<(), Box<dyn Error>> {
    let host = env::var("SSH_EXAMPLE_TARGET")
        .expect("Set SSH_EXAMPLE_TARGET before running the example");
    let user = env::var("SSH_EXAMPLE_USER").unwrap_or_else(|_| "root".into());

    let mut builder = ssh().with_host(&host).with_user(&user);

    if let Ok(password) = env::var("SSH_EXAMPLE_PASSWORD") {
        builder = builder.with_auth(SSHAuth::Password(password));
    }

    let mut session = builder.spawn()?;

    session
        .write(b"echo 'Hello from remote otty-pty session' && uname -a\n")?;

    let mut buffer = [0u8; 4096];

    for _ in 0..100 {
        match session.read(&mut buffer) {
            Ok(0) => {},
            Ok(read) => {
                let text = String::from_utf8_lossy(&buffer[..read]);
                print!("{text}");
            },
            Err(SessionError::IO(err))
                if err.kind() == ErrorKind::WouldBlock => {},
            Err(err) => return Err(err.into()),
        }

        thread::sleep(Duration::from_millis(50));
    }

    let exit_code = session.close()?;
    println!("\nRemote shell terminated with code {exit_code}");

    Ok(())
}
