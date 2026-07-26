use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use otty_libterm::pty::SSHAuth;
use otty_ui_term_gpui::{
    BackendGeneration, BackendState, LocalBackend, LocalOptions, SshBackend,
    SshOptions,
};

#[test]
fn backend_generation_is_monotonic() {
    let first = BackendGeneration::initial();
    let second = first.next();

    assert_eq!(first.value(), 1);
    assert_eq!(second.value(), 2);
    assert_ne!(first, second);
    assert!(matches!(BackendState::Starting, BackendState::Starting));
}

#[test]
fn local_backend_preserves_launch_options() {
    let options = LocalOptions::new("/bin/sh")
        .with_arg("-l")
        .with_env("TERM", "xterm-256color")
        .with_working_directory(PathBuf::from("/tmp"));
    let backend = LocalBackend::new(options);

    assert_eq!(backend.options().program(), "/bin/sh");
    assert_eq!(backend.options().args(), ["-l"]);
    assert_eq!(backend.options().envs(), [("TERM", "xterm-256color")]);
    assert_eq!(
        backend.options().working_directory(),
        Some(PathBuf::from("/tmp").as_path())
    );
}

#[test]
fn default_local_backend_uses_the_portable_system_shell() {
    assert_eq!(LocalOptions::default().program(), "/bin/sh");
}

#[test]
fn ssh_backend_preserves_connection_options() {
    let options = SshOptions::new(
        "example.test:22",
        "user",
        SSHAuth::Password("secret".to_string()),
    )
    .with_timeout(Duration::from_secs(3));
    let backend = SshBackend::new(options);

    assert_eq!(backend.options().host(), "example.test:22");
    assert_eq!(backend.options().user(), "user");
    assert_eq!(backend.options().timeout(), Some(Duration::from_secs(3)));
    assert!(
        matches!(backend.options().auth(), SSHAuth::Password(value) if value == "secret")
    );
}

#[test]
fn backend_options_replace_environment_and_preserve_cancellation() {
    let local = LocalBackend::new(
        LocalOptions::new("/bin/sh")
            .with_env("TERM", "xterm")
            .with_env("TERM", "xterm-256color"),
    );
    let cancel = Arc::new(AtomicBool::new(false));
    let ssh = SshBackend::new(
        SshOptions::new(
            "example.test:22",
            "user",
            SSHAuth::Password("secret".to_string()),
        )
        .with_cancel_token(Arc::clone(&cancel)),
    );

    assert_eq!(local.options().envs(), [("TERM", "xterm-256color")]);
    let stored = ssh.options().cancel_token().expect("cancel token");
    stored.store(true, Ordering::Relaxed);
    assert!(cancel.load(Ordering::Relaxed));
}
