//! Rules about what a widget shows outside: which modules are open and what
//! `mod.rs` re-exports, in the widget and in each of its child widgets, and
//! what the `child/mod.rs` between them declares.

use syn::{Item, Visibility};

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    ALWAYS_OPEN_WIDGET_MODULES, CHILD_DIR, OPEN_CHILD_MODULES,
    OPEN_WIDGET_MODULES, child_names, widget_names,
};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{line_of, path_names, use_paths};

/// A module declared in a widget or child widget `mod.rs`.
struct Declared {
    name: String,
    is_open: bool,
    line: usize,
}

/// Report widget and child widget modules that are open or closed against
/// the rule, re-exports from open modules or from outside the folder, and
/// anything in a `child/mod.rs` but the child widgets.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for widget in widget_names(tree) {
        let children = child_names(tree, &widget);

        if let Some(root) = tree.file(&format!("widgets/{widget}/mod.rs")) {
            findings.extend(mod_file(root, OPEN_WIDGET_MODULES));
        }
        let child_mod = format!("widgets/{widget}/{CHILD_DIR}/mod.rs");
        if let Some(root) = tree.file(&child_mod) {
            findings.extend(child_mod_file(root, &children));
        }
        for child in &children {
            let child_root =
                format!("widgets/{widget}/{CHILD_DIR}/{child}/mod.rs");
            if let Some(root) = tree.file(&child_root) {
                findings.extend(mod_file(root, OPEN_CHILD_MODULES));
            }
        }
    }

    findings
}

/// What breaks a rule in one widget or child widget `mod.rs`, which may
/// open the modules in `open_modules`.
fn mod_file(root: &SourceFile, open_modules: &[&str]) -> Vec<Finding> {
    let modules = declared_modules(root);

    let mut findings = module_visibility(root, &modules, open_modules);
    findings.extend(reexports(root, &modules));

    findings
}

