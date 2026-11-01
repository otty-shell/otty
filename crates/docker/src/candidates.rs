//! The ordered list of places where a local engine may listen.
//!
//! The list is built from plain inputs (environment, platform, user id), so
//! the search order is tested without touching the file system.

use std::path::{Path, PathBuf};

use crate::address::EngineAddress;

/// Docker's default socket on Linux and macOS.
const DOCKER_SOCKET: &str = "/var/run/docker.sock";
/// Docker Desktop's per-user socket on macOS.
const DOCKER_DESKTOP_SOCKET: &str = ".docker/run/docker.sock";
/// Rootless Podman socket, relative to the user runtime directory.
const PODMAN_ROOTLESS_SOCKET: &str = "podman/podman.sock";
/// Rootful Podman socket on Linux.
const PODMAN_SYSTEM_SOCKET: &str = "/run/podman/podman.sock";
/// Podman 5 machine API socket on macOS, relative to `$TMPDIR`.
const PODMAN_5_MACHINE_SOCKET: &str = "podman/podman-machine-default-api.sock";
/// Podman 4 machine API socket links on macOS, relative to `$HOME`.
const PODMAN_4_MACHINE_SOCKETS: [&str; 2] = [
    ".local/share/containers/podman/machine/podman.sock",
    ".local/share/containers/podman/machine/qemu/podman.sock",
];
/// Podman falls back to `/tmp` when `$TMPDIR` is not set.
const DEFAULT_TMPDIR: &str = "/tmp";

/// The operating system the search runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    Linux,
    MacOs,
}

impl Platform {
    /// The platform this binary was built for.
    pub(crate) fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

/// The inputs of the socket search, read once from the process.
#[derive(Debug, Clone)]
pub(crate) struct DiscoveryEnv {
    pub(crate) platform: Platform,
    pub(crate) docker_host: Option<String>,
    pub(crate) xdg_runtime_dir: Option<PathBuf>,
    pub(crate) home: Option<PathBuf>,
    pub(crate) tmpdir: Option<PathBuf>,
    pub(crate) uid: Option<u32>,
}

impl DiscoveryEnv {
    /// Read the search inputs from the environment of this process.
    pub(crate) fn from_process() -> Self {
        Self {
            platform: Platform::current(),
            docker_host: non_empty_var("DOCKER_HOST"),
            xdg_runtime_dir: non_empty_var("XDG_RUNTIME_DIR")
                .map(PathBuf::from),
            home: non_empty_var("HOME").map(PathBuf::from),
            tmpdir: non_empty_var("TMPDIR").map(PathBuf::from),
            uid: current_uid(),
        }
    }
}

/// Which step of the search order produced a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    /// The `DOCKER_HOST` environment variable.
    DockerHost,
    /// A well-known Docker socket.
    Docker,
    /// A well-known Podman socket.
    Podman,
}

/// One place where a local engine may listen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketCandidate {
    address: EngineAddress,
    source: CandidateSource,
}

impl SocketCandidate {
    /// Where to connect.
    pub fn address(&self) -> &EngineAddress {
        &self.address
    }

    /// Which step of the search order found this place.
    pub fn source(&self) -> CandidateSource {
        self.source
    }

    /// Create a candidate.
    pub(crate) fn new(address: EngineAddress, source: CandidateSource) -> Self {
        Self { address, source }
    }
}

/// Build the ordered search list (spec Decision 8).
///
/// Order: `$DOCKER_HOST`, the Docker sockets, the Podman sockets. The same
/// address is listed once, at its first place.
pub(crate) fn socket_candidates(env: &DiscoveryEnv) -> Vec<SocketCandidate> {
    let mut candidates = Vec::new();

    if let Some(address) =
        env.docker_host.as_deref().and_then(docker_host_address)
    {
        push_unique(&mut candidates, address, CandidateSource::DockerHost);
    }

    for path in docker_socket_paths(env) {
        let address = EngineAddress::Unix(path);
        push_unique(&mut candidates, address, CandidateSource::Docker);
    }

    for path in podman_socket_paths(env) {
        let address = EngineAddress::Unix(path);
        push_unique(&mut candidates, address, CandidateSource::Podman);
    }

    candidates
}

/// Drop candidates that lead to a socket already listed, for example
/// `/var/run/docker.sock` and `/run/docker.sock` when `/var/run` links to
/// `/run`. `resolve` follows links; the first candidate keeps its place.
pub(crate) fn drop_same_sockets(
    candidates: Vec<SocketCandidate>,
    resolve: impl Fn(&Path) -> PathBuf,
) -> Vec<SocketCandidate> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut kept = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        if let EngineAddress::Unix(path) = candidate.address() {
            let resolved = resolve(path);
            if seen.contains(&resolved) {
                continue;
            }
            seen.push(resolved);
        }
        kept.push(candidate);
    }

    kept
}

