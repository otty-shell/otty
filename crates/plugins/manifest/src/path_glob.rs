//! A path glob of a permission, such as `~/.codex*/**` or
//! `{XDG_DATA_HOME}/opencode/*.db`, checked when the manifest is read and
//! made into a concrete glob for this computer.

use std::fmt;
use std::path::{Component, Path};

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};

use crate::environment::Environment;

/// A checked path glob: it starts with `~` (the home folder) or with a
/// variable its part lists in `env`, written `{XDG_DATA_HOME}`, and holds
/// no `..`. `*` stays inside one folder; `**` crosses folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathGlob {
    text: String,
    start: GlobStart,
    rest: String,
}

impl PathGlob {
    /// The glob as the manifest writes it.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The glob in `text`, checked against the variables `listed_vars` of
    /// its part.
    pub(crate) fn parse(
        text: &str,
        listed_vars: &[String],
    ) -> Result<Self, GlobProblem> {
        if text.split('/').any(|segment| segment == "..") {
            return Err(GlobProblem::ParentFolder);
        }

        let (start, rest) = split_start(text)?;
        if let GlobStart::Var(name) = &start
            && !listed_vars.contains(name)
        {
            return Err(GlobProblem::UnlistedVariable(name.clone()));
        }
        if let Some(name) = variable_inside(rest) {
            return Err(GlobProblem::VariableInside(name));
        }
        if let Err(error) = path_glob(&format!("/home{rest}")) {
            return Err(GlobProblem::Syntax(error.kind().to_string()));
        }

        Ok(Self {
            text: text.to_string(),
            start,
            rest: rest.to_string(),
        })
    }

    /// The glob for this computer, with `~` or the variable replaced by
    /// its folder; `None` when the variable is not set or does not hold an
    /// absolute path with no `..`, so the glob matches nothing.
    pub(crate) fn resolve(&self, env: &Environment) -> Option<String> {
        let base = match &self.start {
            GlobStart::Home => env.home().to_str()?,
            GlobStart::Var(name) => env.var(name)?,
        };
        if !is_plain_absolute(Path::new(base)) {
            return None;
        }

        let base = base.trim_end_matches('/');

        Some(format!("{}{}", globset::escape(base), self.rest))
    }
}

impl fmt::Display for PathGlob {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// What a glob starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GlobStart {
    /// `~`: the home folder.
    Home,
    /// `{NAME}`: the folder or file the variable `NAME` names.
    Var(String),
}

/// Why a path glob of the manifest is refused, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobProblem {
    /// It holds `..`.
    ParentFolder,
    /// It starts neither with `~` nor with a variable.
    BadStart,
    /// It starts with a variable its part does not list in `env`.
    UnlistedVariable(String),
    /// It names a variable after its start.
    VariableInside(String),
    /// It is not a glob.
    Syntax(String),
}

impl fmt::Display for GlobProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlobProblem::ParentFolder => formatter.write_str("contains `..`"),
            GlobProblem::BadStart => formatter.write_str(
                "must start with `~` or with a variable such as \
                 `{XDG_DATA_HOME}`",
            ),
            GlobProblem::UnlistedVariable(name) => write!(
                formatter,
                "uses `{{{name}}}`, which the part does not list in `env`"
            ),
            GlobProblem::VariableInside(name) => write!(
                formatter,
                "uses `{{{name}}}` after its start; a variable may only \
                 start a glob"
            ),
            GlobProblem::Syntax(reason) => {
                write!(formatter, "is not a valid glob: {reason}")
            },
        }
    }
}

/// The set of the globs `globs` for this computer; globs that match
/// nothing here are left out.
pub(crate) fn glob_set(globs: &[PathGlob], env: &Environment) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in globs.iter().filter_map(|glob| glob.resolve(env)) {
        if let Ok(glob) = path_glob(&pattern) {
            builder.add(glob);
        }
    }

    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

/// Whether `path` is absolute and holds no `..`, so a glob can tell where
/// it points without asking the disk.
pub(crate) fn is_plain_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir))
}

/// Whether `name` can be an environment variable of a manifest: ASCII
/// letters, digits and `_`, not starting with a digit.
pub(crate) fn is_variable_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let first_ok = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');

    first_ok && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// A glob over paths: `*` and `?` stay inside one folder.
fn path_glob(pattern: &str) -> Result<Glob, globset::Error> {
    GlobBuilder::new(pattern).literal_separator(true).build()
}

/// The start of `text` and the rest after it, which is empty or starts
/// with `/`.
pub(crate) fn split_start(
    text: &str,
) -> Result<(GlobStart, &str), GlobProblem> {
    if let Some(rest) = text.strip_prefix('~')
        && (rest.is_empty() || rest.starts_with('/'))
    {
        return Ok((GlobStart::Home, rest));
    }

    let Some(after_brace) = text.strip_prefix('{') else {
        return Err(GlobProblem::BadStart);
    };
    let Some((name, rest)) = after_brace.split_once('}') else {
        return Err(GlobProblem::BadStart);
    };
    if !is_variable_name(name) || !(rest.is_empty() || rest.starts_with('/')) {
        return Err(GlobProblem::BadStart);
    }

    Ok((GlobStart::Var(name.to_string()), rest))
}

