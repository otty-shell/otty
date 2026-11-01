//! Runs discovery against the engines of this machine.
//!
//! Needs a running Docker or Podman (with its API socket on). Run with
//! `cargo test -p otty-docker --test live_engine -- --ignored --nocapture`.

use otty_docker::{Diagnosis, discover};

#[tokio::test]
#[ignore = "needs a running Docker or Podman engine"]
async fn discovery_reaches_a_live_engine() {
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
}
