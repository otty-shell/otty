//! The layers of `app/src` and the names the rules rely on, as written in
//! `docs/internal/architecture.md`.

use super::source::SourceTree;

/// Top-level modules that form the wiring. `update`, `subscription` and
/// `view` are declared inside `app` with `#[path]`.
pub(crate) const WIRING_MODULES: &[&str] =
    &["app", "state", "guards", "router"];

/// Shared modules: they import only each other and crates.
pub(crate) const SHARED_MODULES: &[&str] = &[
    "i18n",
    "theme",
    "fonts",
    "icons",
    "geometry",
    "layout",
    "shortcuts",
    "commands",
    "services",
    "shell_integration",
    "shell_session",
    "components",
    "domain",
];

/// Wiring files outside `router/` and `view/`: the crate root, the app loop
/// and the list of widgets.
const WIRING_FILES: &[&str] = &[
    "main.rs",
    "app.rs",
    "state.rs",
    "update.rs",
    "subscription.rs",
    "guards.rs",
    "view.rs",
    "widgets/mod.rs",
];

/// The files every widget and every part of a widget has.
pub(crate) const REQUIRED_WIDGET_FILES: &[&str] =
    &["mod.rs", "event.rs", "state.rs", "reducer.rs", "model.rs"];

/// Files with a fixed role in a widget or a part; any other file at its
/// root is a topic file.
pub(crate) const STANDARD_WIDGET_FILES: &[&str] = &[
    "mod.rs",
    "event.rs",
    "state.rs",
    "reducer.rs",
    "model.rs",
    "types.rs",
    "errors.rs",
    "services.rs",
    "storage.rs",
    "subscription.rs",
];

/// Modules that may be `pub(crate) mod` in a widget's or a part's `mod.rs`;
/// a widget may also open its parts.
pub(crate) const OPEN_WIDGET_MODULES: &[&str] =
    &["event", "types", "view", "storage", "services"];

/// Modules that must be `pub(crate) mod` in a widget's or a part's
/// `mod.rs`: other routes send intents with `event`, and `view.rs` calls
/// `view`.
pub(crate) const ALWAYS_OPEN_WIDGET_MODULES: &[&str] = &["event", "view"];

/// The layer a source file belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Layer {
    /// The app loop, the input filter, the routes, the root view and the
    /// screen of each mode.
    Wiring,
    /// A file inside `widgets/<name>/`, also inside one of its parts.
    Widget(String),
    /// A module that any layer may use.
    Shared,
}

/// The layer of the file at `path`, or `None` when it belongs to no layer.
pub(crate) fn layer_of(path: &str) -> Option<Layer> {
    if WIRING_FILES.contains(&path)
        || path.starts_with("router/")
        || path.starts_with("view/")
    {
        return Some(Layer::Wiring);
    }

    if let Some(rest) = path.strip_prefix("widgets/")
        && let Some((widget, _)) = rest.split_once('/')
    {
        return Some(Layer::Widget(widget.to_string()));
    }

    let first = path.split('/').next().unwrap_or(path);
    let module = first.strip_suffix(".rs").unwrap_or(first);
    if SHARED_MODULES.contains(&module) {
        return Some(Layer::Shared);
    }

    None
}

/// The widgets of the tree: the folders directly under `widgets/`.
pub(crate) fn widget_names(tree: &SourceTree) -> Vec<String> {
    let mut names: Vec<String> = tree
        .files()
        .iter()
        .filter_map(|file| match layer_of(file.path()) {
            Some(Layer::Widget(name)) => Some(name),
            _ => None,
        })
        .collect();

    names.sort();
    names.dedup();
    names
}

/// The path of `path` inside its widget folder, e.g. `view/menu.rs` for
/// `widgets/modes/view/menu.rs`.
pub(crate) fn path_in_widget<'a>(path: &'a str, widget: &str) -> &'a str {
    let prefix_len = "widgets/".len() + widget.len() + 1;

    path.get(prefix_len..).unwrap_or("")
}

/// The parts of `widget`: the folders directly under `widgets/<widget>/`
/// that hold a `mod.rs`, other than `view`, e.g. `run` for
/// `widgets/containers/run/mod.rs`.
pub(crate) fn part_names(tree: &SourceTree, widget: &str) -> Vec<String> {
    let prefix = format!("widgets/{widget}/");

    tree.files()
        .iter()
        .filter_map(|file| {
            let folder =
                file.path().strip_prefix(&prefix)?.strip_suffix("/mod.rs")?;
            let is_part = !folder.contains('/') && folder != "view";

            is_part.then(|| folder.to_string())
        })
        .collect()
}

