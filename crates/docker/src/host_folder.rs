//! Whether a folder on this computer exists, asked off the UI thread: a hung
//! network mount can block this check for a long time. Also the wait the app
//! uses around these questions, because it has no timer of its own.

use std::path::Path;
use std::time::Duration;

use tokio::sync::oneshot;

/// Whether `path` exists on this computer. A path that cannot be checked
/// counts as missing, the same meaning as [`Path::exists`]. The check runs on
/// its own thread, so one stuck on a hung mount never delays closing the app.
///
/// ```
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// use std::path::Path;
///
/// assert!(!otty_docker::host_folder_exists(Path::new("/otty/none")).await);
/// # }
/// ```
pub async fn host_folder_exists(path: &Path) -> bool {
    let path = path.to_path_buf();

    answer_from_own_thread(move || path.try_exists().unwrap_or(false)).await
}

/// Wait for `duration` without holding a thread.
///
/// Needs a tokio runtime.
///
/// ```
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// use std::time::Duration;
///
/// otty_docker::wait_for(Duration::from_millis(1)).await;
/// # }
/// ```
pub async fn wait_for(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Run `check` on a thread of its own and wait for its answer. Not on the
/// runtime's blocking pool: closing the runtime waits for that pool, so a check
/// stuck on a hung mount would keep the app from exiting. A thread that cannot
/// start or a check that panics counts as "cannot be checked".
async fn answer_from_own_thread(
    check: impl FnOnce() -> bool + Send + 'static,
) -> bool {
    let (answer, answered) = oneshot::channel();

    let thread = std::thread::Builder::new()
        .name(String::from("otty-folder-check"))
        .spawn(move || {
            let _ = answer.send(check());
        });
    if let Err(err) = thread {
        log::warn!("no thread for a folder check: {err}");
        return false;
    }

    answered.await.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::test_support::TempDir;

    /// More than enough for anything these tests wait for.
    const TEST_LIMIT: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn the_wait_lasts_at_least_the_asked_time() {
        let started = tokio::time::Instant::now();

        wait_for(Duration::from_millis(20)).await;

        assert!(started.elapsed() >= Duration::from_millis(20));
    }

    #[test]
    fn closing_the_runtime_does_not_wait_for_a_stuck_check() {
        let (release, stuck) = mpsc::channel::<()>();
        let (started, has_started) = mpsc::channel();
        let (closed, has_closed) = mpsc::channel();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("a runtime");

        // A check that stays stuck until the test ends, like one on a hung
        // network mount.
        drop(runtime.spawn(answer_from_own_thread(move || {
            let _ = started.send(());
            let _ = stuck.recv();
            true
        })));
        runtime.block_on(tokio::task::yield_now());
        has_started
            .recv_timeout(TEST_LIMIT)
            .expect("the check should start");

        // On a thread, so a runtime that waits fails the test and cannot hang
        // it: `release` ends the check when the test unwinds.
        std::thread::spawn(move || {
            drop(runtime);
            let _ = closed.send(());
        });
        let has_closed_in_time = has_closed.recv_timeout(TEST_LIMIT).is_ok();

        assert!(has_closed_in_time, "the runtime waited for the stuck check");
        drop(release);
    }

    #[tokio::test]
    async fn a_check_that_panics_counts_as_missing() {
        let answer = answer_from_own_thread(|| panic!("the check broke")).await;

        assert!(!answer);
    }

    #[tokio::test]
    async fn a_folder_on_disk_exists() {
        let dir = TempDir::new();

        assert!(host_folder_exists(dir.path()).await);
    }

    #[tokio::test]
    async fn a_missing_path_does_not_exist() {
        let dir = TempDir::new();

        assert!(!host_folder_exists(&dir.path().join("missing")).await);
    }

    #[tokio::test]
    async fn a_path_that_cannot_be_checked_does_not_exist() {
        assert!(!host_folder_exists(Path::new("bad\0path")).await);
    }
}
