//! `files.list`: the files whose paths match a glob and that the part may
//! read, read with its keys replaced, or query.
//!
//! The walk goes folder by folder along the glob, so `~/.agent-*/x/*.jsonl`
//! reads only the folders that can match rather than the whole home
//! folder. It never follows a symlink to a folder; a symlink to a file is
//! listed when the real file lies inside the part's globs too.

use std::fs::{self, FileType};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use globset::{GlobBuilder, GlobMatcher};
use otty_plugin_manifest::{Environment, Permissions};

use crate::operations::real_path::{RealFolders, not_allowed};
use crate::operations::v0_1::otty::host::files::FileInfo;
use crate::operations::v0_1::otty::host::types::HostError;

/// The most files `files.list` gives (spec "Host operations").
pub(super) const MOST_FILES: usize = 100_000;

/// One folder level of a glob, between two `/`.
enum Level {
    /// A plain name, such as `sessions`.
    Name(String),
    /// A pattern of one level, such as `*.jsonl`.
    Pattern(GlobMatcher),
    /// `**`, or a pattern that spans levels, such as `{a/b,c}`: every
    /// file below.
    Everything,
}

/// One listing: the glob, the part's permissions and what was found.
struct Listing<'a> {
    glob: GlobMatcher,
    permissions: &'a Permissions,
    real_folders: RealFolders,
    /// The folder the walk starts in and where it really is.
    start: (PathBuf, PathBuf),
    found: Vec<FileInfo>,
}

impl Listing<'_> {
    fn is_full(&self) -> bool {
        self.found.len() >= MOST_FILES
    }

    /// Walks `folder` along `levels`.
    fn walk(&mut self, folder: &Path, levels: &[Level]) {
        let Some((level, rest)) = levels.split_first() else {
            return;
        };

        match level {
            Level::Everything => self.walk_everything(folder),
            Level::Name(name) => {
                let path = folder.join(name);
                if let Ok(metadata) = fs::symlink_metadata(&path) {
                    self.visit(&path, metadata.file_type(), rest);
                }
            },
            Level::Pattern(pattern) => {
                for (path, file_type) in entries(folder) {
                    if self.is_full() {
                        return;
                    }
                    let name = path.file_name().map(Path::new);
                    if name.is_some_and(|name| pattern.is_match(name)) {
                        self.visit(&path, file_type, rest);
                    }
                }
            },
        }
    }

    /// Goes into `path` when levels are left, or takes it as a file.
    fn visit(&mut self, path: &Path, file_type: FileType, rest: &[Level]) {
        if rest.is_empty() {
            self.add(path, file_type);
        } else if file_type.is_dir() {
            self.walk(path, rest);
        }
    }

    /// Every file below `folder`, folders walked into.
    fn walk_everything(&mut self, folder: &Path) {
        for (path, file_type) in entries(folder) {
            if self.is_full() {
                return;
            }
            if file_type.is_dir() {
                self.walk_everything(&path);
            } else {
                self.add(&path, file_type);
            }
        }
    }

    /// Adds the file `path` when it matches the glob and the part may
    /// reach it.
    fn add(&mut self, path: &Path, file_type: FileType) {
        if self.is_full() || !self.glob.is_match(path) || !self.allows(path) {
            return;
        }

        let real = if file_type.is_symlink() {
            match fs::canonicalize(path) {
                Ok(real) => real,
                Err(_) => return,
            }
        } else {
            self.real_path_of(path)
        };
        if real != path
            && !self.real_folders.allows(self.permissions, &real, may_list)
        {
            return;
        }

        if let Some(info) = file_info(path, &real) {
            self.found.push(info);
        }
    }

    fn allows(&self, path: &Path) -> bool {
        may_list(self.permissions, path)
    }

    /// Where the file `path`, reached with no symlink below the start
    /// folder, really is.
    fn real_path_of(&self, path: &Path) -> PathBuf {
        let (start, real_start) = &self.start;

        match path.strip_prefix(start) {
            Ok(rest) if rest.as_os_str().is_empty() => real_start.clone(),
            Ok(rest) => real_start.join(rest),
            Err(_) => path.to_path_buf(),
        }
    }
}

/// The files whose paths match `glob`, such as
/// `/home/user/.agent/sessions/*.jsonl`, that the part may read, read
/// with its keys replaced, or query; at most [`MOST_FILES`]. A glob that
/// is not absolute or holds `..` is refused.
pub(super) fn list(
    permissions: &Permissions,
    env: &Environment,
    glob: &str,
) -> Result<Vec<FileInfo>, HostError> {
    let Some(relative) = glob.strip_prefix('/') else {
        return Err(not_allowed(Path::new(glob)));
    };
    if relative.split('/').any(|level| level == "..") {
        return Err(not_allowed(Path::new(glob)));
    }

    let matcher = path_glob(glob)?;
    let (start, levels) = split_levels(relative)?;
    let real_start = fs::canonicalize(&start).unwrap_or_else(|_| start.clone());

    let mut listing = Listing {
        glob: matcher,
        permissions,
        real_folders: RealFolders::new(env),
        start: (start.clone(), real_start),
        found: Vec::new(),
    };
    if levels.is_empty() {
        if let Ok(metadata) = fs::symlink_metadata(&start) {
            listing.add(&start, metadata.file_type());
        }
    } else {
        listing.walk(&start, &levels);
    }

    Ok(listing.found)
}

