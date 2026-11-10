//! The ordered list of places where a local engine may listen.
//!
//! The list is built from plain inputs (environment, platform, user id), so
//! the search order is tested without touching the file system.

use std::path::{Path, PathBuf};

use crate::address::EngineAddress;
use crate::socket_candidate::{CandidateSource, SocketCandidate};

/// Docker's default socket on Linux and macOS.
const DOCKER_SOCKET: &str = "/var/run/docker.sock";
/// Docker Desktop's per-user socket on macOS.
const DOCKER_DESKTOP_MAC_SOCKET: &str = ".docker/run/docker.sock";
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
/// Docker Desktop's per-user socket on Linux, relative to `$HOME`.
const DOCKER_DESKTOP_LINUX_SOCKET: &str = ".docker/desktop/docker.sock";
/// Colima's socket for its default profile (Colima 0.4.0 and newer),
/// relative to Colima's base folder (see [`colima_dir`]).
const COLIMA_SOCKET: &str = "default/docker.sock";
/// Colima's older base folder, relative to `$HOME`; still used when it
/// exists, and on macOS by a fresh install.
const COLIMA_HOME_DIR: &str = ".colima";
/// Colima's base folder inside the config folder (`$XDG_CONFIG_HOME` or
/// `~/.config`); a fresh install on Linux uses it.
const COLIMA_CONFIG_DIR: &str = "colima";
/// The config folder when `$XDG_CONFIG_HOME` is not set, relative to
/// `$HOME`.
const DEFAULT_CONFIG_DIR: &str = ".config";
/// OrbStack's socket on macOS, relative to `$HOME`.
const ORBSTACK_SOCKET: &str = ".orbstack/run/docker.sock";

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
    /// Colima's base folder, chosen as Colima chooses it.
    pub(crate) colima_dir: Option<PathBuf>,
}

impl DiscoveryEnv {
    /// Read the search inputs from the environment of this process.
    pub(crate) fn from_process() -> Self {
        let platform = Platform::current();
        let home = non_empty_var("HOME").map(PathBuf::from);
        let colima_home = non_empty_var("COLIMA_HOME").map(PathBuf::from);
        let xdg_config_home =
            non_empty_var("XDG_CONFIG_HOME").map(PathBuf::from);

        let colima_dir = colima_dir(
            &ColimaInputs {
                platform,
                colima_home: colima_home.as_deref(),
                home: home.as_deref(),
                xdg_config_home: xdg_config_home.as_deref(),
            },
            Path::exists,
        );

        Self {
            platform,
            docker_host: non_empty_var("DOCKER_HOST"),
            xdg_runtime_dir: non_empty_var("XDG_RUNTIME_DIR")
                .map(PathBuf::from),
            home,
            tmpdir: non_empty_var("TMPDIR").map(PathBuf::from),
            uid: current_uid(),
            colima_dir,
        }
    }
}

/// What Colima reads to choose its base folder.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColimaInputs<'a> {
    pub(crate) platform: Platform,
    pub(crate) colima_home: Option<&'a Path>,
    pub(crate) home: Option<&'a Path>,
    pub(crate) xdg_config_home: Option<&'a Path>,
}

