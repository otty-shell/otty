//! Finds the paths into this crate that a file names, in `use` items and
//! inline, and resolves them from the crate root.

use super::source::{SourceFile, SourceTree};
use super::text::{is_ident_char, join_statement};

/// Path starts that point into this crate.
const CRATE_PREFIXES: &[&str] = &["crate::", "super::", "self::"];

/// A crate path named in a source file, resolved from the crate root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CratePath {
    segments: Vec<String>,
    line: usize,
}

impl CratePath {
    /// The segments from the crate root, e.g. `["widgets", "modes",
    /// "event"]`.
    pub(crate) fn segments(&self) -> &[String] {
        &self.segments
    }

    /// The line that names the path.
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    /// Whether the path starts with `prefix`.
    pub(crate) fn starts_with(&self, prefix: &[&str]) -> bool {
        self.segments.len() >= prefix.len()
            && self
                .segments
                .iter()
                .zip(prefix)
                .all(|(seg, want)| seg == want)
    }

    /// The first `count` segments joined with `::`.
    pub(crate) fn head(&self, count: usize) -> String {
        self.segments
            .iter()
            .take(count)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("::")
    }

    /// The leading module segments: the segments up to the first one that
    /// names a type, constant or other capitalised item.
    pub(crate) fn module_head(&self, max: usize) -> String {
        let modules = self
            .segments
            .iter()
            .take_while(|segment| {
                segment
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_lowercase())
            })
            .count();

        self.head(modules.clamp(1, max))
    }
}

/// Every crate path that `file` names, in `use` items and inline.
pub(crate) fn crate_paths(
    tree: &SourceTree,
    file: &SourceFile,
) -> Vec<CratePath> {
    let children = tree.children(file.module());
    let lines = file.lines();
    let mut paths = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let text = lines[index].text().trim();
        if is_use_statement(text) {
            let (statement, next) = join_statement(lines, index);
            for written in expand_use(&statement) {
                if let Some(segments) =
                    resolve(&written, file.module(), &children)
                {
                    paths.push(CratePath {
                        segments,
                        line: lines[index].number(),
                    });
                }
            }
            index = next;
            continue;
        }

        for written in inline_paths(text) {
            if let Some(segments) = resolve(&written, file.module(), &children)
            {
                paths.push(CratePath {
                    segments,
                    line: lines[index].number(),
                });
            }
        }
        index += 1;
    }

    paths
}

/// Whether `text` starts a `use` item, re-exports included.
pub(crate) fn is_use_statement(text: &str) -> bool {
    use_body(text).is_some()
}

/// The paths of a `use` item, one per imported name, with groups and
/// `as` renames resolved: `use a::{b, c as d};` gives `a::b` and `a::c`.
pub(crate) fn expand_use(statement: &str) -> Vec<String> {
    let Some(body) = use_body(statement) else {
        return Vec::new();
    };
    let body = body.trim().trim_end_matches(';').trim();

    let Some((base, group)) = body.split_once('{') else {
        return vec![without_rename(body).to_string()];
    };
    let base = base.trim().trim_end_matches("::");
    let group = group.trim().trim_end_matches('}');

    group
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| match without_rename(item) {
            "self" => base.to_string(),
            name => format!("{base}::{name}"),
        })
        .collect()
}

/// The text after `use ` of a `use` item.
fn use_body(text: &str) -> Option<&str> {
    let trimmed = text.trim_start();
    let after_vis = trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .unwrap_or(trimmed);

    after_vis.strip_prefix("use ")
}

/// `name as other` without the rename.
fn without_rename(item: &str) -> &str {
    item.split(" as ").next().unwrap_or(item).trim()
}