/// Whether the part may reach `path` in any way a listing shows.
fn may_list(permissions: &Permissions, path: &Path) -> bool {
    permissions.may_read(path)
        || permissions.may_read_with_keys(path)
        || permissions.may_query(path)
}

/// The folder a glob's walk starts in, its plain names up to the first
/// pattern, and the levels after it. A glob of plain names only starts in
/// the file it names, with no levels.
fn split_levels(relative: &str) -> Result<(PathBuf, Vec<Level>), HostError> {
    let mut start = PathBuf::from("/");
    let mut levels = Vec::new();
    let mut parts = relative.split('/').filter(|part| !part.is_empty());

    for part in parts.by_ref() {
        if is_pattern(part) {
            levels.push(level_of(part)?);
            break;
        }
        start.push(part);
    }
    for part in parts {
        if matches!(levels.last(), Some(Level::Everything)) {
            break;
        }
        levels.push(level_of(part)?);
    }

    Ok((start, levels))
}

/// The level of one part of a glob between two `/`.
fn level_of(part: &str) -> Result<Level, HostError> {
    if part == "**" || part.contains("**") || !has_balanced_braces(part) {
        return Ok(Level::Everything);
    }
    if !is_pattern(part) {
        return Ok(Level::Name(part.to_string()));
    }

    Ok(Level::Pattern(path_glob(part)?))
}

fn is_pattern(part: &str) -> bool {
    part.contains(['*', '?', '[', '{', '\\'])
}

/// Whether every `{` of `part` closes in it; a choice such as `{a/b,c}`
/// spans levels and does not.
fn has_balanced_braces(part: &str) -> bool {
    part.matches('{').count() == part.matches('}').count()
}

/// A glob over paths: `*` and `?` stay inside one folder.
fn path_glob(pattern: &str) -> Result<GlobMatcher, HostError> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|error| HostError::Failed(format!("`{pattern}`: {error}")))
}

/// The entries of `folder` with their types, symlinks not followed.
fn entries(folder: &Path) -> Vec<(PathBuf, FileType)> {
    let Ok(read) = fs::read_dir(folder) else {
        return Vec::new();
    };

    read.flatten()
        .filter_map(|entry| Some((entry.path(), entry.file_type().ok()?)))
        .collect()
}

