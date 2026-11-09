//! Rules about which layer may import which: widgets stay apart from each
//! other and from the wiring, shared modules stay shared, views read view
//! models, a child widget stays out of its parent and its siblings, mode
//! screens stay out of the root view, only a route uses its widget's
//! services and storage, and only the places that run effects use the
//! shared modules that talk to the outside world.

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    CHILD_DIR, Layer, SHARED_IO_MODULES, SHARED_MODULES, WIRING_MODULES,
    child_of, is_view_file, layer_of, path_in_owner,
};
use super::super::paths::{CratePath, crate_paths};
use super::super::source::{SourceFile, SourceTree};

/// Modules of a widget that only its route may use.
const ROUTE_ONLY_MODULES: &[&str] = &["services", "storage"];

/// Files of a widget or a child widget that may use the shared modules that
/// talk to the outside world, besides the folders of the same names.
const SHARED_IO_USERS: &[&str] =
    &["services.rs", "storage.rs", "subscription.rs"];

/// The root view, which `app.rs` declares with `#[path]`.
const ROOT_VIEW: &[&str] = &["app", "view"];

/// Modules of a parent widget that its child widgets never import.
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

        if !may_use_shared_io(file, &layer) {
            findings.extend(
                paths.iter().filter_map(|path| shared_io_import(file, path)),
            );
        }
        match &layer {
            Layer::Widget(widget) => {
                findings.extend(widget_imports(file, widget, &paths));
            },
            Layer::Shared => findings.extend(
                paths.iter().filter_map(|path| shared_import(file, path)),
            ),
            Layer::Wiring => {
                findings.extend(
                    paths.iter().filter_map(|path| wiring_import(file, path)),
                );
                findings.extend(
                    paths.iter().filter_map(|path| screen_import(file, path)),
                );
            },
        }
    }

    findings
}

