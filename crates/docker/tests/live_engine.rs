//! Runs discovery and the list calls against the engines of this machine.
//!
//! Needs a running Docker or Podman (with its API socket on). Run with
//! `cargo test -p otty-docker --test live_engine -- --ignored --nocapture`.
//! For a Podman service on another socket, set
//! `DOCKER_HOST=unix:///path/to/podman.sock`.

use otty_docker::{Diagnosis, Engine, discover};

#[tokio::test]
#[ignore = "needs a running Docker or Podman engine"]
async fn discovery_reaches_a_live_engine_and_lists_its_resources() {
    let discovery = discover().await;

    for probe in discovery.probes() {
        let address = probe.candidate().address();
        let outcome = probe.outcome();
        println!("{address}: {outcome:?}");
    }
    let Diagnosis::Engines(engines) = discovery.diagnosis() else {
        panic!("no engine reached: {:?}", discovery.diagnosis());
    };
    assert!(!engines.is_empty());
    assert!(engines.iter().all(|engine| engine.version().is_some()));

    for info in engines {
        let engine = Engine::connect(info).expect("connect");

        let resources = engine.resources().await.expect("list resources");

        let kind = info.kind();
        let address = info.address();
        let containers = resources.containers().len();
        let images = resources.images().len();
        let volumes = resources.volumes().len();
        let networks = resources.networks().len();
        println!(
            "{kind:?} at {address}: {containers} containers, {images} images, {volumes} volumes, {networks} networks"
        );
        assert!(
            resources
                .networks()
                .iter()
                .any(|network| network.is_built_in()),
            "every engine has a built-in network"
        );
        assert!(
            resources
                .containers()
                .iter()
                .all(|container| !container.id().is_empty())
        );
    }
}
