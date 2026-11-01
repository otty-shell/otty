//! A connection to one reached engine and the list calls on it.

use std::path::PathBuf;

use bollard::models::{
    ContainerSummary, ImageSummary, Network as EngineNetwork,
    Volume as EngineVolume,
};
use bollard::query_parameters::{
    ListContainersOptions, ListImagesOptions, ListVolumesOptions,
};
use bollard::{API_DEFAULT_VERSION, ClientVersion, Docker};
use futures_util::future::try_join4;

use crate::address::EngineAddress;
use crate::container::Container;
use crate::engine::EngineInfo;
use crate::image::Image;
use crate::network::Network;
use crate::volume::Volume;

/// An API version every engine OTTY supports accepts: 1.40 (Docker 19.03)
/// is inside the range of every Docker from 19.03 to 29, and Podman accepts
/// any version. Used to ask `/version`, and for an engine that did not say
/// which version it speaks.
pub(crate) const BASELINE_API_VERSION: ClientVersion = ClientVersion {
    major_version: 1,
    minor_version: 40,
};
/// bollard's limit for one request, in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Why talking to an engine failed.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// bollard takes socket paths as text.
    #[error("socket path is not valid UTF-8: {0:?}")]
    PathNotUtf8(PathBuf),
    /// The engine could not be reached, answered with an error, or sent
    /// something OTTY cannot read; the text says which.
    #[error(transparent)]
    Request(#[from] bollard::errors::Error),
}

/// All four lists of one engine, loaded together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    containers: Vec<Container>,
    images: Vec<Image>,
    volumes: Vec<Volume>,
    networks: Vec<Network>,
}

impl Resources {
    /// Every container, running or not, in the engine's order (newest first).
    pub fn containers(&self) -> &[Container] {
        &self.containers
    }

    /// Every top-level image, dangling ones included.
    pub fn images(&self) -> &[Image] {
        &self.images
    }

    /// Every volume.
    pub fn volumes(&self) -> &[Volume] {
        &self.volumes
    }

    /// Every network, built-in ones included.
    pub fn networks(&self) -> &[Network] {
        &self.networks
    }

    /// Map the four list answers of one engine; "used by" comes from the
    /// container list of the same answers.
    pub(crate) fn from_answers(
        container_list: &[ContainerSummary],
        image_list: &[ImageSummary],
        volume_list: &[EngineVolume],
        network_list: &[EngineNetwork],
    ) -> Self {
        let containers: Vec<Container> =
            container_list.iter().map(Container::from_summary).collect();

        let images = image_list
            .iter()
            .map(|image| Image::from_summary(image, &containers))
            .collect();
        let volumes = volume_list
            .iter()
            .map(|volume| Volume::from_volume(volume, &containers))
            .collect();
        let networks = network_list
            .iter()
            .map(|network| Network::from_network(network, &containers))
            .collect();

        Self {
            containers,
            images,
            volumes,
            networks,
        }
    }
}

/// A connection to one engine found by [`crate::discover`].
///
/// The client is set to the engine's own API version, capped at the newest
/// one bollard knows (1.53). Docker refuses a URL version outside its range,
/// and 1.53 is newer than every Docker before 29. bollard 0.21.1 in fact
/// sends no version in the URL (the engine then answers at its newest
/// version); the cap keeps old Docker working if a later bollard sends it.
///
/// Making it does not talk to the engine; each call does, once, with no
/// retries. Cloning is cheap and shares the connection pool. Calls need a
/// tokio runtime.
///
/// ```no_run
/// # async fn run() -> Result<(), otty_docker::EngineError> {
/// use otty_docker::{Diagnosis, Engine, discover};
///
/// let discovery = discover().await;
/// if let Diagnosis::Engines(engines) = discovery.diagnosis() {
///     for info in engines {
///         let engine = Engine::connect(info)?;
///         let resources = engine.resources().await?;
///         println!("{} containers", resources.containers().len());
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Engine {
    info: EngineInfo,
    client: Docker,
}

impl Engine {
    /// The engine this connection talks to.
    pub fn info(&self) -> &EngineInfo {
        &self.info
    }

