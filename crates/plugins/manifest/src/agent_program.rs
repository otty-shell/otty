//! Where an agent part's program is installed and how to install it
//! (issue 32): the places its official installer puts the program, which
//! OTTY checks when the login shell does not find it by its name, and the
//! command or page that installs it.

use std::path::{Path, PathBuf};

use crate::environment::Environment;
use crate::path_glob::{GlobStart, is_plain_absolute, split_start};

/// One place an agent's installer puts its program, such as
/// `~/.opencode/bin/opencode`: from home (`~`), from a variable its part
/// lists in `env` (`{XDG_DATA_HOME}`) or from the root (`/`), one file and
/// no `..`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramPlace {
    text: String,
    start: PlaceStart,
    rest: String,
}

impl ProgramPlace {
    /// The place as the manifest writes it.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The place on this computer, with `~` or the variable replaced by
    /// its folder; `None` when the variable is not set or does not hold an
    /// absolute path with no `..`.
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    ///
    /// use otty_plugin_manifest::{Environment, Manifest};
    ///
    /// let manifest = Manifest::parse(
    ///     r#"
    ///     [plugin]
    ///     id = "example"
    ///     name = "Example"
    ///     version = "0.1.0"
    ///     wasm = "example.wasm"
    ///
    ///     [api]
    ///     agents.agent = "0.1"
    ///
    ///     [[agents.agent]]
    ///     id = "example-agent"
    ///     name = "Example agent"
    ///     icon = "example.svg"
    ///     program_places = ["~/.example/bin/example-agent"]
    ///     "#,
    /// )
    /// .expect("a valid manifest");
    /// let env = Environment::new(PathBuf::from("/home/user"));
    ///
    /// let place = &manifest.agents()[0].program_places()[0];
    ///
    /// assert_eq!(
    ///     place.path_on(&env).as_deref(),
    ///     Some(Path::new("/home/user/.example/bin/example-agent"))
    /// );
    /// ```
    pub fn path_on(&self, env: &Environment) -> Option<PathBuf> {
        let base = match &self.start {
            PlaceStart::Root => return Some(PathBuf::from(&self.rest)),
            PlaceStart::Glob(GlobStart::Home) => env.home().to_str()?,
            PlaceStart::Glob(GlobStart::Var(name)) => env.var(name)?,
        };
        if !is_plain_absolute(Path::new(base)) {
            return None;
        }

        let base = base.trim_end_matches('/');

        Some(PathBuf::from(format!("{base}{}", self.rest)))
    }

    /// The place in `text`, checked against the variables `listed_vars` of
    /// its part; the error says why it is refused, in words that follow
    /// "which".
    pub(crate) fn parse(
        text: &str,
        listed_vars: &[String],
    ) -> Result<Self, String> {
        if text.split('/').any(|segment| segment == "..") {
            return Err(String::from("contains `..`"));
        }
        if text.contains(['*', '?', '[', ']'])
            || text.chars().skip(1).any(|c| c == '{')
        {
            return Err(String::from(
                "holds a pattern; a program place names one file",
            ));
        }

        let (start, rest) = match text.strip_prefix('/') {
            Some(_) => (PlaceStart::Root, text),
            None => {
                let (start, rest) = split_start(text).map_err(|_| {
                    String::from(
                        "must start with `~`, `/` or a variable such as \
                         `{XDG_DATA_HOME}`",
                    )
                })?;
                (PlaceStart::Glob(start), rest)
            },
        };
        if let PlaceStart::Glob(GlobStart::Var(name)) = &start
            && !listed_vars.contains(name)
        {
            return Err(format!(
                "uses `{{{name}}}`, which the part does not list in `env`"
            ));
        }
        if rest.trim_end_matches('/').is_empty() || rest.ends_with('/') {
            return Err(String::from("names no file"));
        }

        Ok(Self {
            text: text.to_string(),
            start,
            rest: rest.to_string(),
        })
    }
}

/// What a program place starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PlaceStart {
    /// `/`: the place is already absolute.
    Root,
    /// `~` or a variable, as a permission glob starts.
    Glob(GlobStart),
}

/// How to install an agent's program: a command that runs in a terminal
/// tab the user sees, a page about it, or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramInstall {
    command: Vec<String>,
    page: Option<String>,
}

impl ProgramInstall {
    /// The program and its arguments, such as `["sh", "-c", "curl … |
    /// sh"]`; empty when only a page is given.
    pub fn command(&self) -> &[String] {
        &self.command
    }

    /// The `https` page about installing the program.
    pub fn page(&self) -> Option<&str> {
        self.page.as_deref()
    }

    /// The install of `command` and `page`, checked; the error says why it
    /// is refused, in words that follow the part's name.
    pub(crate) fn new(
        command: &[String],
        page: Option<&str>,
    ) -> Result<Self, String> {
        if command.is_empty() && page.is_none() {
            return Err(String::from(
                "has an install with no command and no page",
            ));
        }
        if command
            .first()
            .is_some_and(|program| program.trim().is_empty())
        {
            return Err(String::from(
                "has an install command with an empty program",
            ));
        }
        if let Some(page) = page
            && !page.starts_with("https://")
        {
            return Err(format!(
                "has the install page `{page}`, not an https:// link"
            ));
        }

        Ok(Self {
            command: command.to_vec(),
            page: page.map(String::from),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Environment {
        Environment::new(PathBuf::from("/home/user/"))
            .with_var("XDG_DATA_HOME", "/data/xdg/")
            .with_var("RELATIVE", "data/xdg")
    }

    fn place(text: &str) -> ProgramPlace {
        let listed = ["XDG_DATA_HOME", "RELATIVE", "UNSET"].map(String::from);

        ProgramPlace::parse(text, &listed).expect("a valid place")
    }

    #[test]
    fn a_place_is_made_concrete_for_this_computer() {
        assert_eq!(
            place("~/.opencode/bin/opencode").path_on(&env()),
            Some(PathBuf::from("/home/user/.opencode/bin/opencode"))
        );
        assert_eq!(
            place("{XDG_DATA_HOME}/opencode/bin/opencode").path_on(&env()),
            Some(PathBuf::from("/data/xdg/opencode/bin/opencode"))
        );
        assert_eq!(
            place("/opt/opencode/bin/opencode").path_on(&env()),
            Some(PathBuf::from("/opt/opencode/bin/opencode"))
        );
    }

    #[test]
    fn a_variable_that_is_not_set_or_not_absolute_is_no_place() {
        assert_eq!(place("{UNSET}/bin/opencode").path_on(&env()), None);
        assert_eq!(place("{RELATIVE}/bin/opencode").path_on(&env()), None);
    }

    #[test]
    fn a_place_keeps_glob_free_characters_of_the_home_folder() {
        let env = Environment::new(PathBuf::from("/home/a b"));

        assert_eq!(
            place("~/.local/bin/claude").path_on(&env),
            Some(PathBuf::from("/home/a b/.local/bin/claude"))
        );
    }
}
