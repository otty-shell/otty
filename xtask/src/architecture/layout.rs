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
    "components",
    "domain",
];

/// Wiring files outside `router/`: the crate root, the app loop and the
/// list of widgets.
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

/// The files every widget has.
pub(crate) const REQUIRED_WIDGET_FILES: &[&str] =
    &["mod.rs", "event.rs", "state.rs", "reducer.rs", "model.rs"];

/// Files with a fixed role in a widget; any other root file is a topic file.
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

/// Widget modules that may be `pub(crate) mod`.
pub(crate) const OPEN_WIDGET_MODULES: &[&str] =
    &["event", "types", "view", "storage", "services"];

/// Widget modules that must be `pub(crate) mod`: other routes send intents
/// with `event`, and `view.rs` calls `view`.
pub(crate) const ALWAYS_OPEN_WIDGET_MODULES: &[&str] = &["event", "view"];

/// The layer a source file belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Layer {
    /// The app loop, the input filter, the routes and the root view.
    Wiring,
    /// A file inside `widgets/<name>/`.
    Widget(String),
    /// A module that any layer may use.
    Shared,
}

/// The layer of the file at `path`, or `None` when it belongs to no layer.
pub(crate) fn layer_of(path: &str) -> Option<Layer> {
    if WIRING_FILES.contains(&path) || path.starts_with("router/") {
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

/// Whether the file at `path` of `widget` is part of its view.
pub(crate) fn is_view_file(path: &str, widget: &str) -> bool {
    let inner = path_in_widget(path, widget);

    inner == "view.rs" || inner.starts_with("view/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_get_the_layer_of_their_folder() {
        assert_eq!(layer_of("router/tabs.rs"), Some(Layer::Wiring));
        assert_eq!(layer_of("update.rs"), Some(Layer::Wiring));
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
    fn view_files_are_view_rs_or_inside_view() {
        assert!(is_view_file("widgets/palette/view.rs", "palette"));
        assert!(is_view_file("widgets/modes/view/menu.rs", "modes"));
        assert!(!is_view_file("widgets/modes/model.rs", "modes"));
    }
}
