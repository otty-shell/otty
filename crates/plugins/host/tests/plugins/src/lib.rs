//! The test plugins of `otty-plugin-host`. Each build turns on one
//! feature and gives one component; `scripts/build-test-plugins.sh`
//! builds them all and copies them to `tests/fixtures/`.
//!
//! Every one exports the agent contract through `otty-plugin-sdk`:
//! `minimal` only that, `misbehave` also calls that never end or grab
//! memory, `read-files` reads a file with the standard library, so it
//! imports `wasi:filesystem`, and `programs` reads its limits by running,
//! talking to or starting in a hidden terminal the program `stand-in`.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

use otty_plugin_sdk::{
    Agent, Answers, Command, Limits, Login, LoginRef, Need, NeedAction,
    Profile, ProfilePlace, ProfileStatus, ReadError, Session, SpendItem,
    SpendPlace,
};

/// The test plugin.
struct Plugin;

impl Agent for Plugin {
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        Vec::new()
    }

    /// Never logged in; the need runs the agent's login, in the language
    /// the host names.
    fn detect(_profile: &Profile) -> ProfileStatus {
        let title = match otty_plugin_sdk::env::language().as_str() {
            "ru" => "Нужен вход",
            _ => "Needs login",
        };

        ProfileStatus::NotLoggedIn(Need {
            id: "not-logged-in".to_string(),
            title: title.to_string(),
            text: "Log in to the test agent.".to_string(),
            action: NeedAction::Run(command()),
        })
    }

    fn version(_profile: &Profile) -> Option<String> {
        version()
    }

    fn logins(_profile: &Profile) -> Vec<Login> {
        Vec::new()
    }

    fn sessions(_profile: &Profile) -> Result<Vec<Session>, ReadError> {
        Ok(Vec::new())
    }

    fn spend_places(_profile: &Profile) -> Vec<SpendPlace> {
        Vec::new()
    }

    fn read_answers(
        _profile: &Profile,
        _item: &SpendItem,
    ) -> Result<Answers, ReadError> {
        Err(ReadError::NotSupported)
    }

    fn launch(_profile: &Profile, _session: Option<&str>) -> Command {
        command()
    }

    /// The limits read the way the variable name of `login` says, such as
    /// `run`: the tests choose the way with the login they pass.
    fn read_limits(
        profile: &Profile,
        login: &LoginRef,
    ) -> Result<Limits, ReadError> {
        let way = match login {
            LoginRef::EnvVar(name) => name.as_str(),
            _ => "none",
        };

        limits(profile, way)
    }
}

otty_plugin_sdk::export!(Plugin with agent);

#[cfg(feature = "misbehave")]
mod misbehave {
    use crate::Plugin;
    use crate::misbehave::exports::otty::test::misbehave::Guest;

    wit_bindgen::generate!({ path: "wit", world: "misbehave-plugin" });

    impl Guest for Plugin {
        fn spin() -> u64 {
            let mut turns: u64 = 0;
            loop {
                turns = std::hint::black_box(turns.wrapping_add(1));
                if std::hint::black_box(false) {
                    return turns;
                }
            }
        }

        fn hold(mib: u32) -> u64 {
            let bytes = mib as usize * 1024 * 1024;
            let mut held = vec![0_u8; bytes];
            for page in held.chunks_mut(4096) {
                page[0] = 1;
            }

            std::hint::black_box(&held).len() as u64
        }
    }

    export!(Plugin);
}

#[cfg(not(feature = "read-files"))]
fn version() -> Option<String> {
    eprintln!("the test plugin was asked its version");

    Some("1.0.0".to_string())
}

#[cfg(feature = "read-files")]
fn version() -> Option<String> {
    std::fs::read_to_string("/etc/hostname").ok()
}

#[cfg(not(feature = "programs"))]
fn limits(_profile: &Profile, _way: &str) -> Result<Limits, ReadError> {
    Err(ReadError::NotSupported)
}

/// The limits whose plan is what `stand-in` answered: its output for the
/// way `run`, its first line for `talk`, its screen after `/usage` and
/// the working folder of hidden terminals for `terminal`. The talk and
/// the terminal are kept, never dropped, so the host must stop them when
/// the call ends; the way `keep-talk` only starts a talk and keeps it.
#[cfg(feature = "programs")]
fn limits(profile: &Profile, way: &str) -> Result<Limits, ReadError> {
    use std::cell::RefCell;

    use otty_plugin_sdk::programs::{self, Talk};
    use otty_plugin_sdk::terminal::HiddenTerminal;

    thread_local! {
        static KEPT_TALKS: RefCell<Vec<Talk>> = const { RefCell::new(Vec::new()) };
        static KEPT_TERMINALS: RefCell<Vec<HiddenTerminal>> =
            const { RefCell::new(Vec::new()) };
    }

    let env = vec![("STAND_IN_HOME".to_string(), profile.folder.clone())];
    let serve = ["serve", "--profile", "work"].map(String::from);

    let plan = match way {
        "run" => {
            let output = programs::run(
                "stand-in",
                &["--version".to_string()],
                &env,
                5000,
            )
            .map_err(ReadError::Host)?;
            String::from_utf8_lossy(&output.stdout).into_owned()
        },
        "talk" => {
            let talk = Talk::start("stand-in", &serve, &env)
                .map_err(ReadError::Host)?;
            talk.send_line("ping").map_err(ReadError::Host)?;
            let line = talk.next_line(1000).map_err(ReadError::Host)?;
            KEPT_TALKS.with_borrow_mut(|kept| kept.push(talk));
            line.unwrap_or_default()
        },
        "keep-talk" => {
            let talk = Talk::start("stand-in", &serve, &env)
                .map_err(ReadError::Host)?;
            KEPT_TALKS.with_borrow_mut(|kept| kept.push(talk));
            String::new()
        },
        "terminal" => {
            let terminal = HiddenTerminal::start(
                "stand-in",
                &["tui".to_string()],
                &env,
                100,
                40,
            )
            .map_err(ReadError::Host)?;
            terminal.send_keys("/usage\r").map_err(ReadError::Host)?;
            terminal.wait_for_change(1000).map_err(ReadError::Host)?;
            let screen = terminal.screen().map_err(ReadError::Host)?;
            let folders = otty_plugin_sdk::terminal::working_folders()
                .map_err(ReadError::Host)?;
            KEPT_TERMINALS.with_borrow_mut(|kept| kept.push(terminal));
            format!("{screen} in {}", folders.join(", "))
        },
        "used-talk" => {
            // A talk kept by an earlier call: its program was stopped.
            let line = KEPT_TALKS.with_borrow(|kept| {
                kept.first().map(|talk| talk.next_line(1000))
            });
            match line {
                Some(line) => {
                    line.map_err(ReadError::Host)?.unwrap_or_default()
                },
                None => String::new(),
            }
        },
        _ => "no program".to_string(),
    };

    Ok(Limits {
        windows: Vec::new(),
        credits: None,
        plan: Some(plan),
        recorded_at_ms: None,
    })
}

fn command() -> Command {
    Command {
        program: "test-agent".to_string(),
        args: Vec::new(),
        env: Vec::new(),
    }
}
