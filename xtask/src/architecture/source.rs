//! Reads the `app` sources as plain text and keeps only the production code
//! the rules look at: test-only items and comment lines are left out.

use std::path::Path;
use std::{fs, io};

use super::text::{indent_of, strip_trailing_comment};

/// The folder of test helpers, which the rules do not cover.
const TESTING_DIR: &str = "testing/";

/// One production line of a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    number: usize,
    text: String,
}

impl Line {
    /// The line number in the file, counting from 1.
    pub(crate) fn number(&self) -> usize {
        self.number
    }

    /// The code on the line, without a trailing comment.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    #[cfg(test)]
    pub(crate) fn new(number: usize, text: &str) -> Self {
        Self {
            number,
            text: text.to_string(),
        }
    }
}

/// A source file of `app/src`, with test code and comment lines left out.
#[derive(Debug, Clone)]
pub(crate) struct SourceFile {
    path: String,
    module: Vec<String>,
    lines: Vec<Line>,
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

    /// The production lines, in order.
    pub(crate) fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The file name, e.g. `state.rs`.
    pub(crate) fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// Parse `text`, leaving out `#[cfg(test)]` items and comment lines.
    pub(crate) fn parse(path: &str, text: &str) -> Self {
        let raw: Vec<&str> = text.lines().collect();
        let mut lines = Vec::new();
        let mut index = 0;

        while index < raw.len() {
            let trimmed = raw[index].trim();
            if trimmed == "#[cfg(test)]" {
                index = end_of_test_item(&raw, index);
                continue;
            }

            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                lines.push(Line {
                    number: index + 1,
                    text: strip_trailing_comment(raw[index]).to_string(),
                });
            }
            index += 1;
        }

        Self {
            path: path.to_string(),
            module: default_module(path),
            lines,
        }
    }
}

/// All production source files of `app/src`, except the test helpers in
/// `testing/`.
#[derive(Debug, Clone)]
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
            .map(|(path, text)| SourceFile::parse(path, text))
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

/// The module paths set by `#[path = "..."]` attributes, as
/// `(file path, module path)` pairs.
fn path_overrides(files: &[SourceFile]) -> Vec<(String, Vec<String>)> {
    let mut overrides = Vec::new();

    for file in files {
        let dir = match file.path.rfind('/') {
            Some(end) => &file.path[..=end],
            None => "",
        };
        let lines = file.lines();
        for (index, line) in lines.iter().enumerate() {
            let Some(target) = path_attribute(line.text()) else {
                continue;
            };
            let Some(name) = lines
                .get(index + 1)
                .and_then(|next| declared_module(next.text()))
            else {
                continue;
            };

            let mut module = file.module.clone();
            module.push(name.to_string());
            overrides.push((format!("{dir}{target}"), module));
        }
    }

    overrides
}

/// The file named by a `#[path = "..."]` attribute.
fn path_attribute(text: &str) -> Option<&str> {
    let rest = text.trim().strip_prefix("#[path = \"")?;

    rest.strip_suffix("\"]")
}

/// The module name of a `mod name;` declaration, open or closed.
pub(crate) fn declared_module(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    let rest = trimmed
        .strip_prefix("pub(crate) mod ")
        .or_else(|| trimmed.strip_prefix("pub(super) mod "))
        .or_else(|| trimmed.strip_prefix("pub mod "))
        .or_else(|| trimmed.strip_prefix("mod "))?;

    rest.strip_suffix(';')
}

/// The index just after the `#[cfg(test)]` item that starts at `start`.
fn end_of_test_item(raw: &[&str], start: usize) -> usize {
    let indent = indent_of(raw[start]);
    let mut index = start + 1;

    // Other attributes and doc comments of the same item.
    while index < raw.len() {
        let trimmed = raw[index].trim();
        if trimmed.starts_with("//") {
            index += 1;
            continue;
        }
        if !trimmed.starts_with("#[") {
            break;
        }
        while index < raw.len() && !raw[index].trim().ends_with(']') {
            index += 1;
        }
        index += 1;
    }

    let Some(item) = raw.get(index) else {
        return index;
    };
    let trimmed = item.trim();
    let opens = trimmed.matches(['{', '[', '(']).count();
    let closes = trimmed.matches(['}', ']', ')']).count();
    if opens == closes {
        return index + 1;
    }

    let mut end = index + 1;
    while end < raw.len() {
        let line = raw[end];
        let first = line.trim_start().chars().next();
        if indent_of(line) == indent
            && matches!(first, Some('}') | Some(']') | Some(')'))
        {
            return end + 1;
        }
        end += 1;
    }

    end
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
        sources.push(SourceFile::parse(&relative, &text));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(file: &SourceFile) -> Vec<&str> {
        file.lines().iter().map(Line::text).collect()
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
        );

        assert_eq!(texts(&file), ["fn reduce() {}", "fn after() {}"]);
    }

    #[test]
    fn a_test_only_method_inside_an_impl_is_left_out() {
        let file = SourceFile::parse(
            "widgets/notes/state.rs",
            "impl NotesState {\n    \
             #[cfg(test)]\n    \
             pub(crate) fn text(&self) -> &str {\n        &self.text\n    }\n\n    \
             pub(crate) fn kept(&self) {}\n}\n",
        );

        assert_eq!(
            texts(&file),
            ["impl NotesState {", "    pub(crate) fn kept(&self) {}", "}"]
        );
    }

    #[test]
    fn a_test_only_declaration_on_one_line_is_left_out() {
        let file = SourceFile::parse(
            "main.rs",
            "#[cfg(test)]\nmod testing;\nmod theme;\n",
        );

        assert_eq!(texts(&file), ["mod theme;"]);
    }

    #[test]
    fn comment_lines_and_trailing_comments_are_left_out() {
        let file = SourceFile::parse(
            "theme.rs",
            "//! Module doc.\n/// Item doc.\nfn a() {} // note\nlet s = \"a // b\";\n",
        );

        assert_eq!(texts(&file), ["fn a() {}", "let s = \"a // b\";"]);
        assert_eq!(file.lines()[0].number(), 3);
    }

    #[test]
    fn modules_follow_folders_and_mod_files() {
        let tree = SourceTree::from_sources(&[
            ("main.rs", "mod widgets;"),
            ("widgets/modes/mod.rs", "mod state;"),
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
            ("app.rs", "#[path = \"update.rs\"]\nmod update;\n"),
            ("update.rs", ""),
        ]);

        let update = tree.file("update.rs").expect("update.rs is in the tree");

        assert_eq!(update.module(), ["app", "update"]);
    }

    #[test]
    fn test_helpers_are_not_in_the_tree() {
        let tree = SourceTree::from_sources(&[
            ("testing/mod.rs", "use crate::widgets::tabs::state::X;"),
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
