//! Test-only values shared by the tests of this crate: the test host over
//! `fixtures/`, which stands for the home folder, and the profiles of its
//! made-up config folders.

use otty_plugin_sdk::agents::{FoundProfile, Profile};
use otty_plugin_sdk::test_host::TestHost;

use crate::claude_code::start_command;

/// The agent part the tests call.
pub(crate) const AGENT: &str = "claude-code";

/// The provider part the tests call.
pub(crate) const PROVIDER: &str = "anthropic";

/// The host over `fixtures/`.
pub(crate) fn host() -> TestHost {
    TestHost::new(env!("CARGO_MANIFEST_DIR"))
}

/// The absolute path of `relative` in the home folder of `host`.
pub(crate) fn path(host: &TestHost, relative: &str) -> String {
    host.home().join(relative).display().to_string()
}

/// The profile of the config folder `name` in the home folder of `host`.
pub(crate) fn profile(
    host: &TestHost,
    name: &str,
    is_default: bool,
) -> Profile {
    Profile {
        agent: AGENT.to_string(),
        folder: path(host, name),
        is_default,
    }
}

/// The config folder `name` of `host` as the provider part is told an
/// account was found in it: by the agent part `claude-code`, with the
/// command it starts Claude Code with there.
pub(crate) fn found_profile(
    host: &TestHost,
    name: &str,
    is_default: bool,
) -> FoundProfile {
    let profile = profile(host, name, is_default);

    FoundProfile {
        plugin: String::from("anthropic"),
        agent: AGENT.to_string(),
        launch: start_command(&profile, Vec::new()),
        folder: profile.folder,
    }
}

/// Runs `run` as the part `claude-code` of `host`.
pub(crate) fn as_part<R>(host: &TestHost, run: impl FnOnce() -> R) -> R {
    host.as_agent(AGENT, run)
}

/// Runs `run` as the part `anthropic` of `host`.
pub(crate) fn as_provider<R>(host: &TestHost, run: impl FnOnce() -> R) -> R {
    host.as_provider(PROVIDER, run)
}
