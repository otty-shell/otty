/// Read this computer's host name, such as "fedora". Returns `None` when it
/// cannot be read; the status bar then calls the machine "This computer".
pub(crate) fn read_host_name() -> Option<String> {
    #[cfg(unix)]
    {
        match nix::unistd::gethostname() {
            Ok(name) => name.into_string().ok().filter(|name| !name.is_empty()),
            Err(err) => {
                log::warn!("host name read failed: {err}");
                None
            },
        }
    }

    #[cfg(not(unix))]
    {
        None
    }
}
