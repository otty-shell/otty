//! Test-only values shared by the tests of this crate: the test host over
//! `fixtures/`, which stands for the home folder, and the profiles of its
//! made-up config folders.

use otty_plugin_sdk::agents::Profile;
use otty_plugin_sdk::files::FileInfo;
use otty_plugin_sdk::test_host::TestHost;

/// The agent part the tests call.
pub(crate) const AGENT: &str = "codex";

/// 2026-11-16T10:00:00Z, in milliseconds since the Unix epoch.
pub(crate) const TEN: i64 = 1_794_823_200_000;

/// The ChatGPT account of the made-up logins of `.codex-plus`,
/// `.codex-idtoken`, `.codex-expired` and `.codex-gateway`.
pub(crate) const ACCOUNT: &str = "00000000-0000-4000-8000-0000000000c1";

/// The session of the default fixture folder in `/home/dev/shop`.
pub(crate) const SHOP_SESSION: &str = "019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b";

/// The session of the default fixture folder in `/home/dev/blog`, kept
/// compressed in `sessions/` and as a plain copy in `archived_sessions/`.
pub(crate) const BLOG_SESSION: &str = "019a9000-1111-7222-8333-444455556666";

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

/// Runs `run` as the part `codex` of `host`.
pub(crate) fn as_part<R>(host: &TestHost, run: impl FnOnce() -> R) -> R {
    host.as_agent(AGENT, run)
}

/// A file a listing found at `path`, changed at `changed_ms`.
pub(crate) fn file_info(path: &str, changed_ms: i64) -> FileInfo {
    FileInfo {
        path: path.to_string(),
        size: 100,
        changed_ms,
    }
}
