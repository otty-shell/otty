use anyhow::Result;
use otty_libterm::{
    TerminalBuilder, TerminalEvent, TerminalRequest, TerminalSize, pty,
};

#[cfg(not(unix))]
#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("This example is only supported on Unix platforms.");
    Ok(())
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<()> {
    let size = TerminalSize {
        rows: 24,
        cols: 80,
        cell_width: 0,
        cell_height: 0,
    };

    let unix_builder = pty::unix("/bin/sh")
        .with_arg("-i")
        .set_controling_tty_enable()
        .with_size(size.into());

    let (mut runtime, mut engine, _handle, events, proxy) =
        TerminalBuilder::from_unix_builder(unix_builder)
            .with_size(size)
            .build_with_runtime()?;

    // Kick off a simple command and exit the shell.
    proxy.send(TerminalRequest::WriteBytes(
        b"echo 'hello from tokio runtime'\n".to_vec(),
    ))?;
    proxy.send(TerminalRequest::WriteBytes(b"exit\n".to_vec()))?;

    // Drive the runtime on a blocking task while we await events.
    let runtime_task =
        tokio::task::spawn_blocking(move || runtime.run(&mut engine, ()));

    let exit = tokio::spawn(async move {
        while let Ok(event) = events.recv_async().await {
            if let TerminalEvent::ChildExit { status } = event {
                return Ok(status);
            }
        }
        anyhow::bail!("event channel closed before child exit");
    });

    let status = exit.await??;
    runtime_task.await??;

    println!("Child process exited with: {status}");
    Ok(())
}
