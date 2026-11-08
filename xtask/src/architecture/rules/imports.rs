//! Rules about which layer may import which: widgets stay apart from each
//! other and from the wiring, shared modules stay shared, views read view
//! models, and only a route uses its widget's services and storage.

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    Layer, SHARED_MODULES, WIRING_MODULES, is_view_file, layer_of,
};
use super::super::paths::{CratePath, crate_paths};
use super::super::source::{SourceFile, SourceTree};

/// Modules of a widget that only its route may use.
const ROUTE_ONLY_MODULES: &[&str] = &["services", "storage"];

/// Report imports that cross a layer boundary they may not cross.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        let Some(layer) = layer_of(file.path()) else {
            continue;
        };

        for path in crate_paths(tree, file) {
            match &layer {
                Layer::Widget(widget) => {
                    findings.extend(widget_import(file, widget, &path));
                },
                Layer::Shared => findings.extend(shared_import(file, &path)),
                Layer::Wiring => findings.extend(wiring_import(file, &path)),
            }
        }
    }

    findings
}

/// What breaks a rule when a file of `widget` names `path`.
fn widget_import(
    file: &SourceFile,
    widget: &str,
    path: &CratePath,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let segments = path.segments();

    let other_widget = segments.first().is_some_and(|first| first == "widgets")
        && segments.get(1).is_some_and(|name| name != widget);
    if other_widget {
        findings.push(found(
            Rule::WidgetImportsWidget,
            file,
            path.module_head(3),
            path,
        ));
        return findings;
    }

    let names_wiring = segments == ["widgets"]
        || segments
            .first()
            .is_some_and(|first| WIRING_MODULES.contains(&first.as_str()));
    if names_wiring {
        findings.push(found(
            Rule::WidgetImportsWiring,
            file,
            path.module_head(1),
            path,
        ));
    }

    if is_view_file(file.path(), widget)
        && path.starts_with(&["widgets", widget, "state"])
    {
        findings.push(found(Rule::ViewImportsState, file, path.head(3), path));
    }

    if let Some(module) = route_only_module(path, widget)
        && !may_use_route_only_module(file, widget, module)
    {
        findings.push(found(
            Rule::ServicesOutsideRoute,
            file,
            path.head(3),
            path,
        ));
    }

    findings
}

/// What breaks a rule when a shared module names `path`.
fn shared_import(file: &SourceFile, path: &CratePath) -> Option<Finding> {
    let first = path.segments().first()?;
    if SHARED_MODULES.contains(&first.as_str()) {
        return None;
    }

    Some(found(
        Rule::SharedImportsOutside,
        file,
        path.module_head(2),
        path,
    ))
}

/// What breaks a rule when a wiring file names `path`: only a widget's own
/// route may use its services and storage.
fn wiring_import(file: &SourceFile, path: &CratePath) -> Option<Finding> {
    let segments = path.segments();
    if segments.first().is_none_or(|first| first != "widgets") {
        return None;
    }
    let widget = segments.get(1)?;
    route_only_module(path, widget)?;

    let own_route = format!("router/{widget}.rs");
    if file.path() == own_route {
        return None;
    }

    Some(found(Rule::ServicesOutsideRoute, file, path.head(3), path))
}

/// `services` or `storage` when `path` points into one of them of `widget`.
fn route_only_module<'a>(path: &'a CratePath, widget: &str) -> Option<&'a str> {
    let segments = path.segments();
    if segments.len() < 3 || segments[0] != "widgets" || segments[1] != widget {
        return None;
    }

    let module = segments[2].as_str();
    ROUTE_ONLY_MODULES.contains(&module).then_some(module)
}

/// Inside the widget, only the module itself uses its own services or
/// storage, and `mod.rs` may read storage for `XWidget::load`.
fn may_use_route_only_module(
    file: &SourceFile,
    widget: &str,
    module: &str,
) -> bool {
    let own_file = format!("widgets/{widget}/{module}.rs");
    let own_dir = format!("widgets/{widget}/{module}/");
    let widget_root = format!("widgets/{widget}/mod.rs");

    file.path() == own_file
        || file.path().starts_with(&own_dir)
        || (module == "storage" && file.path() == widget_root)
}

fn found(
    rule: Rule,
    file: &SourceFile,
    key: String,
    path: &CratePath,
) -> Finding {
    Finding::new(rule, file.path(), &key, path.line())
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    #[test]
    fn a_widget_importing_another_widget_is_reported() {
        let tree = SourceTree::from_sources(&[
            ("widgets/palette/mod.rs", ""),
            ("widgets/palette/state.rs", ""),
            (
                "widgets/palette/items.rs",
                "use crate::widgets::modes::types::Mode;\nuse super::state::PaletteState;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetImportsWidget),
            [pair("widgets/palette/items.rs", "widgets::modes::types")]
        );
    }

    #[test]
    fn a_widget_importing_the_wiring_is_reported() {
        let tree = SourceTree::from_sources(&[(
            "widgets/notes/reducer.rs",
            "use crate::router::AppEvent;\nuse crate::theme::ThemeProps;\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetImportsWiring),
            [pair("widgets/notes/reducer.rs", "router")]
        );
    }

    #[test]
    fn a_shared_module_importing_a_widget_is_reported() {
        let tree = SourceTree::from_sources(&[(
            "domain/quick_launch.rs",
            "use crate::widgets::quick_launch::types::QuickLaunch;\nuse crate::i18n::Key;\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::SharedImportsOutside),
            [pair("domain/quick_launch.rs", "widgets::quick_launch")]
        );
    }

    #[test]
    fn a_view_importing_the_state_is_reported() {
        let tree = SourceTree::from_sources(&[
            ("widgets/notes/mod.rs", ""),
            (
                "widgets/notes/view/panel.rs",
                "use super::super::state::NotesState;\nuse super::super::model::NotesViewModel;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ViewImportsState),
            [pair("widgets/notes/view/panel.rs", "widgets::notes::state")]
        );
    }

    #[test]
    fn services_and_storage_belong_to_the_route() {
        let tree = SourceTree::from_sources(&[
            ("widgets/notes/mod.rs", "use storage::read_note;\n"),
            ("widgets/notes/storage.rs", ""),
            ("widgets/notes/services.rs", ""),
            (
                "widgets/notes/reducer.rs",
                "use super::services::build_command;\n",
            ),
            (
                "router/notes.rs",
                "use crate::widgets::notes::storage::read_note;\n",
            ),
            ("app.rs", "use crate::widgets::notes::services::open;\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ServicesOutsideRoute),
            [
                pair("app.rs", "widgets::notes::services"),
                pair("widgets/notes/reducer.rs", "widgets::notes::services"),
            ]
        );
    }
}
