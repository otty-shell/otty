//! The engine work of the containers feature: the engine search, the list
//! loads, the clipboard and the browser. Each runs off the update, and its
//! answer comes back as a containers intent.
//!
//! Issue 14 adds the engine calls of the children here: the inspect answer
//! and the run of the followed logs, the actions, the Run form (image
//! defaults, the run, the folder checks and their waits), the pulls with
//! their abort handles, and the shell tab of a container.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary talks to the engines and starts the browser"
)]

use std::process::{Command, Stdio};

use iced::Task;
use iced::futures::future::join3;
use otty_app_containers::{
    ContainersError, ContainersIntent, LoadedList, LoadedLists, ResourceKind,
};
use otty_containers::{Container, Engine, EngineError, EngineInfo, discover};

use crate::message::AppEvent;

/// Search for the local engines; the result comes back as
/// [`ContainersIntent::DiscoveryFinished`].
pub(crate) fn discover_engines() -> Task<AppEvent> {
    Task::perform(discover(), |discovery| {
        intent(ContainersIntent::DiscoveryFinished(discovery))
    })
}

/// The same search behind the no-engine card; the result comes back as
/// [`ContainersIntent::BackgroundSearchFinished`].
pub(crate) fn discover_in_background() -> Task<AppEvent> {
    Task::perform(discover(), |discovery| {
        intent(ContainersIntent::BackgroundSearchFinished(discovery))
    })
}

/// Load the four lists of `info`; they come back as
/// [`ContainersIntent::ListsLoaded`].
pub(crate) fn load_lists(info: EngineInfo) -> Task<AppEvent> {
    let engine = info.address().clone();

    Task::perform(read_lists(info), move |lists| {
        intent(ContainersIntent::ListsLoaded {
            engine: engine.clone(),
            lists,
        })
    })
}

/// Load one list of `info` again, after an engine event; it comes back as
/// [`ContainersIntent::ListLoaded`]. "Used by" of images, volumes and
/// networks is read from `containers`.
pub(crate) fn load_list(
    info: EngineInfo,
    resource: ResourceKind,
    containers: Vec<Container>,
) -> Task<AppEvent> {
    let engine = info.address().clone();

    Task::perform(read_list(info, resource, containers), move |list| {
        intent(ContainersIntent::ListLoaded {
            engine: engine.clone(),
            list,
        })
    })
}

/// Put `text` on the clipboard.
pub(crate) fn copy_to_clipboard(text: String) -> Task<AppEvent> {
    iced::clipboard::write(text)
}

/// Open `url` in the default browser. Whether the browser could be started
/// comes back as [`ContainersIntent::LinkOpened`].
pub(crate) fn open_url(url: String) -> Task<AppEvent> {
    Task::perform(async move { start_browser(&url) }, |result| {
        intent(ContainersIntent::LinkOpened(result))
    })
}

/// The error of an engine as the containers feature shows it: the engine's
/// own words when it refused a request, otherwise the whole error text.
fn containers_error(err: EngineError) -> ContainersError {
    ContainersError::new(err.message())
}

/// Load the four lists of one engine. The containers come first, because
/// "used by" of images, volumes and networks is read from them; then the
/// other three load at the same time. Each list keeps its own error, so one
/// failed list does not hide the others.
async fn read_lists(info: EngineInfo) -> LoadedLists {
    let engine = match Engine::connect(&info) {
        Ok(engine) => engine,
        Err(err) => return LoadedLists::failed(&containers_error(err)),
    };

    let containers = engine.containers().await.map_err(containers_error);

    let users = containers.as_deref().unwrap_or_default();
    let (images, volumes, networks) = join3(
        engine.images(users),
        engine.volumes(users),
        engine.networks(users),
    )
    .await;

    LoadedLists {
        images: images.map_err(containers_error),
        volumes: volumes.map_err(containers_error),
        networks: networks.map_err(containers_error),
        containers,
    }
}

/// Load one list of one engine again. "Used by" of images, volumes and
/// networks is read from `containers`.
async fn read_list(
    info: EngineInfo,
    resource: ResourceKind,
    containers: Vec<Container>,
) -> LoadedList {
    let engine = match Engine::connect(&info) {
        Ok(engine) => engine,
        Err(err) => return failed_list(resource, containers_error(err)),
    };

    match resource {
        ResourceKind::Containers => LoadedList::Containers(
            engine.containers().await.map_err(containers_error),
        ),
        ResourceKind::Images => LoadedList::Images(
            engine.images(&containers).await.map_err(containers_error),
        ),
        ResourceKind::Volumes => LoadedList::Volumes(
            engine.volumes(&containers).await.map_err(containers_error),
        ),
        ResourceKind::Networks => LoadedList::Networks(
            engine.networks(&containers).await.map_err(containers_error),
        ),
    }
}

/// `resource` failed to load with `error`.
fn failed_list(resource: ResourceKind, error: ContainersError) -> LoadedList {
    match resource {
        ResourceKind::Containers => LoadedList::Containers(Err(error)),
        ResourceKind::Images => LoadedList::Images(Err(error)),
        ResourceKind::Volumes => LoadedList::Volumes(Err(error)),
        ResourceKind::Networks => LoadedList::Networks(Err(error)),
    }
}

/// Open `url` in the default browser with `xdg-open` (Linux) or `open`
/// (macOS). The error is why the opener could not be started; it is logged
/// too. An opener that starts but then fails reports later, on a helper
/// thread, and that is only logged.
fn start_browser(url: &str) -> Result<(), String> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    let spawned = Command::new(program)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    let mut child = spawned.map_err(|err| {
        log::warn!("opening {url} failed: cannot run {program}: {err}");
        format!("cannot run {program}: {err}")
    })?;

    // Wait on a thread so the finished opener does not stay a zombie.
    std::thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => {
            log::warn!("{program} failed to open a link: {status}");
        },
        Ok(_) => {},
        Err(err) => log::warn!("{program} wait failed: {err}"),
    });

    Ok(())
}

fn intent(intent: ContainersIntent) -> AppEvent {
    AppEvent::Containers(intent)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_containers::{EngineKind, fixtures};

    use super::*;

    #[test]
    fn each_engine_effect_runs_one_engine_task() {
        let search = discover_engines();
        let background = discover_in_background();
        let load = load_lists(fixtures::engine(EngineKind::Docker));
        let reload = load_list(
            fixtures::engine(EngineKind::Docker),
            ResourceKind::Images,
            Vec::new(),
        );

        assert_eq!(search.units(), 1);
        assert_eq!(background.units(), 1);
        assert_eq!(load.units(), 1);
        assert_eq!(reload.units(), 1);
    }

    #[test]
    fn copying_runs_one_clipboard_task() {
        let copy =
            copy_to_clipboard(String::from("sudo systemctl start docker"));

        assert_eq!(copy.units(), 1);
    }

    #[test]
    fn an_engine_error_becomes_the_text_of_its_message() {
        let engine_error = EngineError::PathNotUtf8(PathBuf::from("/run/x"));
        let message = engine_error.message();

        let error = containers_error(engine_error);

        assert_eq!(error.to_string(), message);
    }

    #[test]
    fn a_failed_list_keeps_its_kind_and_the_error() {
        let error = ContainersError::new("refused");

        let lists = [
            ResourceKind::Containers,
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
        ]
        .map(|resource| failed_list(resource, error.clone()));

        assert_eq!(
            lists,
            [
                LoadedList::Containers(Err(error.clone())),
                LoadedList::Images(Err(error.clone())),
                LoadedList::Volumes(Err(error.clone())),
                LoadedList::Networks(Err(error)),
            ]
        );
    }
}
