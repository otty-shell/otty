//! The layers of `app/src` and the names the rules rely on, as written in
//! `docs/internal/architecture.md`.

use super::source::SourceTree;

/// Top-level modules that form the wiring. `update`, `subscription` and
/// `view` are declared inside `app` with `#[path]`; `screens` holds the
/// screen of each mode.
pub(crate) const WIRING_MODULES: &[&str] =
    &["app", "state", "guards", "router", "screens"];

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
    "focus",
];

/// Shared modules that talk to the outside world. They are the only shared
/// modules that may lift the clippy ban, and only the places that run
/// effects use them.
pub(crate) const SHARED_IO_MODULES: &[&str] =
    &["services", "shell_integration"];

/// Wiring files outside `router/` and `screens/`: the crate root, the app
/// loop, the root view and the list of widgets.
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

/// The files every widget and every child widget has. A `model/` folder
/// with its `mod.rs` may take the place of `model.rs`.
pub(crate) const REQUIRED_WIDGET_FILES: &[&str] =
    &["mod.rs", "event.rs", "state.rs", "reducer.rs", "model.rs"];

/// The file that holds the view models of a widget or a child widget.
pub(crate) const MODEL_FILE: &str = "model.rs";

/// The folder that may hold the view models instead of `model.rs`: its
/// `mod.rs` and one file per view.
pub(crate) const MODEL_DIR: &str = "model";

/// The folder of a large widget that holds its child widgets: every folder
/// directly inside it is one child widget, e.g. `child/run/`.
pub(crate) const CHILD_DIR: &str = "child";

/// Files with a fixed role in a widget or a child widget; any other file at
/// its root is a topic file.
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

/// Modules that may be `pub(crate) mod` in a widget's `mod.rs`.
pub(crate) const OPEN_WIDGET_MODULES: &[&str] =
    &["event", "types", "view", "storage", "services", CHILD_DIR];

/// Modules that may be `pub(crate) mod` in a child widget's `mod.rs`. A
/// child widget has no route, so it opens no `storage` or `services`, and
/// it has no child widgets of its own.
pub(crate) const OPEN_CHILD_MODULES: &[&str] = &["event", "types", "view"];

/// Modules that must be `pub(crate) mod` when a widget's or a child
/// widget's `mod.rs` declares them: other routes send intents with `event`,
/// `view.rs` and the mode screens call `view`, and the route and the mode
/// screen reach the child widgets through `child`.
pub(crate) const ALWAYS_OPEN_WIDGET_MODULES: &[&str] =
    &["event", "view", CHILD_DIR];

/// The layer a source file belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Layer {
    /// The app loop, the input filter, the routes, the root view and the
    /// screen of each mode.
    Wiring,
    /// A file inside `widgets/<name>/`, also inside one of its child
    /// widgets.
    Widget(String),
    /// A module that any layer may use.
    Shared,
}

/// The layer of the file at `path`, or `None` when it belongs to no layer.
pub(crate) fn layer_of(path: &str) -> Option<Layer> {
    if WIRING_FILES.contains(&path)
        || path.starts_with("router/")
        || path.starts_with("screens/")
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

/// The type that holds a widget, e.g. `QuickLaunchWidget` for
/// `quick_launch`.
pub(crate) fn widget_type_name(widget: &str) -> String {
    format!("{}Widget", camel_case(widget))
}

/// `run_panel` as `RunPanel`.
pub(crate) fn camel_case(snake: &str) -> String {
    snake
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + chars.as_str()
                },
                None => String::new(),
            }
        })
        .collect()
}

/// The path of `path` inside its widget folder, e.g. `view/menu.rs` for
/// `widgets/modes/view/menu.rs`.
pub(crate) fn path_in_widget<'a>(path: &'a str, widget: &str) -> &'a str {
    let prefix_len = "widgets/".len() + widget.len() + 1;

    path.get(prefix_len..).unwrap_or("")
}

