//! Runs the architecture check on the real `app/src` against the allowlist.

use std::path::Path;

use otty_xtask::architecture::allowlist::ALLOWLIST;
use otty_xtask::architecture::report::{Output, render};
use otty_xtask::architecture::{Comparison, check};

#[test]
fn the_app_follows_its_architecture() {
    let app_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/src");

    let findings = check(&app_src).expect("app/src can be read");
    let comparison = Comparison::compare(&findings, ALLOWLIST);

    assert!(
        comparison.is_clean(),
        "{}",
        render(&comparison, Output::Plain)
    );
}