/// Read a `DOCKER_HOST` value. Only local kinds are supported: `unix://`
/// sockets and plain `tcp://` (or `http://`) addresses.
fn docker_host_address(value: &str) -> Option<EngineAddress> {
    if let Some(path) = value.strip_prefix("unix://") {
        if path.is_empty() {
            return None;
        }
        return Some(EngineAddress::Unix(PathBuf::from(path)));
    }

    let host = value
        .strip_prefix("tcp://")
        .or_else(|| value.strip_prefix("http://"))?
        .trim_end_matches('/');
    if host.is_empty() {
        return None;
    }

    Some(EngineAddress::Tcp(String::from(host)))
}

/// Docker sockets: the default one, then Docker Desktop's on macOS.
fn docker_socket_paths(env: &DiscoveryEnv) -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(DOCKER_SOCKET)];

    if env.platform == Platform::MacOs
        && let Some(home) = &env.home
    {
        paths.push(home.join(DOCKER_DESKTOP_SOCKET));
    }

    paths
}

/// Podman sockets: rootless first, then rootful on Linux or the default
/// Podman machine on macOS.
fn podman_socket_paths(env: &DiscoveryEnv) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(runtime_dir) = &env.xdg_runtime_dir {
        paths.push(runtime_dir.join(PODMAN_ROOTLESS_SOCKET));
    }

    match env.platform {
        Platform::Linux => {
            if let Some(uid) = env.uid {
                let run_user = PathBuf::from(format!("/run/user/{uid}"));
                paths.push(run_user.join(PODMAN_ROOTLESS_SOCKET));
            }
            paths.push(PathBuf::from(PODMAN_SYSTEM_SOCKET));
        },
        Platform::MacOs => {
            let tmpdir =
                env.tmpdir.as_deref().unwrap_or(Path::new(DEFAULT_TMPDIR));
            paths.push(tmpdir.join(PODMAN_5_MACHINE_SOCKET));

            if let Some(home) = &env.home {
                for socket in PODMAN_4_MACHINE_SOCKETS {
                    paths.push(home.join(socket));
                }
            }
        },
    }

    paths
}

/// Add a candidate unless its address is already listed.
fn push_unique(
    candidates: &mut Vec<SocketCandidate>,
    address: EngineAddress,
    source: CandidateSource,
) {
    if candidates
        .iter()
        .any(|candidate| candidate.address == address)
    {
        return;
    }

    candidates.push(SocketCandidate::new(address, source));
}