/// The child widgets of `widget`: the folders directly under
/// `widgets/<widget>/child/`, e.g. `run` for
/// `widgets/containers/child/run/mod.rs`, sorted.
pub(crate) fn child_names(tree: &SourceTree, widget: &str) -> Vec<String> {
    let prefix = format!("widgets/{widget}/{CHILD_DIR}/");

    let mut names: Vec<String> = tree
        .files()
        .iter()
        .filter_map(|file| {
            let inside = file.path().strip_prefix(&prefix)?;
            let (folder, _) = inside.split_once('/')?;

            Some(folder.to_string())
        })
        .collect();

    names.sort();
    names.dedup();
    names
}

/// The child widget of `widget` that holds the file at `path`, e.g. `run`
/// for `widgets/containers/child/run/view/run_form.rs`; `None` for a file of
/// the widget itself, also for `child/mod.rs`.
pub(crate) fn child_of<'a>(path: &'a str, widget: &str) -> Option<&'a str> {
    let inside = path_in_widget(path, widget)
        .strip_prefix(CHILD_DIR)?
        .strip_prefix('/')?;
    let (child, _) = inside.split_once('/')?;

    Some(child)
}

/// The folder of the widget or the child widget that holds the file at
/// `path`, e.g. `widgets/containers/child/run` for
/// `widgets/containers/child/run/view/run_form.rs`.
pub(crate) fn owner_dir(path: &str, widget: &str) -> String {
    match child_of(path, widget) {
        Some(child) => format!("widgets/{widget}/{CHILD_DIR}/{child}"),
        None => format!("widgets/{widget}"),
    }
}

/// The path of `path` inside the folder of its widget or child widget,
/// e.g. `view/run_form.rs` for
/// `widgets/containers/child/run/view/run_form.rs`.
pub(crate) fn path_in_owner<'a>(path: &'a str, widget: &str) -> &'a str {
    let in_widget = path_in_widget(path, widget);

    match child_of(path, widget) {
        Some(child) => {
            let prefix_len = CHILD_DIR.len() + child.len() + 2;
            in_widget.get(prefix_len..).unwrap_or(in_widget)
        },
        None => in_widget,
    }
}

/// Whether the file at `path` of `widget` belongs to a view: `view.rs` or
/// `view/` of the widget or of one of its child widgets.
pub(crate) fn is_view_file(path: &str, widget: &str) -> bool {
    let inner = path_in_owner(path, widget);

    inner == "view.rs" || inner.starts_with("view/")
}