/// Build the ordered search list (spec Decision 8).
///
/// Order: `$DOCKER_HOST`, the Docker sockets, the Podman sockets, the other
/// Docker-compatible engines. The same address is listed once, at its first
/// place.
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

    for path in compatible_socket_paths(env) {
        let address = EngineAddress::Unix(path);
        push_unique(&mut candidates, address, CandidateSource::Docker);
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

/// Colima's base folder, in Colima's own order (`config/files.go`,
/// `configBaseDir`): `$COLIMA_HOME` if it exists; else `~/.colima` if it
/// exists; else `$XDG_CONFIG_HOME/colima` when that variable is set; else
/// `~/.config/colima` if it exists; else `~/.colima` on macOS and
/// `~/.config/colima` on Linux. `exists` checks the file system.
pub(crate) fn colima_dir(
    inputs: &ColimaInputs<'_>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if let Some(colima_home) = inputs.colima_home
        && exists(colima_home)
    {
        return Some(colima_home.to_path_buf());
    }

    let dot_colima = inputs.home.map(|home| home.join(COLIMA_HOME_DIR));
    if let Some(dot_colima) = &dot_colima
        && exists(dot_colima)
    {
        return Some(dot_colima.clone());
    }

    if let Some(config) = inputs.xdg_config_home {
        return Some(config.join(COLIMA_CONFIG_DIR));
    }

    let config_colima = inputs
        .home
        .map(|home| home.join(DEFAULT_CONFIG_DIR).join(COLIMA_CONFIG_DIR));
    if let Some(config_colima) = &config_colima
        && exists(config_colima)
    {
        return Some(config_colima.clone());
    }

    match inputs.platform {
        Platform::MacOs => dot_colima,
        Platform::Linux => config_colima,
    }
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
        paths.push(home.join(DOCKER_DESKTOP_MAC_SOCKET));
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

/// Other engines that serve the Docker API and show as Docker: Docker
/// Desktop on Linux and Colima, or Colima and OrbStack on macOS. Docker
/// Desktop and OrbStack live under `$HOME`; Colima in its base folder.
fn compatible_socket_paths(env: &DiscoveryEnv) -> Vec<PathBuf> {
    let home = env.home.as_deref();
    let colima = env.colima_dir.as_deref();
    let mut paths = Vec::new();

    if env.platform == Platform::Linux
        && let Some(home) = home
    {
        paths.push(home.join(DOCKER_DESKTOP_LINUX_SOCKET));
    }
    if let Some(colima) = colima {
        paths.push(colima.join(COLIMA_SOCKET));
    }
    if env.platform == Platform::MacOs
        && let Some(home) = home
    {
        paths.push(home.join(ORBSTACK_SOCKET));
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
        .any(|candidate| *candidate.address() == address)
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
            colima_dir: Some(PathBuf::from("/home/ann/.config/colima")),
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
            colima_dir: Some(PathBuf::from("/Users/ann/.colima")),
        }
    }

    fn unix(path: &str, source: CandidateSource) -> SocketCandidate {
        SocketCandidate::new(EngineAddress::Unix(PathBuf::from(path)), source)
    }

    fn linux_compatible_sockets() -> Vec<SocketCandidate> {
        vec![
            unix(
                "/home/ann/.docker/desktop/docker.sock",
                CandidateSource::Docker,
            ),
            unix(
                "/home/ann/.config/colima/default/docker.sock",
                CandidateSource::Docker,
            ),
        ]
    }

    fn with_linux_compatible_sockets(
        mut candidates: Vec<SocketCandidate>,
    ) -> Vec<SocketCandidate> {
        candidates.extend(linux_compatible_sockets());
        candidates
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
                unix(
                    "/home/ann/.docker/desktop/docker.sock",
                    CandidateSource::Docker
                ),
                unix(
                    "/home/ann/.config/colima/default/docker.sock",
                    CandidateSource::Docker
                ),
            ]
        );
    }

    #[test]
    fn linux_runtime_dir_equal_to_run_user_is_listed_once() {
        let candidates = socket_candidates(&linux_env());

        assert_eq!(
            candidates,
            with_linux_compatible_sockets(vec![
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix(
                    "/run/user/1000/podman/podman.sock",
                    CandidateSource::Podman
                ),
                unix("/run/podman/podman.sock", CandidateSource::Podman),
            ])
        );
    }

    #[test]
    fn linux_runtime_dir_with_trailing_slash_is_listed_once() {
        let env = DiscoveryEnv {
            xdg_runtime_dir: Some(PathBuf::from("/run/user/1000/")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(candidates.len(), 5);
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
            with_linux_compatible_sockets(vec![
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix("/run/podman/podman.sock", CandidateSource::Podman),
            ])
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
        assert_eq!(candidates.len(), 6);
    }

    #[test]
    fn tcp_docker_host_comes_first() {
        let env = DiscoveryEnv {
            docker_host: Some(String::from("tcp://127.0.0.1:2375")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(candidates[0], tcp("127.0.0.1:2375"));
        assert_eq!(candidates.len(), 6);
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
            with_linux_compatible_sockets(vec![
                unix("/var/run/docker.sock", CandidateSource::DockerHost),
                unix(
                    "/run/user/1000/podman/podman.sock",
                    CandidateSource::Podman
                ),
                unix("/run/podman/podman.sock", CandidateSource::Podman),
            ])
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

            assert_eq!(candidates.len(), 5, "DOCKER_HOST={value}");
            assert_eq!(candidates[0].source(), CandidateSource::Docker);
        }
    }

    #[test]
    fn macos_lists_docker_desktop_podman_machine_colima_and_orbstack() {
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
                unix(
                    "/Users/ann/.colima/default/docker.sock",
                    CandidateSource::Docker
                ),
                unix(
                    "/Users/ann/.orbstack/run/docker.sock",
                    CandidateSource::Docker
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
        assert_eq!(candidates.len(), 8);
    }

    #[test]
    fn macos_without_home_skips_the_home_sockets() {
        let env = DiscoveryEnv {
            home: None,
            colima_dir: None,
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
    fn linux_without_home_skips_docker_desktop_and_colima() {
        let env = DiscoveryEnv {
            home: None,
            colima_dir: None,
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

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
    fn linux_does_not_list_orbstack() {
        let candidates = socket_candidates(&linux_env());

        assert!(candidates.iter().all(|candidate| {
            !candidate.address().to_string().contains(".orbstack")
        }));
    }

    #[test]
    fn macos_does_not_list_docker_desktop_for_linux() {
        let candidates = socket_candidates(&macos_env());

        assert!(candidates.iter().all(|candidate| {
            !candidate.address().to_string().contains(".docker/desktop")
        }));
    }

    #[test]
    fn colima_socket_in_docker_host_is_listed_once_first() {
        let env = DiscoveryEnv {
            docker_host: Some(String::from(
                "unix:///home/ann/.config/colima/default/docker.sock",
            )),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates,
            vec![
                unix(
                    "/home/ann/.config/colima/default/docker.sock",
                    CandidateSource::DockerHost
                ),
                unix("/var/run/docker.sock", CandidateSource::Docker),
                unix(
                    "/run/user/1000/podman/podman.sock",
                    CandidateSource::Podman
                ),
                unix("/run/podman/podman.sock", CandidateSource::Podman),
                unix(
                    "/home/ann/.docker/desktop/docker.sock",
                    CandidateSource::Docker
                ),
            ]
        );
    }

    #[test]
    fn orbstack_socket_in_docker_host_is_listed_once_first() {
        let env = DiscoveryEnv {
            docker_host: Some(String::from(
                "unix:///Users/ann/.orbstack/run/docker.sock",
            )),
            ..macos_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates[0],
            unix(
                "/Users/ann/.orbstack/run/docker.sock",
                CandidateSource::DockerHost
            )
        );
        assert_eq!(candidates.len(), 7);
    }

    #[test]
    fn default_socket_linked_to_orbstack_is_tried_once() {
        let candidates = socket_candidates(&macos_env());
        let resolve = |path: &Path| {
            if path == Path::new("/var/run/docker.sock") {
                PathBuf::from("/Users/ann/.orbstack/run/docker.sock")
            } else {
                path.to_path_buf()
            }
        };

        let kept = drop_same_sockets(candidates, resolve);

        assert_eq!(kept.len(), 6);
        assert_eq!(
            kept[0],
            unix("/var/run/docker.sock", CandidateSource::Docker)
        );
        assert!(kept.iter().all(|candidate| {
            !candidate.address().to_string().contains(".orbstack")
        }));
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

    // Colima's base folder (ticket 21 follow-up).

    const LINUX_HOME: &str = "/home/ann";
    const MAC_HOME: &str = "/Users/ann";

    fn colima_inputs(platform: Platform, home: &str) -> ColimaInputs<'_> {
        ColimaInputs {
            platform,
            colima_home: None,
            home: Some(Path::new(home)),
            xdg_config_home: None,
        }
    }

    /// A file system where only `existing` folders are there.
    fn only<'a>(existing: &'a [&'a str]) -> impl Fn(&Path) -> bool + 'a {
        move |path: &Path| existing.iter().any(|dir| path == Path::new(dir))
    }

    #[test]
    fn a_fresh_colima_on_linux_uses_the_config_folder() {
        let inputs = colima_inputs(Platform::Linux, LINUX_HOME);

        let dir = colima_dir(&inputs, only(&[]));

        assert_eq!(dir, Some(PathBuf::from("/home/ann/.config/colima")));
    }

    #[test]
    fn a_fresh_colima_on_macos_uses_dot_colima() {
        let inputs = colima_inputs(Platform::MacOs, MAC_HOME);

        let dir = colima_dir(&inputs, only(&[]));

        assert_eq!(dir, Some(PathBuf::from("/Users/ann/.colima")));
    }

    #[test]
    fn an_existing_dot_colima_wins_over_the_config_folder() {
        let inputs = ColimaInputs {
            xdg_config_home: Some(Path::new("/home/ann/.xdg")),
            ..colima_inputs(Platform::Linux, LINUX_HOME)
        };

        let dir = colima_dir(
            &inputs,
            only(&["/home/ann/.colima", "/home/ann/.xdg/colima"]),
        );

        assert_eq!(dir, Some(PathBuf::from("/home/ann/.colima")));
    }

    #[test]
    fn xdg_config_home_is_used_when_dot_colima_is_missing() {
        for platform in [Platform::Linux, Platform::MacOs] {
            let inputs = ColimaInputs {
                xdg_config_home: Some(Path::new("/home/ann/.xdg")),
                ..colima_inputs(platform, LINUX_HOME)
            };

            let dir = colima_dir(&inputs, only(&[]));

            assert_eq!(dir, Some(PathBuf::from("/home/ann/.xdg/colima")));
        }
    }

    #[test]
    fn an_existing_config_folder_is_used_on_macos_too() {
        let inputs = colima_inputs(Platform::MacOs, MAC_HOME);

        let dir = colima_dir(&inputs, only(&["/Users/ann/.config/colima"]));

        assert_eq!(dir, Some(PathBuf::from("/Users/ann/.config/colima")));
    }

    #[test]
    fn an_existing_colima_home_overrides_both() {
        let inputs = ColimaInputs {
            colima_home: Some(Path::new("/srv/colima")),
            xdg_config_home: Some(Path::new("/home/ann/.xdg")),
            ..colima_inputs(Platform::Linux, LINUX_HOME)
        };

        let dir =
            colima_dir(&inputs, only(&["/srv/colima", "/home/ann/.colima"]));

        assert_eq!(dir, Some(PathBuf::from("/srv/colima")));
    }

    #[test]
    fn a_missing_colima_home_is_passed_over() {
        let inputs = ColimaInputs {
            colima_home: Some(Path::new("/srv/colima")),
            ..colima_inputs(Platform::Linux, LINUX_HOME)
        };

        let dir = colima_dir(&inputs, only(&["/home/ann/.colima"]));

        assert_eq!(dir, Some(PathBuf::from("/home/ann/.colima")));
    }

    #[test]
    fn without_home_only_colima_home_or_xdg_config_home_count() {
        let no_home = ColimaInputs {
            home: None,
            ..colima_inputs(Platform::Linux, LINUX_HOME)
        };
        let with_colima_home = ColimaInputs {
            colima_home: Some(Path::new("/srv/colima")),
            ..no_home
        };
        let with_xdg = ColimaInputs {
            xdg_config_home: Some(Path::new("/etc/xdg")),
            ..no_home
        };

        assert_eq!(colima_dir(&no_home, only(&[])), None);
        assert_eq!(
            colima_dir(&with_colima_home, only(&["/srv/colima"])),
            Some(PathBuf::from("/srv/colima"))
        );
        assert_eq!(
            colima_dir(&with_xdg, only(&[])),
            Some(PathBuf::from("/etc/xdg/colima"))
        );
    }

    #[test]
    fn colima_in_its_base_folder_is_listed_after_docker_desktop_on_linux() {
        let env = DiscoveryEnv {
            colima_dir: Some(PathBuf::from("/srv/colima")),
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates[3..],
            [
                unix(
                    "/home/ann/.docker/desktop/docker.sock",
                    CandidateSource::Docker
                ),
                unix(
                    "/srv/colima/default/docker.sock",
                    CandidateSource::Docker
                ),
            ]
        );
    }

    #[test]
    fn colima_in_its_base_folder_is_listed_before_orbstack_on_macos() {
        let env = DiscoveryEnv {
            colima_dir: Some(PathBuf::from("/Users/ann/.config/colima")),
            ..macos_env()
        };

        let candidates = socket_candidates(&env);

        assert_eq!(
            candidates[5..],
            [
                unix(
                    "/Users/ann/.config/colima/default/docker.sock",
                    CandidateSource::Docker
                ),
                unix(
                    "/Users/ann/.orbstack/run/docker.sock",
                    CandidateSource::Docker
                ),
            ]
        );
    }

    #[test]
    fn without_a_colima_folder_colima_is_not_listed() {
        let env = DiscoveryEnv {
            colima_dir: None,
            ..linux_env()
        };

        let candidates = socket_candidates(&env);

        assert!(candidates.iter().all(|candidate| {
            !candidate.address().to_string().contains("colima")
        }));
        assert_eq!(candidates.len(), 4);
    }
}
