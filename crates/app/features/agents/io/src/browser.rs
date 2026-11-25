//! Opening a page in the user's browser, such as the usage page of an
//! account at its provider.

use std::process::{Command, Stdio};

use iced::Task;
use otty_app_agents::AgentsIntent;

/// Open `address` in the user's browser with the system's opener:
/// `xdg-open` on Linux, `open` on macOS. The answer comes back as
/// [`AgentsIntent::PageOpened`], with why the opener could not be started.
pub fn open_page(address: String) -> Task<AgentsIntent> {
    Task::perform(async move { start_browser(&address) }, |result| {
        AgentsIntent::PageOpened(result)
    })
}

/// Start the opener with `address`. An opener that starts but then fails
/// is only logged, from a helper thread that waits for it so it does not
/// stay a zombie.
fn start_browser(address: &str) -> Result<(), String> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    let spawned = Command::new(program)
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = spawned.map_err(|err| {
        log::warn!("opening {address} failed: cannot run {program}: {err}");
        format!("cannot run {program}: {err}")
    })?;

    std::thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => {
            log::warn!("{program} failed to open a page: {status}");
        },
        Ok(_) => {},
        Err(err) => log::warn!("{program} wait failed: {err}"),
    });
    Ok(())
}
