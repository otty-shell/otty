//! Rules about what a widget shows outside: which modules are open and what
//! `mod.rs` re-exports.

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    ALWAYS_OPEN_WIDGET_MODULES, OPEN_WIDGET_MODULES, widget_names,
};
use super::super::paths::expand_use;
use super::super::source::{SourceFile, SourceTree, declared_module};
use super::super::text::join_statement;

/// Report widget modules that are open or closed against the rule, and
/// re-exports from open modules or from outside the widget.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for widget in widget_names(tree) {
        let Some(root) = tree.file(&format!("widgets/{widget}/mod.rs")) else {
            continue;
        };
        let modules = declared_modules(root);

        findings.extend(module_visibility(root, &modules));
        findings.extend(reexports(root, &modules));
    }

    findings
}

/// A module declared in a widget `mod.rs`.
struct Declared {
    name: String,
    is_open: bool,
    line: usize,
}

/// The modules declared at the top level of `root`.
fn declared_modules(root: &SourceFile) -> Vec<Declared> {
    root.lines()
        .iter()
        .filter(|line| !line.text().starts_with(' '))
        .filter_map(|line| {
            let name = declared_module(line.text())?;

            Some(Declared {
                name: name.to_string(),
                is_open: !line.text().trim_start().starts_with("mod "),
                line: line.number(),
            })
        })
        .collect()
}

fn module_visibility(root: &SourceFile, modules: &[Declared]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for module in modules {
        let name = module.name.as_str();
        if module.is_open && !OPEN_WIDGET_MODULES.contains(&name) {
            let key = format!("pub(crate) mod {name}");
            findings.push(finding(root, &key, module.line));
        }
        if !module.is_open && ALWAYS_OPEN_WIDGET_MODULES.contains(&name) {
            let key = format!("mod {name}");
            findings.push(finding(root, &key, module.line));
        }
    }

    findings
}

/// Re-exports from open modules or from outside the widget. A re-export
/// from a closed module is the only way out for a type such as `XCtx`.
fn reexports(root: &SourceFile, modules: &[Declared]) -> Vec<Finding> {
    let lines = root.lines();
    let mut findings = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let text = lines[index].text();
        if !text.starts_with("pub(crate) use ") && !text.starts_with("pub use ")
        {
            index += 1;
            continue;
        }

        let (statement, next) = join_statement(lines, index);
        for path in expand_use(&statement) {
            let inside = path.strip_prefix("self::").unwrap_or(&path);
            let first = inside.split("::").next().unwrap_or(inside);
            let source = modules.iter().find(|module| module.name == first);

            let key = match source {
                Some(module) if module.is_open => first.to_string(),
                Some(_) => continue,
                None => path.clone(),
            };
            findings.push(Finding::new(
                Rule::ReexportFromOpenModule,
                root.path(),
                &key,
                lines[index].number(),
            ));
        }
        index = next;
    }

    findings
}

fn finding(root: &SourceFile, key: &str, line: usize) -> Finding {
    Finding::new(Rule::WidgetModuleVisibility, root.path(), key, line)
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    const EXAMPLE: &str = "pub(crate) mod event;\nmod model;\nmod reducer;\nmod state;\n\
        #[expect(\n    clippy::disallowed_methods,\n    reason = \"this module reads the note\"\n)]\n\
        pub(crate) mod storage;\npub(crate) mod view;\n\npub(crate) use reducer::NotesCtx;\n\
        use event::{NotesEvent, NotesIntent};\n";

    #[test]
    fn the_example_widget_follows_the_rule() {
        let tree =
            SourceTree::from_sources(&[("widgets/notes/mod.rs", EXAMPLE)]);
        let findings = check(&tree);

        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_open_state_and_a_closed_event_are_reported() {
        let tree = SourceTree::from_sources(&[(
            "widgets/notes/mod.rs",
            "mod event;\npub(crate) mod state;\npub(crate) mod view;\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetModuleVisibility),
            [
                pair("widgets/notes/mod.rs", "mod event"),
                pair("widgets/notes/mod.rs", "pub(crate) mod state"),
            ]
        );
    }

    #[test]
    fn re_exports_from_open_modules_or_outside_are_reported() {
        let tree = SourceTree::from_sources(&[(
            "widgets/notes/mod.rs",
            "pub(crate) mod event;\nmod model;\npub(crate) mod view;\n\
             pub(crate) use self::event::{NotesEvent, NotesIntent};\n\
             pub(crate) use model::NotesViewModel;\n\
             pub(crate) use crate::domain::mode::Mode;\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::ReexportFromOpenModule),
            [
                pair("widgets/notes/mod.rs", "crate::domain::mode::Mode"),
                pair("widgets/notes/mod.rs", "event"),
            ]
        );
    }
}
