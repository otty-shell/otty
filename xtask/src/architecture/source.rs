//! Reads the `app` sources and parses them with `syn`, keeping only the
//! production code the rules look at: test-only items are left out.

use std::path::Path;
use std::{fs, io};

use syn::{Expr, ExprLit, File, Item, Lit, Meta};

use super::syntax::line_of;
use super::test_code::remove_test_code;

/// The folder of test helpers, which the rules do not cover.
const TESTING_DIR: &str = "testing/";

/// A parsed source file of `app/src`, with test code left out.
pub(crate) struct SourceFile {
    path: String,
    module: Vec<String>,
    syntax: File,
}

impl SourceFile {
    /// The path relative to `app/src`, with `/` between folders.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The module path from the crate root, e.g. `["widgets", "modes"]`
    /// for `widgets/modes/mod.rs`.
    pub(crate) fn module(&self) -> &[String] {
        &self.module
    }

    /// The syntax tree of the production code.
    pub(crate) fn syntax(&self) -> &File {
        &self.syntax
    }

    /// The file name, e.g. `state.rs`.
    pub(crate) fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Parse `text`, leaving out test-only items. A file that is no valid
    /// Rust gives an error that names the file and the line.
    pub(crate) fn parse(path: &str, text: &str) -> io::Result<Self> {
        let mut syntax = syn::parse_file(text).map_err(|err| {
            let line = line_of(err.span());
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("cannot parse {path}, line {line}: {err}"),
            )
        })?;
        remove_test_code(&mut syntax);

        Ok(Self {
            path: path.to_string(),
            module: default_module(path),
            syntax,
        })
    }
}

/// All production source files of `app/src`, except the test helpers in
/// `testing/`.
pub(crate) struct SourceTree {
    files: Vec<SourceFile>,
}

impl SourceTree {
    /// Every file, sorted by path.
    pub(crate) fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// The file at `path`, if there is one.
    pub(crate) fn file(&self, path: &str) -> Option<&SourceFile> {
        self.files.iter().find(|file| file.path == path)
    }

    /// Whether any file lies inside the folder `dir`.
    pub(crate) fn has_dir(&self, dir: &str) -> bool {
        let prefix = format!("{dir}/");

        self.files.iter().any(|file| file.path.starts_with(&prefix))
    }

    /// The names of the modules declared directly inside `module`.
    pub(crate) fn children(&self, module: &[String]) -> Vec<&str> {
        self.files
            .iter()
            .filter(|file| {
                file.module.len() == module.len() + 1
                    && file.module.starts_with(module)
            })
            .filter_map(|file| file.module.last().map(String::as_str))
            .collect()
    }

    /// Read every `.rs` file under `root`, which is `app/src`.
    pub(crate) fn read(root: &Path) -> io::Result<Self> {
        let mut sources = Vec::new();
        read_dir_sources(root, root, &mut sources)?;

        Ok(Self::from_parsed(sources))
    }

    /// Build a tree from parsed files, sorting them and applying the
    /// `#[path]` attributes to their module paths.
    fn from_parsed(mut files: Vec<SourceFile>) -> Self {
        files.retain(|file| !file.path.starts_with(TESTING_DIR));
        files.sort_by(|left, right| left.path.cmp(&right.path));

        let overrides = path_overrides(&files);
        for file in &mut files {
            if let Some(module) = overrides
                .iter()
                .find(|(path, _)| *path == file.path)
                .map(|(_, module)| module.clone())
            {
                file.module = module;
            }
        }

        Self { files }
    }

    #[cfg(test)]
    pub(crate) fn from_sources(sources: &[(&str, &str)]) -> Self {
        let files = sources
            .iter()
            .map(|(path, text)| {
                SourceFile::parse(path, text)
                    .unwrap_or_else(|err| panic!("{err}"))
            })
            .collect();

        Self::from_parsed(files)
    }
}

/// The module path a file has without `#[path]`: folders, then the file
/// name, with `mod.rs` standing for its folder and `main.rs` for the root.
fn default_module(path: &str) -> Vec<String> {
    let without_ext = path.strip_suffix(".rs").unwrap_or(path);
    let mut segments: Vec<String> =
        without_ext.split('/').map(str::to_string).collect();

    if segments.last().is_some_and(|last| last == "mod") || path == "main.rs" {
        segments.pop();
    }

    segments
}

/// The module paths set by `#[path = "..."]` on the `mod name;`
/// declarations at the top of each file, as `(file path, module path)`
/// pairs.
fn path_overrides(files: &[SourceFile]) -> Vec<(String, Vec<String>)> {
    let mut overrides = Vec::new();

    for file in files {
        let dir = match file.path.rfind('/') {
            Some(end) => &file.path[..=end],
            None => "",
        };
        for item in &file.syntax.items {
            let Item::Mod(module) = item else {
                continue;
            };
            let Some(target) = path_attribute(module) else {
                continue;
            };

            let mut path = file.module.clone();
            path.push(module.ident.to_string());
            overrides.push((format!("{dir}{target}"), path));
        }
    }

    overrides
}

