//! The data types without the engine client.
//!
//! Feature crates use `otty-containers` with `default-features = false`: they
//! get the data types and cannot reach an engine. These tests build only then;
//! run them with `cargo test -p otty-containers --no-default-features`. That
//! `Engine` and `discover` are not there is checked by the `compile_fail`
//! examples in the crate docs, which exist in the same build.

#![cfg(not(feature = "client"))]

use std::cmp::Ordering;
use std::path::PathBuf;

use otty_containers::{
    EngineAddress, ImageDefaults, ListKind, MAX_LOG_LINE_BYTES, MountSource,
    PortProtocol, PublishedPort, PullProgress, RestartPolicy, RunRequest,
    VolumeMount, WatchEvent, compare_text,
};

#[test]
fn a_run_request_is_built_without_the_client() {
    let request = RunRequest::new("nginx:1.27")
        .with_name("web")
        .with_port(PublishedPort::new(Some(8081), 80, PortProtocol::Tcp))
        .with_volume(VolumeMount::new(
            MountSource::Volume(String::from("data")),
            "/data",
            false,
        ))
        .with_restart(RestartPolicy::UnlessStopped);

    assert_eq!(request.image(), "nginx:1.27");
    assert_eq!(request.name(), Some("web"));
    assert_eq!(request.ports().len(), 1);
    assert_eq!(request.volumes().len(), 1);
    assert_eq!(request.restart(), RestartPolicy::UnlessStopped);
}

#[test]
fn an_engine_address_is_shown_without_the_client() {
    let address = EngineAddress::Unix(PathBuf::from("/run/docker.sock"));

    assert_eq!(address.to_string(), "/run/docker.sock");
    assert_eq!(address.host_url(), "unix:///run/docker.sock");
}

#[test]
fn names_are_compared_without_the_client() {
    assert_eq!(compare_text("alpha", "Beta"), Ordering::Less);
}

#[test]
fn empty_values_have_their_defaults_without_the_client() {
    let progress = PullProgress::default();
    let defaults = ImageDefaults::default();

    assert_eq!(progress.layer_count(), 0);
    assert_eq!(progress.fraction(), 0.0);
    assert!(defaults.ports().is_empty());
    assert_eq!(defaults.workdir(), None);
}

#[test]
fn watch_events_and_the_line_limit_are_there_without_the_client() {
    let event = WatchEvent::Changed(ListKind::Images);

    assert_eq!(event, WatchEvent::Changed(ListKind::Images));
    assert_eq!(MAX_LOG_LINE_BYTES, 64 * 1024);
}
