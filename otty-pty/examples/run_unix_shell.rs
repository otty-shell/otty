//! Demonstrate spawning a local shell and exchanging data with it.
//!
//! Run with:
//! `cargo run --package otty-pty --example run_unix_shell`

#[cfg(unix)]
use otty_pty::{Session, SessionError, unix};
use std::error::Error;
use std::io::ErrorKind;
use std::thread;
use std::time::Duration;

#[cfg(not(unix))]
fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("run_unix_shell example is only available on Unix platforms.");
    Ok(())
}

#[cfg(unix)]
fn main() -> Result<(), Box<dyn Error>> {
    let mut session = unix("/bin/sh").with_arg("-i").spawn()?;

    session.write(b"echo 'Hello from otty-pty' && date\n")?;

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
    println!("\nShell terminated with code {exit_code}");

    Ok(())
}
