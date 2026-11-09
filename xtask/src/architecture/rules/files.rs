//! Rules about which files exist: layers, the files of a widget and of its
//! parts, the files of a `model/` folder, the files of `router/` and the
//! screen file of every mode.

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    Layer, MODEL_DIR, MODEL_FILE, REQUIRED_WIDGET_FILES, layer_of, owner_dir,
    part_names, path_in_owner, widget_names,
};
use super::super::source::SourceTree;

/// The file that defines the modes.
const MODE_FILE: &str = "domain/mode.rs";

/// Route files that are not widget routes: the router itself and the route
/// of `AppEvent::Command`.
const NON_WIDGET_ROUTES: &[&str] = &["mod", "command"];

/// Report files outside the layers, missing widget files, model files not
/// named after a view, unexpected route files and modes without a screen
/// file.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let widgets = widget_names(tree);
    let mut findings = Vec::new();

    findings.extend(files_outside_layers(tree));
    findings.extend(missing_widget_files(tree, &widgets));
    findings.extend(model_files_without_view(tree));
    findings.extend(unexpected_router_files(tree, &widgets));
    findings.extend(missing_mode_screens(tree));

    findings
}

fn files_outside_layers(tree: &SourceTree) -> Vec<Finding> {
    tree.files()
        .iter()
        .filter(|file| layer_of(file.path()).is_none())
        .map(|file| {
            Finding::new(Rule::FileOutsideLayers, file.path(), "no layer", 0)
        })
        .collect()
}

/// The required files, `view/` and route of every widget, and the required
/// files and `view/` of every part, keyed `<widget>/<part>`.
fn missing_widget_files(tree: &SourceTree, widgets: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for widget in widgets {
        let dir = format!("widgets/{widget}");
        findings.extend(missing_files(tree, &dir, widget));

        for part in part_names(tree, widget) {
            let part_dir = format!("{dir}/{part}");
            let key = format!("{widget}/{part}");
            findings.extend(missing_files(tree, &part_dir, &key));
        }

        let route = format!("router/{widget}.rs");
        if tree.file(&route).is_none() {
            findings.push(Finding::new(
                Rule::MissingWidgetFile,
                &route,
                widget,
                0,
            ));
        }
    }

    findings
}

/// The required files and the `view/` folder that `dir` lacks. A
/// `model/mod.rs` stands for `model.rs`.
fn missing_files(tree: &SourceTree, dir: &str, key: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for name in REQUIRED_WIDGET_FILES {
        let path = format!("{dir}/{name}");
        let model_dir = format!("{dir}/{MODEL_DIR}/mod.rs");
        let is_model_dir =
            *name == MODEL_FILE && tree.file(&model_dir).is_some();
        if tree.file(&path).is_none() && !is_model_dir {
            findings.push(Finding::new(Rule::MissingWidgetFile, &path, key, 0));
        }
    }

    let view_dir = format!("{dir}/view");
    if !tree.has_dir(&view_dir) {
        let missing = format!("{view_dir}/");
        findings.push(Finding::new(Rule::MissingWidgetFile, &missing, key, 0));
    }

    findings
}

/// The files of a `model/` folder, other than its `mod.rs`, that have no
/// view file of the same name in the same widget or part, keyed by the
/// view file they should be named after.
fn model_files_without_view(tree: &SourceTree) -> Vec<Finding> {
    tree.files()
        .iter()
        .filter_map(|file| {
            let Some(Layer::Widget(widget)) = layer_of(file.path()) else {
                return None;
            };

            let inner = path_in_owner(tree, file.path(), &widget);
            let view_file = inner.strip_prefix(MODEL_DIR)?.strip_prefix('/')?;
            if view_file == "mod.rs" {
                return None;
            }

            let owner = owner_dir(tree, file.path(), &widget);
            let view = format!("{owner}/view/{view_file}");
            tree.file(&view).is_none().then(|| {
                Finding::new(Rule::ModelFileWithoutView, file.path(), &view, 0)
            })
        })
        .collect()
}

fn unexpected_router_files(
    tree: &SourceTree,
    widgets: &[String],
) -> Vec<Finding> {
    tree.files()
        .iter()
        .filter_map(|file| {
            let name = file.path().strip_prefix("router/")?;
            let module = name.strip_suffix(".rs").unwrap_or(name);
            let is_route = NON_WIDGET_ROUTES.contains(&module)
                || widgets.iter().any(|widget| widget == module);

            (!is_route).then(|| {
                Finding::new(Rule::UnexpectedRouterFile, file.path(), module, 0)
            })
        })
        .collect()
}