/// Whether the file at `path` of `widget` is a model file: `model.rs` or a
/// file inside `model/`, of the widget or of one of its child widgets.
pub(crate) fn is_model_file(path: &str, widget: &str) -> bool {
    let inner = path_in_owner(path, widget);

    inner == MODEL_FILE
        || inner
            .strip_prefix(MODEL_DIR)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_get_the_layer_of_their_folder() {
        assert_eq!(layer_of("router/tabs.rs"), Some(Layer::Wiring));
        assert_eq!(layer_of("update.rs"), Some(Layer::Wiring));
        assert_eq!(layer_of("screens/terminal.rs"), Some(Layer::Wiring));
        assert_eq!(layer_of("view/terminal.rs"), None);
        assert_eq!(layer_of("widgets/mod.rs"), Some(Layer::Wiring));
        assert_eq!(
            layer_of("widgets/modes/view/menu.rs"),
            Some(Layer::Widget("modes".to_string()))
        );
        assert_eq!(layer_of("domain/mode.rs"), Some(Layer::Shared));
        assert_eq!(layer_of("theme.rs"), Some(Layer::Shared));
        assert_eq!(layer_of("focus.rs"), Some(Layer::Shared));
        assert_eq!(layer_of("helpers.rs"), None);
    }

    #[test]
    fn a_widget_type_is_named_after_its_folder() {
        assert_eq!(widget_type_name("tabs"), "TabsWidget");
        assert_eq!(widget_type_name("quick_launch"), "QuickLaunchWidget");
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
    fn child_widgets_are_the_folders_under_the_child_folder() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/view/mod.rs", ""),
            ("widgets/containers/model/mod.rs", ""),
            ("widgets/containers/model/main_area.rs", ""),
            ("widgets/containers/child/mod.rs", ""),
            ("widgets/containers/child/run/mod.rs", ""),
            ("widgets/containers/child/run/view/mod.rs", ""),
            ("widgets/containers/child/run/model/mod.rs", ""),
            ("widgets/containers/child/create/state.rs", ""),
            ("widgets/notes/mod.rs", ""),
        ]);

        assert_eq!(child_names(&tree, "containers"), ["create", "run"]);
        assert!(child_names(&tree, "notes").is_empty());
    }

    #[test]
    fn a_folder_outside_the_child_folder_is_no_child_widget() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            ("widgets/containers/run/view/run_form.rs", ""),
            ("widgets/containers/child/mod.rs", ""),
            ("widgets/containers/child/helpers.rs", ""),
        ]);

        assert!(child_names(&tree, "containers").is_empty());
        assert_eq!(
            child_of("widgets/containers/run/view/run_form.rs", "containers"),
            None
        );
    }

    #[test]
    fn a_file_inside_a_child_folder_belongs_to_that_child_widget() {
        let child = |path| child_of(path, "containers");

        assert_eq!(
            child("widgets/containers/child/run/view/run_form.rs"),
            Some("run")
        );
        assert_eq!(child("widgets/containers/child/run/mod.rs"), Some("run"));
        assert_eq!(child("widgets/containers/child/mod.rs"), None);
        assert_eq!(child("widgets/containers/view/main_area.rs"), None);
        assert_eq!(child("widgets/containers/state.rs"), None);
    }

    #[test]
    fn a_child_widget_file_is_placed_inside_its_own_folder() {
        let path = "widgets/containers/child/run/view/run_form.rs";

        assert_eq!(
            owner_dir(path, "containers"),
            "widgets/containers/child/run"
        );
        assert_eq!(path_in_owner(path, "containers"), "view/run_form.rs");
        assert_eq!(
            path_in_owner("widgets/containers/child/mod.rs", "containers"),
            "child/mod.rs"
        );
    }

    #[test]
    fn view_files_are_view_rs_or_inside_view() {
        assert!(is_view_file("widgets/palette/view.rs", "palette"));
        assert!(is_view_file("widgets/modes/view/menu.rs", "modes"));
        assert!(!is_view_file("widgets/modes/model.rs", "modes"));
    }

    #[test]
    fn model_files_are_model_rs_or_inside_a_model_folder() {
        let is_model = |path| is_model_file(path, "containers");

        assert!(is_model("widgets/containers/model.rs"));
        assert!(is_model("widgets/containers/model/mod.rs"));
        assert!(is_model("widgets/containers/model/main_area.rs"));
        assert!(is_model("widgets/containers/child/run/model.rs"));
        assert!(is_model("widgets/containers/child/run/model/run_form.rs"));
        assert!(!is_model("widgets/containers/view/model.rs"));
        assert!(!is_model("widgets/containers/state.rs"));
        assert!(!is_model("widgets/containers/model_rows.rs"));
        assert!(!is_model("widgets/containers/child/run/view/model/x.rs"));
        assert!(!is_model("widgets/containers/run/model.rs"));
    }

    #[test]
    fn a_child_widget_has_its_own_view_files() {
        let is_view = |path| is_view_file(path, "containers");

        assert!(is_view("widgets/containers/child/run/view/run_form.rs"));
        assert!(is_view("widgets/containers/child/run/view.rs"));
        assert!(!is_view("widgets/containers/child/run/state.rs"));
        assert!(!is_view("widgets/containers/child/run/model.rs"));
        assert!(!is_view("widgets/containers/run/view/run_form.rs"));
    }
}
