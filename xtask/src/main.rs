//! `cargo xtask`: development tools for OTTY.
//!
//! Commands:
//! - `arch` prints the architecture report of the `app` crate and exits
//!   with 1 when a place is new or an allowlist entry is stale.

use std::io::IsTerminal;
use std::path::Path;
use std::process::ExitCode;

use otty_xtask::architecture::allowlist::ALLOWLIST;
use otty_xtask::architecture::report::{self, Output};
use otty_xtask::architecture::{Comparison, check};

const USAGE: &str = "usage: cargo xtask <command>

commands:
  arch    print the architecture report of the app crate
";

fn main() -> ExitCode {
    let command = std::env::args().nth(1);

    match command.as_deref() {
        Some("arch") => architecture_report(),
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        },
    }
}

/// Check `app/src`, print the report and return its exit code.
fn architecture_report() -> ExitCode {
    let app_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/src");

    let findings = match check(&app_src) {
        Ok(findings) => findings,
        Err(err) => {
            let path = app_src.display();
            eprintln!("cannot read {path}: {err}");
            return ExitCode::from(2);
        },
    };
    let comparison = Comparison::compare(&findings, ALLOWLIST);

    let output = if std::io::stdout().is_terminal() {
        Output::Terminal
    } else {
        Output::Plain
    };
    print!("{}", report::render(&comparison, output));

    ExitCode::from(report::exit_code(&comparison))
}
