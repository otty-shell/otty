//! Whether SELinux is enabled on this computer, which decides if a folder
//! mounted into a container needs the `z` option (spec Decision 20).

use std::fs;

/// The file the kernel shows only while SELinux is enabled. It holds `1`
/// when SELinux enforces its rules and `0` when it only logs.
const ENFORCE_FILE: &str = "/sys/fs/selinux/enforce";

/// Whether SELinux is enabled on this computer: `/sys/fs/selinux/enforce`
/// exists and can be read. Permissive mode counts as enabled, because the
/// files still carry labels. Read it once per form, not on every redraw.
///
/// ```
/// // Depends on the computer; only checks that the call returns.
/// let _ = otty_docker::is_selinux_enabled();
/// ```
pub fn is_selinux_enabled() -> bool {
    let contents = fs::read_to_string(ENFORCE_FILE).ok();

    is_enabled_from(contents.as_deref())
}

/// SELinux is enabled when the enforce file exists, whatever it says.
/// `contents` is `None` when the file is missing.
fn is_enabled_from(contents: Option<&str>) -> bool {
    contents.is_some()
}

#[cfg(test)]
mod tests {
    use super::is_enabled_from;

    #[test]
    fn a_missing_enforce_file_means_selinux_is_off() {
        assert!(!is_enabled_from(None));
    }

    #[test]
    fn an_enforce_file_means_selinux_is_on_whatever_it_says() {
        assert!(is_enabled_from(Some("1")));
        assert!(is_enabled_from(Some("0")));
        assert!(is_enabled_from(Some("")));
    }
}
