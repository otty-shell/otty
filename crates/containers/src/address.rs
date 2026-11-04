//! Where an engine listens: a unix socket file or a tcp address.

use std::fmt;
use std::path::PathBuf;

/// Where a Docker or Podman engine listens.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_containers::EngineAddress;
///
/// let address = EngineAddress::Unix(PathBuf::from("/var/run/docker.sock"));
/// assert_eq!(address.to_string(), "/var/run/docker.sock");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EngineAddress {
    /// A unix socket file.
    Unix(PathBuf),
    /// A tcp `host:port`, only from `DOCKER_HOST=tcp://…`.
    Tcp(String),
}

impl EngineAddress {
    /// The address in the form the `docker` and `podman` commands take:
    /// `unix:///path` or `tcp://host:port`. The plain path of
    /// [`fmt::Display`] does not work there: `docker` reads it as a tcp host.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_containers::EngineAddress;
    ///
    /// let address = EngineAddress::Unix(PathBuf::from("/run/docker.sock"));
    /// assert_eq!(address.host_url(), "unix:///run/docker.sock");
    /// ```
    pub fn host_url(&self) -> String {
        match self {
            Self::Unix(path) => format!("unix://{}", path.display()),
            Self::Tcp(host) => format!("tcp://{host}"),
        }
    }
}

impl fmt::Display for EngineAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unix(path) => write!(f, "{}", path.display()),
            Self::Tcp(host) => write!(f, "tcp://{host}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_address_shows_the_plain_path() {
        let address = EngineAddress::Unix(PathBuf::from("/run/docker.sock"));

        assert_eq!(address.to_string(), "/run/docker.sock");
    }

    #[test]
    fn tcp_address_shows_the_scheme() {
        let address = EngineAddress::Tcp(String::from("localhost:2375"));

        assert_eq!(address.to_string(), "tcp://localhost:2375");
    }

    #[test]
    fn unix_host_url_has_the_scheme_a_plain_path_lacks() {
        let address = EngineAddress::Unix(PathBuf::from(
            "/home/ann/.colima/default/docker.sock",
        ));

        assert_eq!(
            address.host_url(),
            "unix:///home/ann/.colima/default/docker.sock"
        );
    }

    #[test]
    fn tcp_host_url_keeps_the_scheme() {
        let address = EngineAddress::Tcp(String::from("localhost:2375"));

        assert_eq!(address.host_url(), "tcp://localhost:2375");
    }
}