/// The part of `widget` that holds the file at `path`, e.g. `run` for
/// `widgets/containers/run/view/run_form.rs`; `None` for a file of the
/// widget itself.
pub(crate) fn part_of<'a>(
    tree: &SourceTree,
    path: &'a str,
    widget: &str,
) -> Option<&'a str> {
    let (folder, _) = path_in_widget(path, widget).split_once('/')?;

    part_names(tree, widget)
        .iter()
        .any(|part| part == folder)
        .then_some(folder)
}

/// Whether the file at `path` of `widget` belongs to a view: `view.rs` or
/// `view/` of the widget or of one of its parts.
pub(crate) fn is_view_file(
    tree: &SourceTree,
    path: &str,
    widget: &str,
) -> bool {
    let in_widget = path_in_widget(path, widget);
    let inner = match part_of(tree, path, widget) {
        Some(part) => in_widget
            .strip_prefix(part)
            .and_then(|rest| rest.strip_prefix('/'))
            .unwrap_or(in_widget),
        None => in_widget,
    };

    inner == "view.rs" || inner.starts_with("view/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_get_the_layer_of_their_folder() {
        assert_eq!(layer_of("router/tabs.rs"), Some(Layer::Wiring));
        assert_eq!(layer_of("update.rs"), Some(Layer::Wiring));
        assert_eq!(layer_of("view/terminal.rs"), Some(Layer::Wiring));
        assert_eq!(layer_of("widgets/mod.rs"), Some(Layer::Wiring));
        assert_eq!(
            layer_of("widgets/modes/view/menu.rs"),
            Some(Layer::Widget("modes".to_string()))
        );
        assert_eq!(layer_of("domain/mode.rs"), Some(Layer::Shared));
        assert_eq!(layer_of("theme.rs"), Some(Layer::Shared));
        assert_eq!(layer_of("helpers.rs"), None);
    }

    #[test]
    fn widget_names_come_from_the_widget_folders() {
        let tree = SourceTree::from_sources(&[
            ("widgets/mod.rs", ""),
            ("widgets/tabs/mod.rs", ""),
            ("widgets/modes/mod.rs", ""),
            ("widgets/modes/state.rs", ""),
        ]);

        assert_eq!(widget_names(&tree), ["modes", "tabs"]);
    }

    #[test]
    fn parts_are_the_folders_with_a_mod_file_other_than_view() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/view/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            ("widgets/containers/run/view/mod.rs", ""),
            ("widgets/containers/create/mod.rs", ""),
            ("widgets/notes/mod.rs", ""),
        ]);

        assert_eq!(part_names(&tree, "containers"), ["create", "run"]);
        assert!(part_names(&tree, "notes").is_empty());
    }

    #[test]
    fn a_file_inside_a_part_folder_belongs_to_that_part() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
        ]);

        let part = |path| part_of(&tree, path, "containers");

        assert_eq!(
            part("widgets/containers/run/view/run_form.rs"),
            Some("run")
        );
        assert_eq!(part("widgets/containers/run/mod.rs"), Some("run"));
        assert_eq!(part("widgets/containers/view/main_area.rs"), None);
        assert_eq!(part("widgets/containers/state.rs"), None);
    }

    #[test]
    fn view_files_are_view_rs_or_inside_view() {
        let tree = SourceTree::from_sources(&[("widgets/modes/mod.rs", "")]);

        assert!(is_view_file(&tree, "widgets/palette/view.rs", "palette"));
        assert!(is_view_file(&tree, "widgets/modes/view/menu.rs", "modes"));
        assert!(!is_view_file(&tree, "widgets/modes/model.rs", "modes"));
    }

    #[test]
    fn a_part_has_its_own_view_files() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
        ]);

        let is_view = |path| is_view_file(&tree, path, "containers");

        assert!(is_view("widgets/containers/run/view/run_form.rs"));
        assert!(is_view("widgets/containers/run/view.rs"));
        assert!(!is_view("widgets/containers/run/state.rs"));
        assert!(!is_view("widgets/containers/run/model.rs"));
    }
}