/// The items of a `child/mod.rs` other than one `pub(crate) mod <child>;`
/// for each folder of `children`, keyed as written, e.g. `mod run` or
/// `fn shared`.
fn child_mod_file(root: &SourceFile, children: &[String]) -> Vec<Finding> {
    root.syntax()
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Mod(module) = item
                && module.content.is_none()
                && is_crate_visible(&module.vis)
                && children.iter().any(|child| module.ident == child)
            {
                return None;
            }

            let (key, line) = item_key(item);
            Some(Finding::new(
                Rule::ChildModDeclaresOther,
                root.path(),
                &key,
                line,
            ))
        })
        .collect()
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
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for module in modules {
        let name = module.name.as_str();
        if module.is_open && !open_modules.contains(&name) {
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

/// Whether `visibility` is exactly `pub(crate)`.
fn is_crate_visible(visibility: &Visibility) -> bool {
    match visibility {
        Visibility::Restricted(restricted) => {
            restricted.in_token.is_none() && restricted.path.is_ident("crate")
        },
        _ => false,
    }
}

/// The visibility as written before an item, e.g. `pub(crate) `, or
/// nothing for a private item.
fn visibility_text(visibility: &Visibility) -> String {
    match visibility {
        Visibility::Public(_) => "pub ".to_string(),
        Visibility::Restricted(restricted) => {
            let path = path_names(&restricted.path).join("::");
            match restricted.in_token {
                Some(_) => format!("pub(in {path}) "),
                None => format!("pub({path}) "),
            }
        },
        Visibility::Inherited => String::new(),
    }
}

/// How a finding names `item`, with the line where it starts: `mod` and its
/// visibility for a module, the kind and the name for a function or a type,
/// and the kind alone for anything else.
fn item_key(item: &Item) -> (String, usize) {
    match item {
        Item::Mod(module) => (
            format!("{}mod {}", visibility_text(&module.vis), module.ident),
            line_of(module.mod_token.span),
        ),
        Item::Fn(function) => (
            format!("fn {}", function.sig.ident),
            line_of(function.sig.ident.span()),
        ),
        Item::Struct(item) => {
            (format!("struct {}", item.ident), line_of(item.ident.span()))
        },
        Item::Enum(item) => {
            (format!("enum {}", item.ident), line_of(item.ident.span()))
        },
        Item::Use(item) => ("use".to_string(), line_of(item.use_token.span)),
        Item::Impl(item) => ("impl".to_string(), line_of(item.impl_token.span)),
        _ => ("item".to_string(), 0),
    }
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

    const CHILD_EXAMPLE: &str = "pub(crate) mod event;\npub(crate) mod types;\npub(crate) mod view;\n\
        mod checks;\nmod model;\nmod reducer;\nmod state;\n\n\
        pub(in super::super) use model::{RunPanelViewModel, RunFormViewModel};\n\
        pub(in super::super) use reducer::{RunCtx, RunEngine, RunOutcome, reduce};\n\
        pub(in super::super) use state::RunState;\n";

    /// The `containers` widget whose `mod.rs` is `root`, with the child
    /// widget `run` whose `mod.rs` is `run`, and the files in `extra`.
    fn with_run(root: &str, run: &str, extra: &[(&str, &str)]) -> SourceTree {
        let mut sources = vec![
            ("widgets/containers/mod.rs", root),
            ("widgets/containers/child/mod.rs", "pub(crate) mod run;\n"),
            ("widgets/containers/child/run/mod.rs", run),
        ];
        sources.extend_from_slice(extra);

        SourceTree::from_sources(&sources)
    }

    /// A parent `mod.rs` that follows the rule.
    const PARENT: &str = "pub(crate) mod child;\npub(crate) mod event;\nmod state;\npub(crate) mod view;\n";

    #[test]
    fn the_example_child_widget_follows_the_rule() {
        let tree = with_run(PARENT, CHILD_EXAMPLE, &[]);
        let findings = check(&tree);

        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_widget_opens_its_child_folder_but_no_other_folder() {
        let tree = with_run(
            "mod child;\npub(crate) mod event;\npub(crate) mod helpers;\npub(crate) mod run;\npub(crate) mod view;\n",
            CHILD_EXAMPLE,
            &[
                ("widgets/containers/helpers.rs", ""),
                ("widgets/containers/run/mod.rs", ""),
            ],
        );

        assert_eq!(
            found(&check(&tree), Rule::WidgetModuleVisibility),
            [
                pair("widgets/containers/mod.rs", "mod child"),
                pair("widgets/containers/mod.rs", "pub(crate) mod helpers"),
                pair("widgets/containers/mod.rs", "pub(crate) mod run"),
            ]
        );
    }

    #[test]
    fn a_model_folder_is_no_child_widget_and_stays_closed() {
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
    fn a_child_mod_file_follows_the_widget_lists() {
        let tree = with_run(
            PARENT,
            "mod event;\npub(crate) mod inner;\npub(crate) mod state;\npub(crate) mod view;\n",
            &[],
        );

        assert_eq!(
            found(&check(&tree), Rule::WidgetModuleVisibility),
            [
                pair("widgets/containers/child/run/mod.rs", "mod event"),
                pair(
                    "widgets/containers/child/run/mod.rs",
                    "pub(crate) mod inner"
                ),
                pair(
                    "widgets/containers/child/run/mod.rs",
                    "pub(crate) mod state"
                ),
            ]
        );
    }

    #[test]
    fn a_child_never_opens_services_storage_or_child() {
        let tree = with_run(
            "pub(crate) mod child;\npub(crate) mod event;\npub(crate) mod services;\npub(crate) mod view;\n",
            "pub(crate) mod child;\npub(crate) mod event;\npub(crate) mod services;\npub(crate) mod storage;\npub(crate) mod view;\n",
            &[],
        );

        assert_eq!(
            found(&check(&tree), Rule::WidgetModuleVisibility),
            [
                pair(
                    "widgets/containers/child/run/mod.rs",
                    "pub(crate) mod child"
                ),
                pair(
                    "widgets/containers/child/run/mod.rs",
                    "pub(crate) mod services"
                ),
                pair(
                    "widgets/containers/child/run/mod.rs",
                    "pub(crate) mod storage"
                ),
            ]
        );
    }

    #[test]
    fn re_exports_of_a_child_come_from_its_closed_modules() {
        let tree = with_run(
            PARENT,
            "pub(crate) mod event;\nmod model;\npub(crate) mod view;\n\
             pub(in super::super) use model::RunFormViewModel;\n\
             pub(in super::super) use event::RunIntent;\n\
             pub(in super::super) use super::super::types::Place;\n",
            &[],
        );

        assert_eq!(
            found(&check(&tree), Rule::ReexportFromOpenModule),
            [
                pair("widgets/containers/child/run/mod.rs", "event"),
                pair(
                    "widgets/containers/child/run/mod.rs",
                    "super::super::types::Place"
                ),
            ]
        );
    }

    #[test]
    fn the_child_mod_file_declares_only_its_child_widgets() {
        let tree = with_run(PARENT, CHILD_EXAMPLE, &[]);

        let findings = check(&tree);

        assert!(
            found(&findings, Rule::ChildModDeclaresOther).is_empty(),
            "{findings:?}"
        );
    }

    #[test]
    fn anything_else_in_the_child_mod_file_is_reported() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", PARENT),
            (
                "widgets/containers/child/mod.rs",
                "pub(crate) mod create;\nmod run;\npub mod inspector;\n\
                 pub(crate) mod helpers;\n\
                 pub(crate) use create::CreateState;\n\
                 fn shared() {}\n\
                 pub(crate) struct Shared;\n\
                 #[cfg(test)]\nmod tests {}\n",
            ),
            ("widgets/containers/child/create/mod.rs", ""),
            ("widgets/containers/child/run/mod.rs", ""),
            ("widgets/containers/child/inspector/mod.rs", ""),
            ("widgets/containers/child/helpers.rs", ""),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ChildModDeclaresOther),
            [
                pair("widgets/containers/child/mod.rs", "fn shared"),
                pair("widgets/containers/child/mod.rs", "mod run"),
                pair("widgets/containers/child/mod.rs", "pub mod inspector"),
                pair(
                    "widgets/containers/child/mod.rs",
                    "pub(crate) mod helpers"
                ),
                pair("widgets/containers/child/mod.rs", "struct Shared"),
                pair("widgets/containers/child/mod.rs", "use"),
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