fn missing_mode_screens(tree: &SourceTree) -> Vec<Finding> {
    let Some(mode_file) = tree.file(MODE_FILE) else {
        return Vec::new();
    };

    mode_variants(mode_file.syntax())
        .into_iter()
        .filter_map(|variant| {
            let screen = format!("screens/{}.rs", snake_case(&variant));

            tree.file(&screen).is_none().then(|| {
                Finding::new(Rule::MissingModeScreen, &screen, &variant, 0)
            })
        })
        .collect()
}

/// The variants of the top-level `enum Mode`.
fn mode_variants(file: &syn::File) -> Vec<String> {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(mode) if mode.ident == "Mode" => Some(mode),
            _ => None,
        })
        .map(|mode| {
            mode.variants
                .iter()
                .map(|variant| variant.ident.to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// `QuickLaunch` as `quick_launch`.
fn snake_case(name: &str) -> String {
    let mut snake = String::new();

    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }

    snake
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    const COMPLETE_WIDGET: &[(&str, &str)] = &[
        ("widgets/notes/mod.rs", ""),
        ("widgets/notes/event.rs", ""),
        ("widgets/notes/state.rs", ""),
        ("widgets/notes/reducer.rs", ""),
        ("widgets/notes/model.rs", ""),
        ("widgets/notes/view/mod.rs", ""),
        ("router/mod.rs", ""),
        ("router/notes.rs", ""),
    ];

    #[test]
    fn a_complete_widget_has_no_missing_files() {
        let tree = SourceTree::from_sources(COMPLETE_WIDGET);

        assert!(found(&check(&tree), Rule::MissingWidgetFile).is_empty());
    }

    #[test]
    fn a_missing_state_file_and_route_are_reported() {
        let tree = SourceTree::from_sources(&[
            ("widgets/notes/mod.rs", ""),
            ("widgets/notes/event.rs", ""),
            ("widgets/notes/reducer.rs", ""),
            ("widgets/notes/model.rs", ""),
            ("widgets/notes/view/mod.rs", ""),
            ("router/mod.rs", ""),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::MissingWidgetFile),
            [
                pair("router/notes.rs", "notes"),
                pair("widgets/notes/state.rs", "notes"),
            ]
        );
    }

    #[test]
    fn a_view_file_instead_of_a_view_folder_is_reported() {
        let tree = SourceTree::from_sources(&[
            ("widgets/notes/mod.rs", ""),
            ("widgets/notes/event.rs", ""),
            ("widgets/notes/state.rs", ""),
            ("widgets/notes/reducer.rs", ""),
            ("widgets/notes/model.rs", ""),
            ("widgets/notes/view.rs", ""),
            ("router/notes.rs", ""),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::MissingWidgetFile),
            [pair("widgets/notes/view/", "notes")]
        );
    }

    #[test]
    fn a_complete_part_has_no_missing_files_and_needs_no_route() {
        let mut sources = COMPLETE_WIDGET.to_vec();
        sources.extend_from_slice(&[
            ("widgets/notes/editor/mod.rs", ""),
            ("widgets/notes/editor/event.rs", ""),
            ("widgets/notes/editor/state.rs", ""),
            ("widgets/notes/editor/reducer.rs", ""),
            ("widgets/notes/editor/model.rs", ""),
            ("widgets/notes/editor/view/mod.rs", ""),
        ]);
        let tree = SourceTree::from_sources(&sources);

        let findings = check(&tree);

        assert!(found(&findings, Rule::MissingWidgetFile).is_empty());
        assert!(found(&findings, Rule::UnexpectedRouterFile).is_empty());
    }

    #[test]
    fn a_part_lacking_a_file_or_its_view_folder_is_reported() {
        let mut sources = COMPLETE_WIDGET.to_vec();
        sources.extend_from_slice(&[
            ("widgets/notes/editor/mod.rs", ""),
            ("widgets/notes/editor/event.rs", ""),
            ("widgets/notes/editor/reducer.rs", ""),
            ("widgets/notes/editor/model.rs", ""),
            ("widgets/notes/editor/view.rs", ""),
        ]);
        let tree = SourceTree::from_sources(&sources);

        assert_eq!(
            found(&check(&tree), Rule::MissingWidgetFile),
            [
                pair("widgets/notes/editor/state.rs", "notes/editor"),
                pair("widgets/notes/editor/view/", "notes/editor"),
            ]
        );
    }

    /// The notes widget with its `editor` part, both keeping their view
    /// models in a `model/` folder, and the files in `extra`.
    fn with_model_folders(extra: &[(&str, &str)]) -> SourceTree {
        let mut sources = vec![
            ("widgets/notes/mod.rs", ""),
            ("widgets/notes/event.rs", ""),
            ("widgets/notes/state.rs", ""),
            ("widgets/notes/reducer.rs", ""),
            ("widgets/notes/model/mod.rs", ""),
            ("widgets/notes/model/notes_panel.rs", ""),
            ("widgets/notes/view/mod.rs", ""),
            ("widgets/notes/view/notes_panel.rs", ""),
            ("widgets/notes/editor/mod.rs", ""),
            ("widgets/notes/editor/event.rs", ""),
            ("widgets/notes/editor/state.rs", ""),
            ("widgets/notes/editor/reducer.rs", ""),
            ("widgets/notes/editor/model/mod.rs", ""),
            ("widgets/notes/editor/model/editor_form.rs", ""),
            ("widgets/notes/editor/view/mod.rs", ""),
            ("widgets/notes/editor/view/editor_form.rs", ""),
            ("router/mod.rs", ""),
            ("router/notes.rs", ""),
        ];
        sources.extend_from_slice(extra);

        SourceTree::from_sources(&sources)
    }

    #[test]
    fn a_model_folder_takes_the_place_of_model_rs() {
        let tree = with_model_folders(&[]);

        let findings = check(&tree);

        assert!(found(&findings, Rule::MissingWidgetFile).is_empty());
        assert!(found(&findings, Rule::ModelFileWithoutView).is_empty());
    }

    #[test]
    fn a_model_folder_without_its_mod_file_is_no_model() {
        let tree = SourceTree::from_sources(&[
            ("widgets/notes/mod.rs", ""),
            ("widgets/notes/event.rs", ""),
            ("widgets/notes/state.rs", ""),
            ("widgets/notes/reducer.rs", ""),
            ("widgets/notes/model/notes_panel.rs", ""),
            ("widgets/notes/view/mod.rs", ""),
            ("widgets/notes/view/notes_panel.rs", ""),
            ("router/notes.rs", ""),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::MissingWidgetFile),
            [pair("widgets/notes/model.rs", "notes")]
        );
    }

    #[test]
    fn each_file_of_a_model_folder_is_named_after_a_view_of_its_own() {
        let tree = with_model_folders(&[
            ("widgets/notes/model/helpers.rs", ""),
            ("widgets/notes/editor/model/notes_panel.rs", ""),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ModelFileWithoutView),
            [
                pair(
                    "widgets/notes/editor/model/notes_panel.rs",
                    "widgets/notes/editor/view/notes_panel.rs"
                ),
                pair(
                    "widgets/notes/model/helpers.rs",
                    "widgets/notes/view/helpers.rs"
                ),
            ]
        );
    }

    #[test]
    fn router_holds_only_widget_routes_and_the_command_route() {
        let mut sources = COMPLETE_WIDGET.to_vec();
        sources.push(("router/command.rs", ""));
        sources.push(("router/shortcuts.rs", ""));
        let tree = SourceTree::from_sources(&sources);

        assert_eq!(
            found(&check(&tree), Rule::UnexpectedRouterFile),
            [pair("router/shortcuts.rs", "shortcuts")]
        );
    }

    #[test]
    fn a_file_outside_every_layer_is_reported() {
        let tree =
            SourceTree::from_sources(&[("helpers.rs", ""), ("theme.rs", "")]);

        assert_eq!(
            found(&check(&tree), Rule::FileOutsideLayers),
            [pair("helpers.rs", "no layer")]
        );
    }

    #[test]
    fn every_mode_needs_a_screen_file() {
        let mode = "pub(crate) enum Mode {\n    #[default]\n    Terminal,\n    Containers,\n}\n";
        let missing = SourceTree::from_sources(&[
            ("domain/mode.rs", mode),
            ("screens/terminal.rs", ""),
            ("view/containers.rs", ""),
        ]);
        let complete = SourceTree::from_sources(&[
            ("domain/mode.rs", mode),
            ("screens/terminal.rs", ""),
            ("screens/containers.rs", ""),
        ]);

        assert_eq!(
            found(&check(&missing), Rule::MissingModeScreen),
            [pair("screens/containers.rs", "Containers")]
        );
        assert!(found(&check(&complete), Rule::MissingModeScreen).is_empty());
    }
}
