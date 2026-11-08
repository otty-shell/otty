//! Rules about which layer may import which: widgets stay apart from each
//! other and from the wiring, shared modules stay shared, views read view
//! models, a part stays out of its parent and its sibling parts, and only a
//! route uses its widget's services and storage.

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    Layer, SHARED_MODULES, WIRING_MODULES, is_view_file, layer_of, part_names,
    part_of,
};
use super::super::paths::{CratePath, crate_paths};
use super::super::source::{SourceFile, SourceTree};

/// Modules of a widget that only its route may use.
const ROUTE_ONLY_MODULES: &[&str] = &["services", "storage"];

/// Modules of a parent widget that its parts never import.
const PARENT_ONLY_MODULES: &[&str] = &[
    "state",
    "reducer",
    "event",
    "model",
    "subscription",
    "view",
    "services",
    "storage",
];

/// Report imports that cross a layer boundary they may not cross.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        let Some(layer) = layer_of(file.path()) else {
            continue;
        };
        let paths = crate_paths(tree, file);

        match &layer {
            Layer::Widget(widget) => {
                findings.extend(widget_imports(tree, file, widget, &paths));
            },
            Layer::Shared => findings.extend(
                paths.iter().filter_map(|path| shared_import(file, path)),
            ),
            Layer::Wiring => findings.extend(
                paths.iter().filter_map(|path| wiring_import(file, path)),
            ),
        }
    }

    findings
}