/// What breaks a rule in the paths that a file of `widget` names: the
/// checks of every widget file, the state check of a view and the outside
/// check of a child widget.
fn widget_imports(
    file: &SourceFile,
    widget: &str,
    paths: &[CratePath],
) -> Vec<Finding> {
    let is_view = is_view_file(file.path(), widget);
    let own_child = child_of(file.path(), widget);

    let mut findings = Vec::new();
    for path in paths {
        findings.extend(widget_import(file, widget, path));

        if is_view && let Some(key) = state_import(path, widget) {
            findings.push(found(Rule::ViewImportsState, file, key, path));
        }
        if let Some(child) = own_child
            && let Some(key) = outside_child_import(path, widget, child)
        {
            findings.push(found(Rule::ChildImportsOutside, file, key, path));
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
/// of one of its child widgets: `widgets::<widget>::state` or
/// `widgets::<widget>::child::<child>::state`.
fn state_import(path: &CratePath, widget: &str) -> Option<String> {
    if path.starts_with(&["widgets", widget, "state"]) {
        return Some(path.head(3));
    }

    let names_child_state = path.starts_with(&["widgets", widget, CHILD_DIR])
        && path.segments().get(4).is_some_and(|name| name == "state");

    names_child_state.then(|| path.head(5))
}

/// The key of a path that a file of the child widget `child` may not name:
/// one of the [`PARENT_ONLY_MODULES`] of `widget`, another of its child
/// widgets, or an item of its `mod.rs`, which has a capitalized name.
fn outside_child_import(
    path: &CratePath,
    widget: &str,
    child: &str,
) -> Option<String> {
    if !path.starts_with(&["widgets", widget]) {
        return None;
    }
    let name = path.segments().get(2)?;

    if name == CHILD_DIR {
        let other = path.segments().get(3)?;
        return (other != child).then(|| path.module_head(4));
    }

    let is_parent_module = PARENT_ONLY_MODULES.contains(&name.as_str());
    let is_parent_item = name.starts_with(|ch: char| ch.is_ascii_uppercase());

    (is_parent_module || is_parent_item).then(|| path.module_head(3))
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

/// Whether `file` may use the shared modules that talk to the outside
/// world: a route, a widget's or a child widget's `services`, `storage` or
/// `subscription`, or one of those shared modules itself.
fn may_use_shared_io(file: &SourceFile, layer: &Layer) -> bool {
    let path = file.path();

    match layer {
        Layer::Wiring => path.starts_with("router/"),
        Layer::Widget(widget) => {
            let inner = path_in_owner(path, widget);
            SHARED_IO_USERS.iter().any(|name| {
                let dir = format!("{}/", name.trim_end_matches(".rs"));
                inner == *name || inner.starts_with(&dir)
            })
        },
        Layer::Shared => {
            let first = path.split('/').next().unwrap_or(path);
            let module = first.strip_suffix(".rs").unwrap_or(first);
            SHARED_IO_MODULES.contains(&module)
        },
    }
}

/// What breaks a rule when a file that runs no effects names `path`: a
/// shared module that talks to the outside world.
fn shared_io_import(file: &SourceFile, path: &CratePath) -> Option<Finding> {
    let first = path.segments().first()?;
    if !SHARED_IO_MODULES.contains(&first.as_str()) {
        return None;
    }

    Some(found(Rule::SharedIoOutsideRoute, file, path.head(1), path))
}

/// What breaks a rule when a mode screen names `path`: the root view.
fn screen_import(file: &SourceFile, path: &CratePath) -> Option<Finding> {
    if !file.path().starts_with("screens/") || !path.starts_with(ROOT_VIEW) {
        return None;
    }

    Some(found(
        Rule::ScreenImportsRootView,
        file,
        ROOT_VIEW.join("::"),
        path,
    ))
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

    /// The modules of a parent widget that its child widgets never import,
    /// written out here so that the tests do not read the rule's own list.
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

    /// The `containers` widget with the child widgets `run` and `create`,
    /// and the files in `extra`.
    fn with_children(extra: &[(&str, &str)]) -> SourceTree {
        let mut sources = vec![
            ("widgets/containers/mod.rs", "pub(crate) mod child;\n"),
            (
                "widgets/containers/child/mod.rs",
                "pub(crate) mod create;\npub(crate) mod run;\n",
            ),
            ("widgets/containers/child/run/mod.rs", ""),
            ("widgets/containers/child/create/mod.rs", ""),
        ];
        sources.extend_from_slice(extra);

        SourceTree::from_sources(&sources)
    }

    #[test]
    fn a_view_importing_the_state_of_a_child_widget_is_reported() {
        let tree = with_children(&[
            (
                "widgets/containers/child/run/view/run_form.rs",
                "use super::super::state::RunState;\nuse super::super::model::RunFormViewModel;\n",
            ),
            (
                "widgets/containers/view/main_area.rs",
                "use super::super::child::run::state::RunState;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ViewImportsState),
            [
                pair(
                    "widgets/containers/child/run/view/run_form.rs",
                    "widgets::containers::child::run::state"
                ),
                pair(
                    "widgets/containers/view/main_area.rs",
                    "widgets::containers::child::run::state"
                ),
            ]
        );
    }

    #[test]
    fn a_child_imports_its_own_files_the_parent_types_and_shared_modules() {
        let tree = with_children(&[
            (
                "widgets/containers/child/run/reducer.rs",
                "use super::state::RunState;\n\
                 use super::super::super::types::Place;\n\
                 use super::super::super::errors::ContainersError;\n\
                 use super::super::super::format::engine_label;\n\
                 use crate::widgets::containers::child::run::event::RunIntent;\n\
                 use crate::components::pull_line::PullLineViewModel;\n\
                 use crate::i18n::Key;\n\
                 use crate::domain::mode::Mode;\n",
            ),
            (
                "widgets/containers/child/run/view/run_form.rs",
                "use super::super::event::RunIntent;\n",
            ),
        ]);

        let findings = check(&tree);

        assert!(
            found(&findings, Rule::ChildImportsOutside).is_empty(),
            "{findings:?}"
        );
    }

    #[test]
    fn a_child_importing_a_module_of_its_parent_is_reported() {
        for module in PARENT_MODULES {
            let relative =
                format!("use super::super::super::{module}::Item;\n");
            let absolute =
                format!("use crate::widgets::containers::{module}::Item;\n");
            let tree = with_children(&[
                ("widgets/containers/child/run/reducer.rs", relative.as_str()),
                (
                    "widgets/containers/child/run/view/run_form.rs",
                    absolute.as_str(),
                ),
                ("widgets/containers/sort.rs", absolute.as_str()),
            ]);

            let key = format!("widgets::containers::{module}");

            assert_eq!(
                found(&check(&tree), Rule::ChildImportsOutside),
                [
                    pair("widgets/containers/child/run/reducer.rs", &key),
                    pair("widgets/containers/child/run/view/run_form.rs", &key),
                ],
                "{module}"
            );
        }
    }

    #[test]
    fn the_folders_of_a_widget_outside_child_are_no_child_widgets() {
        let tree = with_children(&[
            ("widgets/containers/model/mod.rs", ""),
            (
                "widgets/containers/model/main_area.rs",
                "use super::super::state::ContainersState;\n\
                 use super::super::child::run::RunPanelViewModel;\n",
            ),
            ("widgets/containers/legacy/mod.rs", ""),
            (
                "widgets/containers/legacy/reducer.rs",
                "use super::super::state::ContainersState;\n",
            ),
            (
                "widgets/containers/child/run/model/run_form.rs",
                "use super::super::super::super::model::MainAreaViewModel;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ChildImportsOutside),
            [pair(
                "widgets/containers/child/run/model/run_form.rs",
                "widgets::containers::model"
            )]
        );
    }

    #[test]
    fn a_child_importing_another_child_is_reported() {
        let tree = with_children(&[
            (
                "widgets/containers/child/run/reducer.rs",
                "use super::super::create::types::FormField;\n",
            ),
            (
                "widgets/containers/child/create/model.rs",
                "use crate::widgets::containers::child::run::event::RunIntent;\n",
            ),
            (
                "widgets/containers/reducer.rs",
                "use super::child::run::event::RunIntent;\n\
                 use super::child::create::event::CreateIntent;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ChildImportsOutside),
            [
                pair(
                    "widgets/containers/child/create/model.rs",
                    "widgets::containers::child::run"
                ),
                pair(
                    "widgets/containers/child/run/reducer.rs",
                    "widgets::containers::child::create"
                ),
            ]
        );
    }

    #[test]
    fn a_child_importing_an_item_of_the_parent_mod_file_is_reported() {
        let tree = with_children(&[
            (
                "widgets/containers/child/run/reducer.rs",
                "use super::super::super::ContainersCtx;\n",
            ),
            (
                "widgets/containers/child/run/model.rs",
                "fn widget(w: &W) {\n    let widget: &crate::widgets::containers::ContainersWidget = w;\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ChildImportsOutside),
            [
                pair(
                    "widgets/containers/child/run/model.rs",
                    "widgets::containers"
                ),
                pair(
                    "widgets/containers/child/run/reducer.rs",
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

    #[test]
    fn shared_io_is_used_only_where_effects_run() {
        let tree = with_children(&[
            ("main.rs", "mod services;\nmod shell_integration;\n"),
            (
                "services.rs",
                "use crate::shell_integration::with_shell_integration;\n",
            ),
            ("shell_integration.rs", "use crate::services::open_url;\n"),
            (
                "router/containers.rs",
                "use crate::services::{self, open_url};\n",
            ),
            (
                "widgets/quick_launch/services.rs",
                "use crate::shell_integration::dir;\n",
            ),
            (
                "widgets/quick_launch/storage.rs",
                "use crate::services::home;\n",
            ),
            (
                "widgets/containers/subscription.rs",
                "use crate::services::home;\n",
            ),
            (
                "widgets/containers/child/run/subscription.rs",
                "use crate::services::home;\n",
            ),
            (
                "widgets/tabs/reducer.rs",
                "use crate::services::open_url;\nfn reduce() {\n    open_url(url);\n}\n",
            ),
            (
                "widgets/tabs/mod.rs",
                "fn load() {\n    crate::services::home();\n}\n",
            ),
            ("app.rs", "use crate::{layout, services};\n"),
            (
                "shell_session.rs",
                "use crate::shell_integration::{dir, with_shell_integration};\n",
            ),
            ("domain/mode.rs", "use crate::services::home;\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::SharedIoOutsideRoute),
            [
                pair("app.rs", "services"),
                pair("domain/mode.rs", "services"),
                pair("shell_session.rs", "shell_integration"),
                pair("widgets/tabs/mod.rs", "services"),
                pair("widgets/tabs/reducer.rs", "services"),
            ]
        );
    }

    #[test]
    fn a_mode_screen_never_imports_the_root_view() {
        let tree = SourceTree::from_sources(&[
            ("app.rs", "#[path = \"view.rs\"]\npub(crate) mod view;\n"),
            ("view.rs", "use crate::screens::terminal;\n"),
            (
                "screens/terminal.rs",
                "use crate::app::view::view as root_view;\nuse crate::app::App;\n",
            ),
            (
                "screens/containers.rs",
                "fn f() {\n    let _ = crate::app::view::layers();\n}\n",
            ),
            ("screens/mod.rs", "use crate::app::App;\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ScreenImportsRootView),
            [
                pair("screens/containers.rs", "app::view"),
                pair("screens/terminal.rs", "app::view"),
            ]
        );
    }
}
