//! Finds the paths into this crate that a file names, in `use` items and
//! inline, and resolves them from the crate root.

use super::source::{SourceFile, SourceTree};
use super::syntax::written_paths;

/// Path starts that point into this crate.
const CRATE_PREFIXES: &[&str] = &["crate", "super", "self"];

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

/// Every crate path that `file` names, in `use` items and inline. An
/// import may also start with a module declared next to the file; an inline
/// path counts only when it starts with `crate::`, `super::` or `self::`.
pub(crate) fn crate_paths(
    tree: &SourceTree,
    file: &SourceFile,
) -> Vec<CratePath> {
    let children = tree.children(file.module());

    written_paths(file.syntax())
        .iter()
        .filter_map(|written| {
            let segments = written.segments();
            let resolved = if written.is_import() {
                resolve(segments, file.module(), &children)
            } else if segments.len() > 1
                && CRATE_PREFIXES.contains(&segments[0].as_str())
            {
                resolve(segments, file.module(), &[])
            } else {
                None
            }?;

            Some(CratePath {
                segments: resolved,
                line: written.line(),
            })
        })
        .collect()
}

/// Resolve the segments `written` from the crate root, as seen from
/// `module`, where `children` are the modules declared inside it; `None`
/// for a path into another crate.
fn resolve(
    written: &[String],
    module: &[String],
    children: &[&str],
) -> Option<Vec<String>> {
    let first = written.first()?.as_str();

    let resolved: Vec<String> = match first {
        "crate" => written[1..].to_vec(),
        "self" => [module, &written[1..]].concat(),
        "super" => {
            let ups = written.iter().take_while(|s| *s == "super").count();
            let kept = module.len().checked_sub(ups)?;
            [&module[..kept], &written[ups..]].concat()
        },
        name if children.contains(&name) => [module, written].concat(),
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
        let paths = resolved(
            &[
                (
                    "widgets/notes/mod.rs",
                    "use crate::i18n::{self, Key as K};\n\
                     pub(crate) use event::{NotesEvent, NotesIntent};\n",
                ),
                ("widgets/notes/event.rs", ""),
            ],
            "widgets/notes/mod.rs",
        );

        assert_eq!(
            paths,
            [
                "i18n",
                "i18n::Key",
                "widgets::notes::event::NotesEvent",
                "widgets::notes::event::NotesIntent",
            ]
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
                "fn f() {\n    let p = crate::widgets::containers::types::Place::new(1);\n    \
                 let q = column![super::theme::size()];\n    let r = self.state;\n}\nuse iced::Task;\n",
            )],
            "view.rs",
        );

        assert_eq!(
            paths,
            ["widgets::containers::types::Place::new", "theme::size"]
        );
    }

    #[test]
    fn an_inline_path_from_a_child_module_is_not_resolved() {
        let paths = resolved(
            &[
                (
                    "widgets/notes/mod.rs",
                    "use state::NotesState;\nfn f() {\n    state::reset();\n}\n",
                ),
                ("widgets/notes/state.rs", ""),
            ],
            "widgets/notes/mod.rs",
        );

        assert_eq!(paths, ["widgets::notes::state::NotesState"]);
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