/// The first `{NAME}` in `rest` that reads as a variable rather than as a
/// glob's choice such as `{json,jsonl}`.
fn variable_inside(rest: &str) -> Option<String> {
    rest.split('{').skip(1).find_map(|after_brace| {
        let (inside, _) = after_brace.split_once('}')?;
        let looks_like_variable = is_variable_name(inside)
            && inside.bytes().any(|byte| byte.is_ascii_uppercase())
            && !inside.bytes().any(|byte| byte.is_ascii_lowercase());

        looks_like_variable.then(|| inside.to_string())
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn listed(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn env() -> Environment {
        Environment::new(PathBuf::from("/home/user"))
            .with_var("CODEX_HOME", "/data/codex/")
            .with_var("RELATIVE", "data/codex")
    }

    #[test]
    fn a_glob_from_home_matches_paths_under_home() {
        let glob = PathGlob::parse("~/.codex*/**", &[]).expect("valid");
        let set = glob_set(&[glob], &env());

        assert!(set.is_match("/home/user/.codex/sessions/a.jsonl"));
        assert!(set.is_match("/home/user/.codex-work/auth.json"));
        assert!(!set.is_match("/home/other/.codex/auth.json"));
    }

    #[test]
    fn a_star_stays_inside_one_folder() {
        let glob = PathGlob::parse("~/.local/share/opencode/*.db", &[])
            .expect("valid");
        let set = glob_set(&[glob], &env());

        assert!(set.is_match("/home/user/.local/share/opencode/opencode.db"));
        assert!(!set.is_match("/home/user/.local/share/opencode/x/a.db"));
    }

    #[test]
    fn a_glob_from_a_listed_variable_matches_under_its_folder() {
        let glob = PathGlob::parse("{CODEX_HOME}/**", &listed(&["CODEX_HOME"]))
            .expect("valid");
        let set = glob_set(&[glob], &env());

        assert!(set.is_match("/data/codex/auth.json"));
    }

    #[test]
    fn a_variable_that_is_not_set_or_not_absolute_matches_nothing() {
        let unset =
            PathGlob::parse("{XDG_DATA_HOME}/**", &listed(&["XDG_DATA_HOME"]))
                .expect("valid");
        let relative = PathGlob::parse("{RELATIVE}/**", &listed(&["RELATIVE"]))
            .expect("valid");

        assert_eq!(unset.resolve(&env()), None);
        assert_eq!(relative.resolve(&env()), None);
    }

    #[test]
    fn a_glob_with_a_parent_folder_is_refused() {
        let problem =
            PathGlob::parse("~/.codex/../.ssh/*", &[]).expect_err("holds ..");

        assert_eq!(problem, GlobProblem::ParentFolder);
        assert_eq!(problem.to_string(), "contains `..`");
    }

    #[test]
    fn a_variable_the_part_does_not_list_is_refused() {
        let problem =
            PathGlob::parse("{CODEX_HOME}/**", &listed(&["XDG_DATA_HOME"]))
                .expect_err("not listed");

        assert_eq!(
            problem,
            GlobProblem::UnlistedVariable("CODEX_HOME".to_string())
        );
    }

    #[test]
    fn a_glob_must_start_from_home_or_a_variable() {
        for text in ["/etc/passwd", "**", "~user/x", "{a,b}/x", "{X}y"] {
            assert_eq!(
                PathGlob::parse(text, &listed(&["X"])),
                Err(GlobProblem::BadStart),
                "{text}"
            );
        }
    }

    #[test]
    fn a_variable_after_the_start_is_refused_but_a_choice_is_kept() {
        assert_eq!(
            PathGlob::parse("~/{CODEX_HOME}/x", &listed(&["CODEX_HOME"])),
            Err(GlobProblem::VariableInside("CODEX_HOME".to_string()))
        );
        assert!(PathGlob::parse("~/.claude/**/*.{json,jsonl}", &[]).is_ok());
    }

    #[test]
    fn a_broken_glob_is_refused() {
        let problem = PathGlob::parse("~/[abc", &[]).expect_err("broken");

        assert!(matches!(problem, GlobProblem::Syntax(_)));
    }

    #[test]
    fn glob_characters_in_the_home_folder_are_taken_literally() {
        let env = Environment::new(PathBuf::from("/home/[x]"));
        let glob = PathGlob::parse("~/a", &[]).expect("valid");
        let set = glob_set(&[glob], &env);

        assert!(set.is_match("/home/[x]/a"));
        assert!(!set.is_match("/home/x/a"));
    }

    #[test]
    fn only_absolute_paths_with_no_parent_folder_are_plain() {
        assert!(is_plain_absolute(Path::new("/home/user/a")));
        assert!(!is_plain_absolute(Path::new("home/user/a")));
        assert!(!is_plain_absolute(Path::new("/home/user/../x")));
    }
}