/// The file named by `#[path = "..."]` on a module declaration.
fn path_attribute(module: &syn::ItemMod) -> Option<String> {
    module.attrs.iter().find_map(|attr| match &attr.meta {
        Meta::NameValue(pair) if pair.path.is_ident("path") => {
            match &pair.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(target),
                    ..
                }) => Some(target.value()),
                _ => None,
            }
        },
        _ => None,
    })
}

/// Collect the parsed `.rs` files under `dir`, with paths relative to
/// `root`.
fn read_dir_sources(
    root: &Path,
    dir: &Path,
    sources: &mut Vec<SourceFile>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            read_dir_sources(root, &path, sources)?;
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .map_err(io::Error::other)?
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let text = fs::read_to_string(&path)?;
        sources.push(SourceFile::parse(&relative, &text)?);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names of the top-level items of `file`, functions and modules.
    fn item_names(file: &SourceFile) -> Vec<String> {
        file.syntax()
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Fn(function) => Some(function.sig.ident.to_string()),
                Item::Mod(module) => Some(module.ident.to_string()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_test_module_and_its_attributes_are_left_out() {
        let file = SourceFile::parse(
            "widgets/notes/reducer.rs",
            "fn reduce() {}\n\
             #[cfg(test)]\n\
             #[expect(\n    clippy::disallowed_methods,\n    reason = \"x\"\n)]\n\
             mod tests {\n    use crate::widgets::tabs::state::TabsState;\n}\n\
             fn after() {}\n",
        )
        .expect("the file parses");

        assert_eq!(item_names(&file), ["reduce", "after"]);
    }

    #[test]
    fn a_test_only_declaration_on_one_line_is_left_out() {
        let file = SourceFile::parse(
            "main.rs",
            "#[cfg(test)]\nmod testing;\nmod theme;\n",
        )
        .expect("the file parses");

        assert_eq!(item_names(&file), ["theme"]);
    }

    #[test]
    fn a_file_that_is_no_valid_rust_names_the_file_and_the_line() {
        let Err(err) =
            SourceFile::parse("widgets/notes/state.rs", "fn a() {}\nfn b(\n")
        else {
            panic!("the file parses");
        };

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(
            err.to_string()
                .starts_with("cannot parse widgets/notes/state.rs, line "),
            "{err}"
        );
    }

    #[test]
    fn modules_follow_folders_and_mod_files() {
        let tree = SourceTree::from_sources(&[
            ("main.rs", "mod widgets;\n"),
            ("widgets/modes/mod.rs", "mod state;\n"),
            ("widgets/modes/state.rs", ""),
        ]);

        let modules: Vec<&[String]> =
            tree.files().iter().map(SourceFile::module).collect();

        assert_eq!(modules[0], Vec::<String>::new());
        assert_eq!(modules[1], ["widgets", "modes"]);
        assert_eq!(modules[2], ["widgets", "modes", "state"]);
    }

    #[test]
    fn a_path_attribute_moves_a_file_into_the_declaring_module() {
        let tree = SourceTree::from_sources(&[
            (
                "app.rs",
                "#[path = \"update.rs\"]\nmod update;\n\
                 /// The root view.\n#[path = \"view.rs\"]\npub(crate) mod view;\n",
            ),
            ("update.rs", ""),
            ("view.rs", ""),
        ]);

        let update = tree.file("update.rs").expect("update.rs is in the tree");
        let view = tree.file("view.rs").expect("view.rs is in the tree");

        assert_eq!(update.module(), ["app", "update"]);
        assert_eq!(view.module(), ["app", "view"]);
    }

    #[test]
    fn test_helpers_are_not_in_the_tree() {
        let tree = SourceTree::from_sources(&[
            ("testing/mod.rs", "use crate::widgets::tabs::state::X;\n"),
            ("theme.rs", ""),
        ]);

        let paths: Vec<&str> =
            tree.files().iter().map(SourceFile::path).collect();

        assert_eq!(paths, ["theme.rs"]);
    }

    #[test]
    fn children_are_the_modules_one_level_down() {
        let tree = SourceTree::from_sources(&[
            ("widgets/modes/mod.rs", ""),
            ("widgets/modes/state.rs", ""),
            ("widgets/modes/view/mod.rs", ""),
            ("widgets/modes/view/menu.rs", ""),
        ]);
        let module = vec!["widgets".to_string(), "modes".to_string()];

        let mut children = tree.children(&module);
        children.sort();

        assert_eq!(children, ["state", "view"]);
    }
}
