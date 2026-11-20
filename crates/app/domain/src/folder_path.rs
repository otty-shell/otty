//! A folder written short, as a terminal prompt writes it: the pane header
//! of Terminal mode and the project screen of Agents mode show the same
//! text for the same folder. Also how `~` stands for the home folder in a
//! path written or typed by hand. These are rules of text only: nothing
//! here looks at the disk.

use std::path::{Component, Path, PathBuf};

/// Longest folder path, counted after `~` or `/`, shown in full.
const SHORT_PATH_FOLDERS: usize = 3;
/// Folders kept at the end of a longer path.
const SHORT_PATH_TAIL: usize = 2;

/// Write `folder` short, as a terminal prompt does: the home folder `home`
/// is `~`, and when more than three folders follow `~` (or `/` outside the
/// home), only the last two stay, after `…`.
///
/// # Example
///
/// ```
/// use std::path::Path;
///
/// use otty_app_domain::folder_path::short_folder_path;
///
/// let home = Some(Path::new("/home/user"));
///
/// assert_eq!(
///     short_folder_path(
///         Path::new("/home/user/projects/main/ottyrun/otty"),
///         home
///     ),
///     "~/…/ottyrun/otty"
/// );
/// assert_eq!(
///     short_folder_path(Path::new("/home/user/otty"), home),
///     "~/otty"
/// );
/// assert_eq!(
///     short_folder_path(Path::new("/srv/data/a/b/c"), home),
///     "/…/b/c"
/// );
/// ```
pub fn short_folder_path(folder: &Path, home: Option<&Path>) -> String {
    let (start, rest) =
        match home.and_then(|home| folder.strip_prefix(home).ok()) {
            Some(rest) => ("~", rest),
            None => ("", folder),
        };
    let folders: Vec<_> = rest
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy()),
            _ => None,
        })
        .collect();

    if folders.is_empty() {
        return match start {
            "" => folder.display().to_string(),
            _ => String::from(start),
        };
    }
    if folders.len() <= SHORT_PATH_FOLDERS {
        return format!("{start}/{}", folders.join("/"));
    }

    let tail = folders[folders.len() - SHORT_PATH_TAIL..].join("/");
    format!("{start}/…/{tail}")
}

/// The name of `folder`, as a pane header or a project list shows it:
/// `~` for the home folder `home`, else the last part of the path, or the
/// whole path when it has no last part, such as `/`.
///
/// # Example
///
/// ```
/// use std::path::Path;
///
/// use otty_app_domain::folder_path::folder_name;
///
/// let home = Some(Path::new("/home/user"));
///
/// assert_eq!(folder_name(Path::new("/home/user/otty"), home), "otty");
/// assert_eq!(folder_name(Path::new("/home/user"), home), "~");
/// assert_eq!(folder_name(Path::new("/"), home), "/");
/// ```
pub fn folder_name(folder: &Path, home: Option<&Path>) -> String {
    if home == Some(folder) {
        return String::from("~");
    }

    match folder.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => folder.display().to_string(),
    }
}

/// Write `path` whole, with `~` for the home folder `home`: `~` for the
/// home folder itself, `~/` and the rest for a path in it, else the path
/// as it is.
///
/// # Example
///
/// ```
/// use std::path::Path;
///
/// use otty_app_domain::folder_path::tilde_path;
///
/// let home = Path::new("/home/user");
///
/// assert_eq!(tilde_path(Path::new("/home/user/.coder"), home), "~/.coder");
/// assert_eq!(tilde_path(Path::new("/srv/app"), home), "/srv/app");
/// ```
pub fn tilde_path(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => String::from("~"),
        Ok(rest) => {
            let rest = rest.display();
            format!("~/{rest}")
        },
        Err(_) => path.display().to_string(),
    }
}

/// The path that `text` names when it is written from the home folder
/// `home`: `~` alone is `home`, and `~/` and the rest is the rest in it.
/// Any other text gives `None`, such as `/srv`, `work` or `~user`, whose
/// `~` names the home folder of another user.
///
/// # Example
///
/// ```
/// use std::path::{Path, PathBuf};
///
/// use otty_app_domain::folder_path::expand_home;
///
/// let home = Path::new("/home/user");
///
/// assert_eq!(
///     expand_home("~/work", home),
///     Some(PathBuf::from("/home/user/work"))
/// );
/// assert_eq!(expand_home("/srv", home), None);
/// ```
pub fn expand_home(text: &str, home: &Path) -> Option<PathBuf> {
    let rest = rest_after_home(text)?;

    if rest.is_empty() {
        return Some(home.to_path_buf());
    }
    Some(home.join(rest))
}

/// Whether `text` is written from the home folder: `~` alone or `~/` and
/// the rest, which [`expand_home`] turns into a full path.
///
/// # Example
///
/// ```
/// use otty_app_domain::folder_path::is_written_from_home;
///
/// assert!(is_written_from_home("~/work"));
/// assert!(!is_written_from_home("~user/work"));
/// ```
pub fn is_written_from_home(text: &str) -> bool {
    rest_after_home(text).is_some()
}

