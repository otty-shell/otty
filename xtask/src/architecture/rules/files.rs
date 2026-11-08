//! Rules about which files exist: layers, the files of a widget and of its
//! parts, the files of `router/` and the screen file of every mode.

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    REQUIRED_WIDGET_FILES, layer_of, part_names, widget_names,
};
use super::super::source::{Line, SourceTree};

/// The file that defines the modes.
const MODE_FILE: &str = "domain/mode.rs";

/// Route files that are not widget routes: the router itself and the route
/// of `AppEvent::Command`.
const NON_WIDGET_ROUTES: &[&str] = &["mod", "command"];

/// Report files outside the layers, missing widget files, unexpected route
/// files and modes without a screen file.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let widgets = widget_names(tree);
    let mut findings = Vec::new();

    findings.extend(files_outside_layers(tree));
    findings.extend(missing_widget_files(tree, &widgets));
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

/// The required files and the `view/` folder that `dir` lacks.
fn missing_files(tree: &SourceTree, dir: &str, key: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for name in REQUIRED_WIDGET_FILES {
        let path = format!("{dir}/{name}");
        if tree.file(&path).is_none() {
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

    mode_variants(mode_file.lines())
        .into_iter()
        .filter_map(|variant| {
            let screen = format!("screens/{}.rs", snake_case(&variant));

            tree.file(&screen).is_none().then(|| {
                Finding::new(Rule::MissingModeScreen, &screen, &variant, 0)
            })
        })
        .collect()
}

/// The variants of `enum Mode`, read from its lines.
fn mode_variants(lines: &[Line]) -> Vec<String> {
    let Some(start) = lines
        .iter()
        .position(|line| line.text().contains("enum Mode {"))
    else {
        return Vec::new();
    };

    lines[start + 1..]
        .iter()
        .map(|line| line.text().trim())
        .take_while(|text| !text.starts_with('}'))
        .filter(|text| !text.starts_with('#'))
        .filter_map(|text| {
            let name: String = text
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric())
                .collect();

            (!name.is_empty()).then_some(name)
        })
        .collect()
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
