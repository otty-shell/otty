//! `cargo xtask`: development tools for OTTY.
//!
//! Commands:
//! - `arch` prints the architecture report of the `app` crate and exits
//!   with 1 when a place is new or an allowlist entry is stale.
//! - `new-widget <name>` creates the widget `name` in the `app` crate, with
//!   the files of the widget example in `docs/internal/architecture/app/widgets.md`,
//!   wires it in and prints what is left to do: showing it in a view.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use otty_xtask::architecture::allowlist::ALLOWLIST;
use otty_xtask::architecture::report::{self, Output};
use otty_xtask::architecture::{Comparison, check};
use otty_xtask::new_widget;

const USAGE: &str = "usage: cargo xtask <command>

commands:
  arch               print the architecture report of the app crate
  new-widget <name>  create a widget in the app crate and wire it in
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    match args.as_slice() {
        ["arch"] => architecture_report(),
        ["new-widget", name] => create_widget(name),
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        },
    }
}

/// Check `app/src`, print the report and return its exit code.
fn architecture_report() -> ExitCode {
    let app_src = app_src();

    let findings = match check(&app_src) {
        Ok(findings) => findings,
        Err(err) => {
            let path = app_src.display();
            eprintln!("cannot check {path}: {err}");
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

/// Create the widget `name` in `app/src` and print what is left to do.
fn create_widget(name: &str) -> ExitCode {
    match new_widget::create(&app_src(), name) {
        Ok(widget) => {
            print!("{}", widget.summary());
            ExitCode::SUCCESS
        },
        Err(err) => {
            eprintln!("cannot create the widget `{name}`: {err}");
            ExitCode::FAILURE
        },
    }
}

fn app_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/src")
}