/// What follows `~/` in `text`, without the slashes; empty for `~` alone;
/// `None` when `text` is not written from the home folder. All the slashes
/// go, as a path in the home folder is joined to it and joining an
/// absolute path would drop the home folder.
fn rest_after_home(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('~')?;
    if !rest.is_empty() && !rest.starts_with('/') {
        return None;
    }

    Some(rest.trim_start_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/user";

    fn short(folder: &str) -> String {
        short_folder_path(Path::new(folder), Some(Path::new(HOME)))
    }

    #[test]
    fn the_home_folder_is_a_tilde() {
        assert_eq!(short("/home/user"), "~");
        assert_eq!(short("/home/user/"), "~");
    }

    #[test]
    fn a_short_folder_in_the_home_is_written_from_the_tilde() {
        assert_eq!(short("/home/user/otty"), "~/otty");
        assert_eq!(short("/home/user/work/otty"), "~/work/otty");
        assert_eq!(
            short("/home/user/projects/main/ottyrun"),
            "~/projects/main/ottyrun"
        );
    }

    #[test]
    fn a_long_folder_in_the_home_keeps_its_last_two_folders() {
        assert_eq!(
            short("/home/user/projects/main/ottyrun/otty"),
            "~/…/ottyrun/otty"
        );
    }

    #[test]
    fn a_folder_outside_the_home_is_counted_from_the_root() {
        assert_eq!(short("/srv/data/a/b/c"), "/…/b/c");
        assert_eq!(short("/var/log"), "/var/log");
        assert_eq!(short("/"), "/");
    }

    #[test]
    fn a_trailing_slash_changes_nothing() {
        assert_eq!(short("/home/user/work/otty/"), "~/work/otty");
        assert_eq!(
            short("/home/user/projects/main/ottyrun/otty/"),
            "~/…/ottyrun/otty"
        );
        assert_eq!(short("/srv/data/a/b/c/"), "/…/b/c");
    }

    #[test]
    fn a_neighbour_of_the_home_that_starts_alike_is_outside_it() {
        assert_eq!(short("/home/username/otty"), "/home/username/otty");
    }

    #[test]
    fn with_no_known_home_every_folder_is_counted_from_the_root() {
        let folder = Path::new("/home/user/projects/main/ottyrun/otty");

        assert_eq!(short_folder_path(folder, None), "/…/ottyrun/otty");
    }

    #[test]
    fn a_folder_is_named_after_its_last_part() {
        let home = Some(Path::new(HOME));

        assert_eq!(folder_name(Path::new("/work/otty"), home), "otty");
        assert_eq!(folder_name(Path::new("/home/user/otty"), home), "otty");
        assert_eq!(folder_name(Path::new("/"), home), "/");
    }

    /// The home folder is `~`, as the short path writes it, not the name
    /// of the user.
    #[test]
    fn the_home_folder_is_named_with_a_tilde() {
        let home = Some(Path::new(HOME));

        assert_eq!(folder_name(Path::new("/home/user"), home), "~");
        assert_eq!(folder_name(Path::new("/home/user/"), home), "~");
        assert_eq!(folder_name(Path::new("/home/user"), None), "user");
    }

    #[test]
    fn a_path_in_the_home_folder_is_written_from_a_tilde() {
        let home = Path::new(HOME);

        assert_eq!(
            tilde_path(Path::new("/home/user/.codex"), home),
            "~/.codex"
        );
        assert_eq!(
            tilde_path(Path::new("/home/user/a/b/c/d"), home),
            "~/a/b/c/d"
        );
        assert_eq!(tilde_path(home, home), "~");
    }

    #[test]
    fn a_path_outside_the_home_folder_is_written_whole() {
        let home = Path::new(HOME);

        assert_eq!(tilde_path(Path::new("/data/.codex"), home), "/data/.codex");
        assert_eq!(
            tilde_path(Path::new("/home/username/.codex"), home),
            "/home/username/.codex"
        );
    }

    #[test]
    fn a_tilde_stands_for_the_home_folder() {
        let home = Path::new(HOME);

        assert_eq!(expand_home("~", home), Some(PathBuf::from(HOME)));
        assert_eq!(
            expand_home("~/.claude-work", home),
            Some(PathBuf::from("/home/user/.claude-work"))
        );
        assert_eq!(
            expand_home("~//srv", home),
            Some(PathBuf::from("/home/user/srv")),
            "extra slashes do not leave the home folder"
        );
        assert_eq!(expand_home("~/", home), Some(PathBuf::from(HOME)));
    }

    #[test]
    fn a_text_not_written_from_the_home_folder_is_not_expanded() {
        let home = Path::new(HOME);

        for text in ["/srv/data", "work", "~other/src", "/opt/~/bin", ""] {
            assert_eq!(expand_home(text, home), None, "{text}");
            assert!(!is_written_from_home(text), "{text}");
        }
        assert!(is_written_from_home("~"));
        assert!(is_written_from_home("~/src"));
    }
}
