//! The full path of a typed path, the folder a typed path names, and what
//! is wrong with a typed path that is not one.

use std::path::{Path, PathBuf};

use otty_app_domain::folder_path::expand_home;

/// What is wrong with a typed path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathProblem {
    /// No path was typed or picked.
    Empty,
    /// The path is not a full path, such as `work/.coder`.
    NotFull,
}

/// The full path typed as `text`: spaces at the ends do not count, and `~`
/// stands for the home folder `home`.
pub(crate) fn full_path(
    text: &str,
    home: &Path,
) -> Result<PathBuf, PathProblem> {
    let text = text.trim();
    if text.is_empty() {
        return Err(PathProblem::Empty);
    }

    let path = expand_home(text, home).unwrap_or_else(|| PathBuf::from(text));
    if !path.is_absolute() {
        return Err(PathProblem::NotFull);
    }

    Ok(path)
}

/// The folder typed as `text`, as [`full_path`] reads it, with no slash
/// at the end: a picked suggestion has one.
pub(crate) fn full_folder(
    text: &str,
    home: &Path,
) -> Result<PathBuf, PathProblem> {
    let path = full_path(text, home)?;

    Ok(path.components().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/user";

    fn parse(text: &str) -> Result<PathBuf, PathProblem> {
        full_path(text, Path::new(HOME))
    }

    #[test]
    fn a_full_path_is_taken_as_it_is_without_spaces_at_the_ends() {
        assert_eq!(
            parse("  /data/claude-work "),
            Ok(PathBuf::from("/data/claude-work"))
        );
    }

    #[test]
    fn a_tilde_stands_for_the_home_folder() {
        assert_eq!(
            parse("~/.claude-work"),
            Ok(PathBuf::from("/home/user/.claude-work"))
        );
        assert_eq!(parse("~"), Ok(PathBuf::from(HOME)));
    }

    #[test]
    fn a_folder_has_no_slash_at_the_end() {
        let home = Path::new(HOME);

        assert_eq!(
            full_folder("~/work/otty/ ", home),
            Ok(PathBuf::from("/home/user/work/otty"))
        );
        assert_eq!(full_folder("~/", home), Ok(PathBuf::from(HOME)));
        assert_eq!(full_folder("work/", home), Err(PathProblem::NotFull));
    }

    #[test]
    fn an_empty_or_relative_path_is_refused() {
        assert_eq!(parse("   "), Err(PathProblem::Empty));
        assert_eq!(parse(".claude-work"), Err(PathProblem::NotFull));
        assert_eq!(parse("~work/.claude"), Err(PathProblem::NotFull));
    }
}