/// The `crate::`, `super::` and `self::` paths written inline in `text`.
fn inline_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut at = 0;

    while at < text.len() {
        let rest = &text[at..];
        let starts_here =
            CRATE_PREFIXES.iter().any(|prefix| rest.starts_with(prefix));
        let before = text[..at].chars().next_back();
        let at_boundary =
            !before.is_some_and(|ch| is_ident_char(ch) || ch == ':');

        if starts_here && at_boundary {
            let length = rest
                .find(|ch: char| !is_ident_char(ch) && ch != ':')
                .unwrap_or(rest.len());
            let written = rest[..length].trim_end_matches(':');
            paths.push(written.to_string());
            at += length.max(1);
            continue;
        }

        at += rest.chars().next().map_or(1, char::len_utf8);
    }

    paths
}

/// Resolve `written` from the crate root, as seen from `module`; `None`
/// for a path into another crate.
fn resolve(
    written: &str,
    module: &[String],
    children: &[&str],
) -> Option<Vec<String>> {
    let segments: Vec<&str> =
        written.split("::").filter(|s| !s.is_empty()).collect();
    let first = *segments.first()?;

    let resolved: Vec<String> = match first {
        "crate" => segments[1..].iter().map(|s| s.to_string()).collect(),
        "self" => module
            .iter()
            .cloned()
            .chain(segments[1..].iter().map(|s| s.to_string()))
            .collect(),
        "super" => {
            let ups = segments.iter().take_while(|s| **s == "super").count();
            let kept = module.len().checked_sub(ups)?;
            module[..kept]
                .iter()
                .cloned()
                .chain(segments[ups..].iter().map(|s| s.to_string()))
                .collect()
        },
        name if children.contains(&name) => module
            .iter()
            .cloned()
            .chain(segments.iter().map(|s| s.to_string()))
            .collect(),
        _ => return None,
    };

    if resolved.is_empty() {
        return None;
    }

    Some(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(sources: &[(&str, &str)], path: &str) -> Vec<String> {
        let tree = SourceTree::from_sources(sources);
        let file = tree.file(path).expect("the file is in the tree");

        crate_paths(&tree, file)
            .iter()
            .map(|path| path.segments().join("::"))
            .collect()
    }

    #[test]
    fn a_group_import_gives_one_path_per_name() {
        assert_eq!(
            expand_use("use crate::i18n::{self, Key as K};"),
            ["crate::i18n", "crate::i18n::Key"]
        );
        assert_eq!(
            expand_use("pub(crate) use event::{NotesEvent, NotesIntent};"),
            ["event::NotesEvent", "event::NotesIntent"]
        );
    }

    #[test]
    fn super_self_and_child_paths_resolve_from_the_crate_root() {
        let paths = resolved(
            &[
                ("widgets/notes/mod.rs", "use state::NotesState;\n"),
                ("widgets/notes/state.rs", ""),
                (
                    "widgets/notes/view/panel.rs",
                    "use super::super::event::NotesIntent;\nuse crate::theme::ThemeProps;\n",
                ),
            ],
            "widgets/notes/view/panel.rs",
        );

        assert_eq!(
            paths,
            ["widgets::notes::event::NotesIntent", "theme::ThemeProps"]
        );
    }

    #[test]
    fn a_multi_line_import_is_read_as_one() {
        let paths = resolved(
            &[(
                "router/tabs.rs",
                "use crate::widgets::terminal_workspace::{\n    TerminalWorkspaceEvent, TerminalWorkspaceIntent,\n};\n",
            )],
            "router/tabs.rs",
        );

        assert_eq!(paths.len(), 2);
        assert!(paths[0].starts_with("widgets::terminal_workspace::"));
    }

    #[test]
    fn inline_paths_are_found_and_other_crates_are_skipped() {
        let paths = resolved(
            &[(
                "view.rs",
                "let p = crate::widgets::containers::types::Place::new(1);\nuse iced::Task;\n",
            )],
            "view.rs",
        );

        assert_eq!(paths, ["widgets::containers::types::Place::new"]);
    }

    #[test]
    fn the_module_head_stops_at_the_first_item() {
        let path = CratePath {
            segments: ["widgets", "modes", "types", "Mode"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            line: 1,
        };

        assert_eq!(path.module_head(3), "widgets::modes::types");
        assert_eq!(path.module_head(2), "widgets::modes");
    }
}