fn non_empty_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The user id, needed only for `/run/user/$UID` on Linux.
fn current_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;

    std::fs::metadata("/proc/self")
        .map(|metadata| metadata.uid())
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux_env() -> DiscoveryEnv {
        DiscoveryEnv {
            platform: Platform::Linux,
            docker_host: None,
            xdg_runtime_dir: Some(PathBuf::from("/run/user/1000")),
            home: Some(PathBuf::from("/home/ann")),
            tmpdir: None,
            uid: Some(1000),
        }
    }

    fn macos_env() -> DiscoveryEnv {
        DiscoveryEnv {
            platform: Platform::MacOs,
            docker_host: None,
            xdg_runtime_dir: None,
            home: Some(PathBuf::from("/Users/ann")),
            tmpdir: Some(PathBuf::from("/var/folders/xy/abc/T/")),
            uid: None,
        }
    }

    fn unix(path: &str, source: CandidateSource) -> SocketCandidate {
        SocketCandidate::new(EngineAddress::Unix(PathBuf::from(path)), source)
    }

    fn tcp(host: &str) -> SocketCandidate {
        SocketCandidate::new(
            EngineAddress::Tcp(String::from(host)),
            CandidateSource::DockerHost,
        )
    }

    #[test]
    fn linux_without_docker_host_lists_docker_then_podman() {
        let env = DiscoveryEnv {
            xdg_runtime_dir: Some(PathBuf::from("/tmp/runtime-ann")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates,
            vec![
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix(
                    "/tmp/runtime-ann/podman/podman.sock",
                    CandidateSource::Podman
                ),
                unix(
                    "/run/user/1000/podman/podman.sock",
                    CandidateSource::Podman
                ),
                unix("/run/podman/podman.sock", CandidateSource::Podman),
            ]
        );
    }

    #[test]
    fn linux_runtime_dir_equal_to_run_user_is_listed_once() {
        let candidates = socket_candidates(&linux_env());

        assert_eq!(
            candidates,
            vec![
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix(
                    "/run/user/1000/podman/podman.sock",
                    CandidateSource::Podman
                ),
                unix("/run/podman/podman.sock", CandidateSource::Podman),
            ]
        );
    }

    #[test]
    fn linux_runtime_dir_with_trailing_slash_is_listed_once() {
        let env = DiscoveryEnv {
            xdg_runtime_dir: Some(PathBuf::from("/run/user/1000/")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(candidates.len(), 3);
    }

    #[test]
    fn linux_without_runtime_dir_and_uid_keeps_the_system_sockets() {
        let env = DiscoveryEnv {
            xdg_runtime_dir: None,
            uid: None,
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates,
            vec![
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix("/run/podman/podman.sock", CandidateSource::Podman),
            ]
        );
    }

    #[test]
    fn linux_without_runtime_dir_uses_the_uid() {
        let env = DiscoveryEnv {
            xdg_runtime_dir: None,
            uid: Some(1001),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates[1],
            unix("/run/user/1001/podman/podman.sock", CandidateSource::Podman)
        );
    }

    #[test]
    fn unix_docker_host_comes_first() {
        let env = DiscoveryEnv {
            docker_host: Some(String::from("unix:///home/ann/.docker.sock")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates[0],
            unix("/home/ann/.docker.sock", CandidateSource::DockerHost)
        );
        assert_eq!(candidates.len(), 4);
    }

    #[test]
    fn tcp_docker_host_comes_first() {
        let env = DiscoveryEnv {
            docker_host: Some(String::from("tcp://127.0.0.1:2375")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(candidates[0], tcp("127.0.0.1:2375"));
        assert_eq!(candidates.len(), 4);
    }

    #[test]
    fn http_docker_host_is_read_as_tcp() {
        let env = DiscoveryEnv {
            docker_host: Some(String::from("http://localhost:2375/")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(candidates[0], tcp("localhost:2375"));
    }

    #[test]
    fn docker_host_equal_to_a_default_socket_is_listed_once() {
        let env = DiscoveryEnv {
            docker_host: Some(String::from("unix:///var/run/docker.sock")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates,
            vec![
                unix("/var/run/docker.sock", CandidateSource::DockerHost),
                unix(
                    "/run/user/1000/podman/podman.sock",
                    CandidateSource::Podman
                ),
                unix("/run/podman/podman.sock", CandidateSource::Podman),
            ]
        );
    }

    #[test]
    fn remote_or_unknown_docker_host_is_skipped() {
        for value in
            ["ssh://ann@build", "npipe:////./pipe/docker", "unix://", ""]
        {
            let env = DiscoveryEnv {
                docker_host: Some(String::from(value)),
                ..linux_env()
            };

            let candidates = socket_candidates(&env);

            assert_eq!(candidates.len(), 3, "DOCKER_HOST={value}");
            assert_eq!(candidates[0].source(), CandidateSource::Docker);
        }
    }

    #[test]
    fn macos_lists_docker_desktop_and_podman_machine_sockets() {
        let candidates = socket_candidates(&macos_env());

        assert_eq!(
            candidates,
            vec![
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix(
                    "/Users/ann/.docker/run/docker.sock",
                    CandidateSource::Docker
                ),
                unix(
                    "/var/folders/xy/abc/T/podman/podman-machine-default-api.sock",
                    CandidateSource::Podman
                ),
                unix(
                    "/Users/ann/.local/share/containers/podman/machine/podman.sock",
                    CandidateSource::Podman
                ),
                unix(
                    "/Users/ann/.local/share/containers/podman/machine/qemu/podman.sock",
                    CandidateSource::Podman
                ),
            ]
        );
    }

    #[test]
    fn macos_without_tmpdir_uses_tmp_like_podman() {
        let env = DiscoveryEnv {
            tmpdir: None,
            ..macos_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates[2],
            unix(
                "/tmp/podman/podman-machine-default-api.sock",
                CandidateSource::Podman
            )
        );
    }

    #[test]
    fn macos_with_runtime_dir_checks_it_before_the_machine() {
        let env = DiscoveryEnv {
            xdg_runtime_dir: Some(PathBuf::from("/Users/ann/.run")),
            ..macos_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates[2],
            unix(
                "/Users/ann/.run/podman/podman.sock",
                CandidateSource::Podman
            )
        );
        assert_eq!(candidates.len(), 6);
    }

    #[test]
    fn macos_without_home_skips_the_home_sockets() {
        let env = DiscoveryEnv {
            home: None,
            ..macos_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates,
            vec![
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix(
                    "/var/folders/xy/abc/T/podman/podman-machine-default-api.sock",
                    CandidateSource::Podman
                ),
            ]
        );
    }

    #[test]
    fn same_socket_through_a_link_is_kept_once_at_its_first_place() {
        let candidates = vec![
            unix("/var/run/docker.sock", CandidateSource::Docker),
            unix("/run/user/1000/podman/podman.sock", CandidateSource::Podman),
            unix("/run/docker.sock", CandidateSource::Podman),
            tcp("localhost:2375"),
        ];
        let resolve = |path: &Path| match path.strip_prefix("/var/run") {
            Ok(rest) => Path::new("/run").join(rest),
            Err(_) => path.to_path_buf(),
        };

        let kept = drop_same_sockets(candidates, resolve);

        assert_eq!(
            kept,
            vec![
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix(
                    "/run/user/1000/podman/podman.sock",
                    CandidateSource::Podman
                ),
                tcp("localhost:2375"),
            ]
        );
    }
}