    /// Prepare a connection to a reached engine.
    pub fn connect(info: &EngineInfo) -> Result<Self, EngineError> {
        let version = request_api_version(info.api_version());

        let client =
            open_client(info.address(), REQUEST_TIMEOUT_SECS, &version)?;

        Ok(Self {
            info: info.clone(),
            client,
        })
    }

    /// Load all four lists at once (entering Docker mode, Refresh). The four
    /// requests run at the same time; "used by" comes from the container
    /// list of the same load.
    pub async fn resources(&self) -> Result<Resources, EngineError> {
        let (container_list, image_list, volume_list, network_list) =
            try_join4(
                self.client.list_containers(Some(all_containers())),
                self.client.list_images(None::<ListImagesOptions>),
                self.client.list_volumes(None::<ListVolumesOptions>),
                self.client.list_networks(None),
            )
            .await?;

        Ok(Resources::from_answers(
            &container_list,
            &image_list,
            volume_list.volumes.as_deref().unwrap_or_default(),
            &network_list,
        ))
    }

    /// Every container, running or not.
    pub async fn containers(&self) -> Result<Vec<Container>, EngineError> {
        let list = self.client.list_containers(Some(all_containers())).await?;

        Ok(list.iter().map(Container::from_summary).collect())
    }

    /// Every top-level image. "Used by" counts come from `containers`, the
    /// container list the caller already has.
    pub async fn images(
        &self,
        containers: &[Container],
    ) -> Result<Vec<Image>, EngineError> {
        let list = self.client.list_images(None::<ListImagesOptions>).await?;

        Ok(list
            .iter()
            .map(|image| Image::from_summary(image, containers))
            .collect())
    }

    /// Every volume. Who uses each comes from `containers`.
    pub async fn volumes(
        &self,
        containers: &[Container],
    ) -> Result<Vec<Volume>, EngineError> {
        let answer =
            self.client.list_volumes(None::<ListVolumesOptions>).await?;

        Ok(answer
            .volumes
            .iter()
            .flatten()
            .map(|volume| Volume::from_volume(volume, containers))
            .collect())
    }

    /// Every network. Who is attached comes from `containers`.
    pub async fn networks(
        &self,
        containers: &[Container],
    ) -> Result<Vec<Network>, EngineError> {
        let list = self.client.list_networks(None).await?;

        Ok(list
            .iter()
            .map(|network| Network::from_network(network, containers))
            .collect())
    }
}

/// Make a bollard client for an address. bollard does not connect yet.
pub(crate) fn open_client(
    address: &EngineAddress,
    timeout_secs: u64,
    version: &ClientVersion,
) -> Result<Docker, EngineError> {
    let client = match address {
        EngineAddress::Unix(path) => {
            let text = path
                .to_str()
                .ok_or_else(|| EngineError::PathNotUtf8(path.clone()))?;
            Docker::connect_with_unix(text, timeout_secs, version)?
        },
        EngineAddress::Tcp(host) => {
            Docker::connect_with_http(host, timeout_secs, version)?
        },
    };

    Ok(client)
}

/// The API version for the client: what the engine reported, at most
/// bollard's default; the baseline when the engine said nothing readable.
fn request_api_version(reported: Option<&str>) -> ClientVersion {
    let Some(reported) = reported.and_then(parse_api_version) else {
        return BASELINE_API_VERSION;
    };

    if reported < *API_DEFAULT_VERSION {
        reported
    } else {
        *API_DEFAULT_VERSION
    }
}

/// Read `major.minor`, for example `1.44`.
fn parse_api_version(text: &str) -> Option<ClientVersion> {
    let (major, minor) = text.trim().split_once('.')?;

    Some(ClientVersion {
        major_version: major.parse().ok()?,
        minor_version: minor.parse().ok()?,
    })
}

