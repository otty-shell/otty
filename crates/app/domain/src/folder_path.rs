//! A folder written short, as a terminal prompt writes it: the pane header
//! of Terminal mode and the project screen of Agents mode show the same
//! text for the same folder.

use std::path::{Component, Path};

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
}
