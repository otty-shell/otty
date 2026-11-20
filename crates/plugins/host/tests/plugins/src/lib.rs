//! The test plugins of `otty-plugin-host`. Each build turns on one
//! feature and gives one component; `scripts/build-test-plugins.sh`
//! builds them all and copies them to `tests/fixtures/`.
//!
//! Every one exports the agent contract through `otty-plugin-sdk`:
//! `minimal` only that, `misbehave` also calls that never end or grab
//! memory, `read-files` reads a file with the standard library, so it
//! imports `wasi:filesystem`.

use otty_plugin_sdk::{
    Agent, Answers, Command, Login, Profile, ProfilePlace, ProfileStatus,
    ReadError, Session, SpendItem, SpendPlace,
};

/// The test plugin.
struct Plugin;

impl Agent for Plugin {
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        Vec::new()
    }

    fn detect(_profile: &Profile) -> ProfileStatus {
        ProfileStatus::NotLoggedIn
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

    fn login_command(_profile: &Profile) -> Command {
        command()
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

fn command() -> Command {
    Command {
        program: "test-agent".to_string(),
        args: Vec::new(),
        env: Vec::new(),
    }
}