/// The size and change time of the real file `real`, listed as `path`;
/// none for a folder.
fn file_info(path: &Path, real: &Path) -> Option<FileInfo> {
    let metadata = fs::metadata(real).ok()?;
    if !metadata.is_file() {
        return None;
    }

    let changed_ms = metadata
        .modified()
        .ok()
        .and_then(|changed| changed.duration_since(UNIX_EPOCH).ok())
        .and_then(|since_epoch| i64::try_from(since_epoch.as_millis()).ok())
        .unwrap_or_default();

    Some(FileInfo {
        path: path.display().to_string(),
        size: metadata.len(),
        changed_ms,
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;
    use crate::testing::{TempDir, debug_err, files_permissions, write_file};

    struct Home {
        dir: TempDir,
        env: Environment,
    }

    impl Home {
        fn path(&self, relative: &str) -> PathBuf {
            self.dir.path().join(relative)
        }

        fn new() -> Self {
            let dir = TempDir::new();
            let env = Environment::new(dir.path().to_path_buf());

            Self { dir, env }
        }

        fn write(&self, relative: &str, bytes: &[u8]) {
            write_file(&self.path(relative), bytes);
        }

        /// The files `part` lists with `glob`, written from the home
        /// folder, as paths from the home folder, sorted.
        fn list(&self, part: &str, glob: &str) -> Result<Vec<String>, String> {
            let permissions = files_permissions(part, &self.env);
            let glob = format!("{}/{glob}", self.dir.path().display());
            let home = format!("{}/", self.dir.path().display());

            let found = debug_err(list(&permissions, &self.env, &glob))?;

            let mut paths: Vec<String> = found
                .iter()
                .map(|info| info.path.trim_start_matches(&home).to_string())
                .collect();
            paths.sort();

            Ok(paths)
        }
    }

    fn names(paths: &[&str]) -> Result<Vec<String>, String> {
        Ok(paths.iter().map(|path| path.to_string()).collect())
    }

    #[test]
    fn the_files_matching_the_glob_are_listed_with_size_and_time() {
        let home = Home::new();
        home.write(".alpha/sessions/a.jsonl", b"12345");
        home.write(".alpha/sessions/b.jsonl", b"1");
        home.write(".alpha/sessions/notes.txt", b"1");
        home.write(".alpha/sessions/old/c.jsonl", b"1");
        let permissions = files_permissions("alpha", &home.env);
        let glob = home.path(".alpha/sessions/a.jsonl").display().to_string();

        let found = list(&permissions, &home.env, &glob).expect("listed");

        assert_eq!(
            home.list("alpha", ".alpha/sessions/*.jsonl"),
            names(&[".alpha/sessions/a.jsonl", ".alpha/sessions/b.jsonl"])
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].size, 5);
        assert!(found[0].changed_ms > 0);
    }

    #[test]
    fn two_stars_and_choices_cross_folders() {
        let home = Home::new();
        home.write(".alpha/sessions/a.jsonl", b"1");
        home.write(".alpha/sessions/2026/11/b.jsonl", b"1");
        home.write(".alpha/archive/c.jsonl", b"1");
        home.write(".alpha/other/d.jsonl", b"1");

        assert_eq!(
            home.list("alpha", ".alpha/sessions/**/*.jsonl"),
            names(&[
                ".alpha/sessions/2026/11/b.jsonl",
                ".alpha/sessions/a.jsonl"
            ])
        );
        assert_eq!(
            home.list("alpha", ".alpha/{sessions,archive}/*.jsonl"),
            names(&[".alpha/archive/c.jsonl", ".alpha/sessions/a.jsonl"])
        );
        assert_eq!(
            home.list("alpha", ".alpha/{sessions/2026/11,archive}/*.jsonl"),
            names(&[
                ".alpha/archive/c.jsonl",
                ".alpha/sessions/2026/11/b.jsonl"
            ])
        );
        assert_eq!(
            home.list("alpha", ".alph*/other/*"),
            names(&[".alpha/other/d.jsonl"])
        );
    }

    #[test]
    fn files_outside_the_part_globs_are_left_out() {
        let home = Home::new();
        home.write(".alpha/a.jsonl", b"1");
        home.write(".beta/b.jsonl", b"1");
        home.write(".ssh/id_ed25519", b"secret");

        assert_eq!(home.list("alpha", "**"), names(&[".alpha/a.jsonl"]));
        assert_eq!(home.list("alpha", ".ssh/*"), names(&[]));
    }

    #[test]
    fn files_of_another_part_of_the_same_plugin_are_left_out() {
        let home = Home::new();
        home.write(".beta/b.jsonl", b"1");

        assert_eq!(home.list("alpha", ".beta/*.jsonl"), names(&[]));
        assert_eq!(
            home.list("beta", ".beta/*.jsonl"),
            names(&[".beta/b.jsonl"])
        );
    }

    #[test]
    fn key_files_and_databases_are_listed() {
        let home = Home::new();
        home.write(".alpha/auth.json", b"{}");
        home.write(".alpha/data.db", b"");

        assert_eq!(
            home.list("alpha", ".alpha/*"),
            names(&[".alpha/auth.json", ".alpha/data.db"])
        );
    }

    #[test]
    fn a_glob_with_dot_dot_or_not_absolute_is_refused() {
        let home = Home::new();
        let permissions = files_permissions("alpha", &home.env);
        let up = format!("{}/.alpha/../.ssh/*", home.dir.path().display());

        let refused = |glob: &str| {
            debug_err(list(&permissions, &home.env, glob))
                .map(|found| found.len())
        };

        assert_eq!(refused(&up), debug_err(Err(not_allowed(Path::new(&up)))));
        assert_eq!(
            refused(".alpha/*"),
            debug_err(Err(not_allowed(Path::new(".alpha/*"))))
        );
    }

    #[test]
    fn a_symlink_leaving_the_globs_is_left_out() {
        let home = Home::new();
        home.write(".ssh/id_ed25519", b"secret");
        home.write(".alpha/data/a.jsonl", b"1");
        symlink(home.path(".ssh/id_ed25519"), home.path(".alpha/key"))
            .expect("symlink");
        symlink(home.path(".alpha/data/a.jsonl"), home.path(".alpha/latest"))
            .expect("symlink");
        symlink(home.path(".ssh"), home.path(".alpha/ssh")).expect("symlink");

        assert_eq!(
            home.list("alpha", ".alpha/**"),
            names(&[".alpha/data/a.jsonl", ".alpha/latest"])
        );
        assert_eq!(home.list("alpha", ".alpha/ssh/*"), names(&[]));
    }

    #[test]
    fn a_start_folder_that_is_a_symlink_is_checked_where_it_leads() {
        let home = Home::new();
        home.write("dotfiles/alpha/a.jsonl", b"1");
        symlink(home.path("dotfiles/alpha"), home.path(".alpha"))
            .expect("symlink");

        assert_eq!(home.list("alpha", ".alpha/*.jsonl"), names(&[]));
    }

    #[test]
    fn a_missing_folder_lists_nothing() {
        let home = Home::new();

        assert_eq!(home.list("alpha", ".alpha/sessions/*.jsonl"), names(&[]));
    }
}
