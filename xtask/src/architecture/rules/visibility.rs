//! Rules about what a widget shows outside: which modules are open and what
//! `mod.rs` re-exports, in the widget and in each of its parts.

use syn::{Item, Visibility};

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    ALWAYS_OPEN_WIDGET_MODULES, OPEN_PART_MODULES, OPEN_WIDGET_MODULES,
    part_names, widget_names,
};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{line_of, use_paths};

/// A module declared in a widget or part `mod.rs`.
struct Declared {
    name: String,
    is_open: bool,
    line: usize,
}

/// Report widget and part modules that are open or closed against the
/// rule, and re-exports from open modules or from outside the folder.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for widget in widget_names(tree) {
        let parts = part_names(tree, &widget);

        if let Some(root) = tree.file(&format!("widgets/{widget}/mod.rs")) {
            findings.extend(mod_file(root, OPEN_WIDGET_MODULES, &parts));
        }
        for part in &parts {
            let part_root = format!("widgets/{widget}/{part}/mod.rs");
            if let Some(root) = tree.file(&part_root) {
                findings.extend(mod_file(root, OPEN_PART_MODULES, &[]));
            }
        }
    }

    findings
}

/// What breaks a rule in one widget or part `mod.rs`, which may open the
/// modules in `open_modules` and the folders in `parts`.
fn mod_file(
    root: &SourceFile,
    open_modules: &[&str],
    parts: &[String],
) -> Vec<Finding> {
    let modules = declared_modules(root);

    let mut findings = module_visibility(root, &modules, open_modules, parts);
    findings.extend(reexports(root, &modules));

    findings
}

/// The `mod name;` declarations at the top level of `root`.
fn declared_modules(root: &SourceFile) -> Vec<Declared> {
    root.syntax()
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(module) if module.content.is_none() => Some(Declared {
                name: module.ident.to_string(),
                is_open: !matches!(module.vis, Visibility::Inherited),
                line: line_of(module.mod_token.span),
            }),
            _ => None,
        })
        .collect()
}

fn module_visibility(
    root: &SourceFile,
    modules: &[Declared],
    open_modules: &[&str],
    parts: &[String],
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for module in modules {
        let name = module.name.as_str();
        let may_be_open = open_modules.contains(&name)
            || parts.iter().any(|part| part == name);
        if module.is_open && !may_be_open {
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

/// Re-exports from open modules or from outside the folder of `root`. A
/// re-export from a closed module is the only way out for a type such as
/// `XCtx`.
fn reexports(root: &SourceFile, modules: &[Declared]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for item in &root.syntax().items {
        let Item::Use(reexport) = item else {
            continue;
        };
        if matches!(reexport.vis, Visibility::Inherited) {
            continue;
        }

        for segments in use_paths(&reexport.tree) {
            let path = segments.join("::");
            let inside = match segments.first() {
                Some(first) if first == "self" => &segments[1..],
                _ => &segments[..],
            };
            let Some(first) = inside.first() else {
                continue;
            };
            let source = modules.iter().find(|module| &module.name == first);

            let key = match source {
                Some(module) if module.is_open => first.clone(),
                Some(_) => continue,
                None => path,
            };
            findings.push(Finding::new(
                Rule::ReexportFromOpenModule,
                root.path(),
                &key,
                line_of(reexport.use_token.span),
            ));
        }
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

    const PART_EXAMPLE: &str = "pub(crate) mod event;\npub(crate) mod types;\npub(crate) mod view;\n\
        mod checks;\nmod model;\nmod reducer;\nmod state;\n\n\
        pub(super) use model::{RunPanelViewModel, RunFormViewModel};\n\
        pub(super) use reducer::{RunCtx, RunEngine, RunOutcome, reduce};\n\
        pub(super) use state::RunState;\n";

    #[test]
    fn the_example_part_follows_the_rule() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/mod.rs",
                "pub(crate) mod event;\npub(crate) mod run;\nmod state;\npub(crate) mod view;\n",
            ),
            ("widgets/containers/run/mod.rs", PART_EXAMPLE),
        ]);
        let findings = check(&tree);

        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_widget_may_open_its_parts_but_no_other_folder() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/mod.rs",
                "pub(crate) mod event;\npub(crate) mod helpers;\npub(crate) mod run;\npub(crate) mod view;\n",
            ),
            ("widgets/containers/helpers.rs", ""),
            ("widgets/containers/run/mod.rs", PART_EXAMPLE),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetModuleVisibility),
            [pair("widgets/containers/mod.rs", "pub(crate) mod helpers")]
        );
    }

    #[test]
    fn a_model_folder_is_no_part_and_stays_closed() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/notes/mod.rs",
                "pub(crate) mod event;\npub(crate) mod model;\npub(crate) mod view;\n",
            ),
            (
                "widgets/notes/model/mod.rs",
                "pub(crate) mod notes_panel;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetModuleVisibility),
            [pair("widgets/notes/mod.rs", "pub(crate) mod model")]
        );
    }

    #[test]
    fn a_part_mod_file_follows_the_widget_lists() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/mod.rs",
                "pub(crate) mod event;\npub(crate) mod run;\npub(crate) mod view;\n",
            ),
            (
                "widgets/containers/run/mod.rs",
                "mod event;\npub(crate) mod inner;\npub(crate) mod state;\npub(crate) mod view;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetModuleVisibility),
            [
                pair("widgets/containers/run/mod.rs", "mod event"),
                pair("widgets/containers/run/mod.rs", "pub(crate) mod inner"),
                pair("widgets/containers/run/mod.rs", "pub(crate) mod state"),
            ]
        );
    }

    #[test]
    fn a_part_never_opens_services_or_storage() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/mod.rs",
                "pub(crate) mod event;\npub(crate) mod run;\npub(crate) mod services;\npub(crate) mod view;\n",
            ),
            (
                "widgets/containers/run/mod.rs",
                "pub(crate) mod event;\npub(crate) mod services;\npub(crate) mod storage;\npub(crate) mod view;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetModuleVisibility),
            [
                pair(
                    "widgets/containers/run/mod.rs",
                    "pub(crate) mod services"
                ),
                pair("widgets/containers/run/mod.rs", "pub(crate) mod storage"),
            ]
        );
    }

    #[test]
    fn re_exports_of_a_part_come_from_its_closed_modules() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/mod.rs",
                "pub(crate) mod event;\npub(crate) mod run;\npub(crate) mod view;\n",
            ),
            (
                "widgets/containers/run/mod.rs",
                "pub(crate) mod event;\nmod model;\npub(crate) mod view;\n\
                 pub(super) use model::RunFormViewModel;\n\
                 pub(super) use event::RunIntent;\n\
                 pub(super) use super::types::Place;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ReexportFromOpenModule),
            [
                pair("widgets/containers/run/mod.rs", "event"),
                pair("widgets/containers/run/mod.rs", "super::types::Place"),
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