/// Ask for stopped containers too.
fn all_containers() -> ListContainersOptions {
    ListContainersOptions {
        all: true,
        ..ListContainersOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::thread::JoinHandle;

    use bollard::models::SystemVersion;

    use super::*;
    use crate::test_support::{
        DOCKER_CONTAINERS, PODMAN_CONTAINERS, TempDir, read_request_line,
        write_response,
    };

    const DOCKER_IMAGES: &str =
        include_str!("../tests/fixtures/docker-images.json");
    const DOCKER_VOLUMES: &str =
        include_str!("../tests/fixtures/docker-volumes.json");
    const DOCKER_NETWORKS: &str =
        include_str!("../tests/fixtures/docker-networks.json");
    const PODMAN_IMAGES: &str =
        include_str!("../tests/fixtures/podman-images.json");
    const PODMAN_VOLUMES: &str =
        include_str!("../tests/fixtures/podman-volumes.json");
    const PODMAN_NETWORKS: &str =
        include_str!("../tests/fixtures/podman-networks.json");

    /// The four list answers of one fake engine.
    struct Answers {
        containers: &'static str,
        images: &'static str,
        volumes: &'static str,
        networks: &'static str,
    }

    const DOCKER: Answers = Answers {
        containers: DOCKER_CONTAINERS,
        images: DOCKER_IMAGES,
        volumes: DOCKER_VOLUMES,
        networks: DOCKER_NETWORKS,
    };
    const PODMAN: Answers = Answers {
        containers: PODMAN_CONTAINERS,
        images: PODMAN_IMAGES,
        volumes: PODMAN_VOLUMES,
        networks: PODMAN_NETWORKS,
    };

    fn version(major: usize, minor: usize) -> ClientVersion {
        ClientVersion {
            major_version: major,
            minor_version: minor,
        }
    }

    fn info(path: &Path, api_version: &str) -> EngineInfo {
        let answer: SystemVersion = serde_json::from_str(&format!(
            r#"{{"Version":"1.0","ApiVersion":"{api_version}"}}"#
        ))
        .expect("version answer");

        EngineInfo::from_version(
            &answer,
            EngineAddress::Unix(path.to_path_buf()),
        )
    }

    /// Serve `requests` requests from the fixture answers by path and return
    /// the request lines in the order they came.
    fn serve(
        listener: UnixListener,
        requests: usize,
        answers: Answers,
    ) -> JoinHandle<Vec<String>> {
        std::thread::spawn(move || {
            let mut lines = Vec::new();
            for mut stream in listener.incoming().take(requests).flatten() {
                let Some(line) = read_request_line(&mut stream) else {
                    continue;
                };
                let body = if line.contains("/containers/json") {
                    answers.containers
                } else if line.contains("/images/json") {
                    answers.images
                } else if line.contains("/volumes") {
                    answers.volumes
                } else {
                    answers.networks
                };
                write_response(&mut stream, "200 OK", body);
                lines.push(line);
            }
            lines
        })
    }

    /// The path of a request line without its query. bollard 0.21.1 sends
    /// no `/v1.xx` prefix (it joins an absolute path onto the versioned
    /// base); if a later bollard sends one, it must be `version`.
    fn plain_path(line: &str, version: &str) -> String {
        let target = line.split(' ').nth(1).unwrap_or_default();
        let path = target.split('?').next().unwrap_or_default();
        let prefix = format!("/v{version}");

        path.strip_prefix(&prefix).unwrap_or(path).to_owned()
    }

    #[test]
    fn newer_engine_gets_bollards_default_version() {
        assert_eq!(request_api_version(Some("1.56")), version(1, 53));
        assert_eq!(request_api_version(Some("2.0")), version(1, 53));
    }

    #[test]
    fn older_engine_gets_its_own_version() {
        assert_eq!(request_api_version(Some("1.44")), version(1, 44));
        assert_eq!(request_api_version(Some("1.53")), version(1, 53));
        assert_eq!(request_api_version(Some(" 1.41 ")), version(1, 41));
    }

    #[test]
    fn unknown_version_gets_the_baseline() {
        assert_eq!(request_api_version(None), version(1, 40));
        assert_eq!(request_api_version(Some("")), version(1, 40));
        assert_eq!(request_api_version(Some("1.44.1")), version(1, 40));
        assert_eq!(request_api_version(Some("one.two")), version(1, 40));
    }

    #[test]
    fn socket_path_that_is_not_utf8_is_an_error() {
        let path = Path::new(OsStr::from_bytes(b"/tmp/\xff.sock"));

        let result = Engine::connect(&info(path, "1.44"));

        assert!(
            matches!(result, Err(EngineError::PathNotUtf8(_))),
            "{result:?}"
        );
    }

    #[test]
    fn tcp_engine_connects_without_talking_to_it() {
        let answer: SystemVersion =
            serde_json::from_str(r#"{"ApiVersion":"1.44"}"#).expect("answer");
        let info = EngineInfo::from_version(
            &answer,
            EngineAddress::Tcp(String::from("127.0.0.1:1")),
        );

        let engine = Engine::connect(&info).expect("tcp client");

        assert_eq!(engine.info(), &info);
    }

    #[tokio::test]
    async fn docker_resources_are_loaded_at_its_capped_version() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve(listener, 4, DOCKER);
        let engine = Engine::connect(&info(&path, "1.56")).expect("engine");

        let resources = engine.resources().await.expect("resources");

        assert_eq!(resources.containers().len(), 6);
        assert_eq!(resources.images().len(), 3);
        assert_eq!(resources.volumes().len(), 2);
        assert_eq!(resources.networks().len(), 4);
        let busybox = resources
            .images()
            .iter()
            .find(|image| image.tags() == ["busybox:1.37"])
            .expect("busybox");
        assert_eq!(busybox.containers(), 6);
        assert_eq!(engine.client.client_version(), version(1, 53));
        let mut paths: Vec<String> = server
            .join()
            .expect("server thread")
            .iter()
            .map(|line| plain_path(line, "1.53"))
            .collect();
        paths.sort();
        assert_eq!(
            paths,
            ["/containers/json", "/images/json", "/networks", "/volumes"]
        );
    }

    #[tokio::test]
    async fn container_list_asks_for_stopped_containers_too() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve(listener, 1, DOCKER);
        let engine = Engine::connect(&info(&path, "1.44")).expect("engine");

        engine.containers().await.expect("containers");

        let lines = server.join().expect("server thread");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("all=true"), "{}", lines[0]);
    }

    #[tokio::test]
    async fn podman_lists_are_loaded_one_by_one_at_its_version() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve(listener, 4, PODMAN);
        let engine = Engine::connect(&info(&path, "1.44")).expect("engine");

        let containers = engine.containers().await.expect("containers");
        let images = engine.images(&containers).await.expect("images");
        let volumes = engine.volumes(&containers).await.expect("volumes");
        let networks = engine.networks(&containers).await.expect("networks");

        assert_eq!(containers.len(), 6);
        assert_eq!(
            images.iter().filter(|image| image.is_dangling()).count(),
            1
        );
        assert_eq!(
            volumes
                .iter()
                .filter(|volume| volume.is_anonymous())
                .count(),
            1
        );
        assert_eq!(networks.len(), 2);
        assert_eq!(engine.client.client_version(), version(1, 44));
        let paths: Vec<String> = server
            .join()
            .expect("server thread")
            .iter()
            .map(|line| plain_path(line, "1.44"))
            .collect();
        assert_eq!(
            paths,
            ["/containers/json", "/images/json", "/volumes", "/networks"]
        );
    }

    #[tokio::test]
    async fn engine_error_answer_is_a_request_error_with_its_message() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = std::thread::spawn(move || {
            if let Some(mut stream) = listener.incoming().flatten().next()
                && read_request_line(&mut stream).is_some()
            {
                write_response(
                    &mut stream,
                    "500 Internal Server Error",
                    r#"{"message":"storage is broken"}"#,
                );
            }
        });
        let engine = Engine::connect(&info(&path, "1.44")).expect("engine");

        let result = engine.containers().await;

        let Err(error @ EngineError::Request(_)) = result else {
            panic!("expected a request error, got {result:?}");
        };
        assert!(error.to_string().contains("storage is broken"), "{error}");
        server.join().expect("server thread");
    }
}
