use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, process, thread};

use super::{
    ShellLaunch, fallback_shell_launch_with_shell, setup_bash_launch,
    setup_shell_launch_with_shell, setup_zsh_launch,
};

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "otty-shell-bootstrap-{prefix}-{}-{stamp}",
            process::id(),
        ));
        fs::create_dir_all(&path)
            .expect("failed to create temporary directory");

        Self { path }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn env_value<'a>(launch: &'a ShellLaunch, key: &str) -> Option<&'a str> {
    launch
        .envs()
        .iter()
        .find_map(|(name, value)| (name == key).then_some(value.as_str()))
}

#[test]
fn given_shell_launch_when_constructed_then_getters_return_values() {
    let launch =
        ShellLaunch::new(String::from("shell"), String::from("/bin/sh"));

    assert_eq!(launch.name(), "shell");
    assert_eq!(launch.program(), "/bin/sh");
    assert!(launch.args().is_empty());
    assert!(launch.envs().is_empty());
}

#[test]
fn given_shell_path_when_fallback_launch_created_then_uses_local_program() {
    let launch = fallback_shell_launch_with_shell("/usr/bin/bash");

    assert_eq!(launch.name(), "bash");
    assert_eq!(launch.program(), "/usr/bin/bash");
    assert!(launch.args().is_empty());
    assert_eq!(
        env_value(&launch, "OTTY_INTEGRATION_DEGRADED"),
        Some("bootstrap_failed"),
    );
}

#[test]
fn unsupported_shell_is_reported_in_launch_environment() {
    let launch = setup_shell_launch_with_shell("/usr/bin/fish")
        .expect("unsupported shell still starts normally");

    assert_eq!(launch.program(), "/usr/bin/fish");
    assert_eq!(
        env_value(&launch, "OTTY_INTEGRATION_UNSUPPORTED"),
        Some("fish"),
    );
}

#[cfg(unix)]
#[test]
fn given_temp_dir_when_setup_bash_launch_then_wrapper_files_are_written() {
    let temp_dir = TempDirGuard::new("bash");
    let launch = setup_bash_launch("/bin/bash", &temp_dir.path).expect("setup");

    let script_path = temp_dir.path.join("shell-integrations/v2/otty.bash");
    let wrapper_path = temp_dir.path.join("otty-v2.bashrc");
    assert!(script_path.exists());
    assert!(wrapper_path.exists());
    assert_eq!(launch.program(), "/bin/bash");
    assert_eq!(launch.args().len(), 2);
    assert_eq!(launch.args()[0], "--rcfile");
    assert_eq!(launch.args()[1], wrapper_path.to_string_lossy());
}

#[cfg(unix)]
#[test]
fn shell_launch_uses_unique_cryptographic_terminal_session_id() {
    let first_dir = TempDirGuard::new("session-id-first");
    let second_dir = TempDirGuard::new("session-id-second");
    let first =
        setup_bash_launch("/bin/bash", &first_dir.path).expect("first setup");
    let second =
        setup_bash_launch("/bin/bash", &second_dir.path).expect("second setup");

    let first_id = env_value(&first, "OTTY_TERMINAL_SESSION_ID")
        .expect("first terminal session id");
    let second_id = env_value(&second, "OTTY_TERMINAL_SESSION_ID")
        .expect("second terminal session id");

    assert_ne!(first_id, second_id);
    assert_eq!(first_id.len(), 32);
    assert!(first_id.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[cfg(unix)]
#[test]
fn concurrent_shell_setup_publishes_complete_versioned_assets() {
    let temp_dir = TempDirGuard::new("concurrent");
    let handles = (0..8)
        .map(|_| {
            let path = temp_dir.path.clone();
            thread::spawn(move || setup_bash_launch("/bin/bash", &path))
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle
            .join()
            .expect("setup thread should not panic")
            .expect("concurrent setup should succeed");
    }

    let script_path = temp_dir.path.join("shell-integrations/v2/otty.bash");
    assert_eq!(
        fs::read_to_string(script_path).expect("versioned Bash script"),
        super::OTTY_BASH_SCRIPT,
    );
}

#[cfg(unix)]
#[test]
fn existing_integration_asset_permissions_are_repaired() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let temp_dir = TempDirGuard::new("permissions");
    setup_bash_launch("/bin/bash", &temp_dir.path).expect("first setup");
    let script_path = temp_dir.path.join("shell-integrations/v2/otty.bash");
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o666))
        .expect("make asset unsafe for regression setup");

    setup_bash_launch("/bin/bash", &temp_dir.path)
        .expect("second setup repairs permissions");

    let mode = fs::metadata(script_path).expect("asset metadata").mode();
    assert_eq!(mode & 0o077, 0);
}

#[cfg(unix)]
#[test]
fn given_temp_dir_when_setup_zsh_launch_then_wrapper_files_are_written() {
    let temp_dir = TempDirGuard::new("zsh");
    let launch = setup_zsh_launch("/bin/zsh", &temp_dir.path).expect("setup");

    let script_path = temp_dir.path.join("shell-integrations/v2/otty.zsh");
    let zshrc_path = temp_dir.path.join(".zshrc");
    assert!(script_path.exists());
    assert!(zshrc_path.exists());
    assert_eq!(launch.program(), "/bin/zsh");
    assert_eq!(
        env_value(&launch, "ZDOTDIR"),
        Some(temp_dir.path.to_string_lossy().as_ref()),
    );
}