/// What breaks a rule in the paths that a file of `widget` names: the
/// checks of every widget file, the state check of a view and the outside
/// check of a part.
fn widget_imports(
    tree: &SourceTree,
    file: &SourceFile,
    widget: &str,
    paths: &[CratePath],
) -> Vec<Finding> {
    let parts = part_names(tree, widget);
    let is_view = is_view_file(tree, file.path(), widget);
    let own_part = part_of(tree, file.path(), widget);

    let mut findings = Vec::new();
    for path in paths {
        findings.extend(widget_import(file, widget, path));

        if is_view && let Some(key) = state_import(path, widget, &parts) {
            findings.push(found(Rule::ViewImportsState, file, key, path));
        }
        if let Some(part) = own_part
            && let Some(key) = outside_part_import(path, widget, part, &parts)
        {
            findings.push(found(Rule::PartImportsOutside, file, key, path));
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

/// The state module that `path` names when it is the state of `widget` or
/// of one of its `parts`: `widgets::<widget>::state` or
/// `widgets::<widget>::<part>::state`.
fn state_import(
    path: &CratePath,
    widget: &str,
    parts: &[String],
) -> Option<String> {
    if path.starts_with(&["widgets", widget, "state"]) {
        return Some(path.head(3));
    }

    let part = path.segments().get(2)?;
    let names_part_state = parts.contains(part)
        && path.starts_with(&["widgets", widget, part.as_str(), "state"]);

    names_part_state.then(|| path.head(4))
}

/// The key of a path that a file of `part` may not name: one of the
/// [`PARENT_ONLY_MODULES`] of `widget`, another of its `parts`, or an item
/// of its `mod.rs`, which has a capitalized name.
fn outside_part_import(
    path: &CratePath,
    widget: &str,
    part: &str,
    parts: &[String],
) -> Option<String> {
    if !path.starts_with(&["widgets", widget]) {
        return None;
    }
    let name = path.segments().get(2)?;
    if name == part {
        return None;
    }

    let is_parent_module = PARENT_ONLY_MODULES.contains(&name.as_str());
    let is_other_part = parts.contains(name);
    let is_parent_item = name.starts_with(|ch: char| ch.is_ascii_uppercase());

    (is_parent_module || is_other_part || is_parent_item)
        .then(|| path.module_head(3))
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
    fn a_widget_importing_a_mode_screen_is_reported() {
        let tree = SourceTree::from_sources(&[(
            "widgets/notes/view/panel.rs",
            "use crate::screens::terminal;\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetImportsWiring),
            [pair("widgets/notes/view/panel.rs", "screens")]
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

    /// The modules of a parent widget that its parts never import, written
    /// out here so that the tests do not read the rule's own list.
    const PARENT_MODULES: &[&str] = &[
        "state",
        "reducer",
        "event",
        "model",
        "subscription",
        "view",
        "services",
        "storage",
    ];

    /// The `containers` widget with the parts `run` and `create`, and the
    /// files in `extra`.
    fn with_parts(extra: &[(&str, &str)]) -> SourceTree {
        let mut sources = vec![
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            ("widgets/containers/create/mod.rs", ""),
        ];
        sources.extend_from_slice(extra);

        SourceTree::from_sources(&sources)
    }

    #[test]
    fn a_view_importing_the_state_of_a_part_is_reported() {
        let tree = with_parts(&[
            (
                "widgets/containers/run/view/run_form.rs",
                "use super::super::state::RunState;\nuse super::super::model::RunFormViewModel;\n",
            ),
            (
                "widgets/containers/view/main_area.rs",
                "use super::super::run::state::RunState;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ViewImportsState),
            [
                pair(
                    "widgets/containers/run/view/run_form.rs",
                    "widgets::containers::run::state"
                ),
                pair(
                    "widgets/containers/view/main_area.rs",
                    "widgets::containers::run::state"
                ),
            ]
        );
    }

    #[test]
    fn a_part_imports_its_own_files_the_parent_types_and_shared_modules() {
        let tree = with_parts(&[
            (
                "widgets/containers/run/reducer.rs",
                "use super::state::RunState;\n\
                 use super::super::types::Place;\n\
                 use super::super::errors::ContainersError;\n\
                 use super::super::format::engine_label;\n\
                 use crate::widgets::containers::run::event::RunIntent;\n\
                 use crate::components::pull_line::PullLineViewModel;\n\
                 use crate::i18n::Key;\n\
                 use crate::domain::mode::Mode;\n",
            ),
            (
                "widgets/containers/run/view/run_form.rs",
                "use super::super::event::RunIntent;\n",
            ),
        ]);

        let findings = check(&tree);

        assert!(
            found(&findings, Rule::PartImportsOutside).is_empty(),
            "{findings:?}"
        );
    }

    #[test]
    fn a_part_importing_a_module_of_its_parent_is_reported() {
        for module in PARENT_MODULES {
            let relative = format!("use super::super::{module}::Item;\n");
            let absolute =
                format!("use crate::widgets::containers::{module}::Item;\n");
            let tree = with_parts(&[
                ("widgets/containers/run/reducer.rs", relative.as_str()),
                ("widgets/containers/run/view/run_form.rs", absolute.as_str()),
                ("widgets/containers/sort.rs", absolute.as_str()),
            ]);

            let key = format!("widgets::containers::{module}");

            assert_eq!(
                found(&check(&tree), Rule::PartImportsOutside),
                [
                    pair("widgets/containers/run/reducer.rs", &key),
                    pair("widgets/containers/run/view/run_form.rs", &key),
                ],
                "{module}"
            );
        }
    }

    #[test]
    fn the_model_folder_of_a_widget_is_no_part() {
        let tree = with_parts(&[
            ("widgets/containers/model/mod.rs", ""),
            (
                "widgets/containers/model/main_area.rs",
                "use super::super::state::ContainersState;\n\
                 use super::super::run::RunPanelViewModel;\n",
            ),
            (
                "widgets/containers/run/model/run_form.rs",
                "use super::super::super::model::MainAreaViewModel;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::PartImportsOutside),
            [pair(
                "widgets/containers/run/model/run_form.rs",
                "widgets::containers::model"
            )]
        );
    }

    #[test]
    fn a_part_importing_another_part_is_reported() {
        let tree = with_parts(&[
            (
                "widgets/containers/run/reducer.rs",
                "use super::super::create::types::FormField;\n",
            ),
            (
                "widgets/containers/create/model.rs",
                "use crate::widgets::containers::run::event::RunIntent;\n",
            ),
            (
                "widgets/containers/reducer.rs",
                "use super::run::event::RunIntent;\nuse super::create::event::CreateIntent;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::PartImportsOutside),
            [
                pair(
                    "widgets/containers/create/model.rs",
                    "widgets::containers::run"
                ),
                pair(
                    "widgets/containers/run/reducer.rs",
                    "widgets::containers::create"
                ),
            ]
        );
    }

    #[test]
    fn a_part_importing_an_item_of_the_parent_mod_file_is_reported() {
        let tree = with_parts(&[
            (
                "widgets/containers/run/reducer.rs",
                "use super::super::ContainersCtx;\n",
            ),
            (
                "widgets/containers/run/model.rs",
                "let widget: &crate::widgets::containers::ContainersWidget = w;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::PartImportsOutside),
            [
                pair("widgets/containers/run/model.rs", "widgets::containers"),
                pair(
                    "widgets/containers/run/reducer.rs",
                    "widgets::containers"
                ),
            ]
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
