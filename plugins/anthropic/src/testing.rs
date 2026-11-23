//! Test-only values shared by the tests of this crate: the test host over
//! `fixtures/`, which stands for the home folder, the profiles of its
//! made-up config folders, and those folders as OTTY gives them to the
//! provider part.

use otty_plugin_sdk::agents::{LoginFolder, Profile};
use otty_plugin_sdk::test_host::TestHost;

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

/// The config folder `name` of `host` as OTTY gives it to the provider
/// part: a profile of the agent part `claude-code`.
pub(crate) fn login_folder(host: &TestHost, name: &str) -> LoginFolder {
    LoginFolder::Profile((AGENT.to_string(), path(host, name)))
}

/// The folder of OTTY's own login of the fixtures, in the home folder of
/// the host: `<data dir>/accounts/<id>`, outside every `~/.claude*`.
pub(crate) const OWN_FOLDER: &str = ".local/share/otty/accounts/6f0c1e2a";

/// [`OWN_FOLDER`] of `host` as OTTY gives it to the provider part: its
/// own login `6f0c1e2a`, where `claude auth login` ran.
pub(crate) fn own_login_folder(host: &TestHost) -> LoginFolder {
    LoginFolder::OwnLogin(("6f0c1e2a".to_string(), path(host, OWN_FOLDER)))
}

/// Runs `run` as the part `claude-code` of `host`.
pub(crate) fn as_part<R>(host: &TestHost, run: impl FnOnce() -> R) -> R {
    host.as_agent(AGENT, run)
}

/// Runs `run` as the part `anthropic` of `host` in a call given the login
/// folders `folders`, as OTTY calls `accounts`, or `numbers` and
/// `read_bill` with the folders of an account.
pub(crate) fn as_provider_in<R>(
    host: &TestHost,
    folders: &[LoginFolder],
    run: impl FnOnce() -> R,
) -> R {
    host.as_provider_in(PROVIDER, folders, run)
}
