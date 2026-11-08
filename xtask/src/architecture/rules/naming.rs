//! Rules about names and shapes the widget template fixes: the reducer
//! parameter, the intent and effect parameters of routes, how view models
//! are built and the props of a view.

use super::super::finding::{Finding, Rule};
use super::super::layout::{Layer, is_model_file, is_view_file, layer_of};
use super::super::source::{Line, SourceFile, SourceTree};
use super::super::text::{block_end, find_word, is_ident_char, name_after};

/// Name ending of the props of a view.
const PROPS_ENDING: &str = "Props";

/// The name a route parameter must have for each type name ending.
const ROUTE_PARAMETER_NAMES: &[(&str, &str)] =
    &[("Intent", "intent"), ("Effect", "effect")];

/// Report a reducer parameter not named `intent`, route parameters not
/// named `intent` or `effect`, view model accessors that do not build
/// through `from_state` and props not named after their view file.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        if file.path().starts_with("router/") {
            findings.extend(route_parameters(file));
            continue;
        }

        let Some(Layer::Widget(widget)) = layer_of(file.path()) else {
            continue;
        };
        if let Some(view) = view_name(tree, file, &widget) {
            findings.extend(props_not_named_after_view(file, &view));
        }
        match file.name() {
            "reducer.rs" => findings.extend(reduce_parameter(file)),
            // The `mod.rs` of a `model/` folder is a model file, not the
            // `mod.rs` that holds `XWidget`.
            "mod.rs" if !is_model_file(tree, file.path(), &widget) => {
                findings.extend(reduce_parameter(file));
                findings.extend(view_models_not_from_state(file));
            },
            _ => {},
        }
    }

    findings
}

/// The name of the `XIntent` parameter of `fn reduce`, when it is not
/// `intent`.
fn reduce_parameter(file: &SourceFile) -> Vec<Finding> {
    let lines = file.lines();
    let mut findings = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if !line.text().contains("fn reduce(") {
            continue;
        }

        let signature = signature_at(lines, index);
        let Some(params) =
            signature.split_once("fn reduce(").map(|(_, rest)| rest)
        else {
            continue;
        };
        for param in split_params(params) {
            let Some((name, kind)) = param.split_once(':') else {
                continue;
            };
            let name = name.trim();
            let kind = kind.trim().trim_start_matches('&').trim();
            if kind.ends_with("Intent") && name != "intent" {
                findings.push(Finding::new(
                    Rule::ReduceParameterName,
                    file.path(),
                    name,
                    line.number(),
                ));
            }
        }
    }

    findings
}

/// Parameters of the functions in a route file whose type is an `XIntent`
/// or an `XEffect` and whose name is not `intent` or `effect`, keyed
/// `function(parameter)`.
fn route_parameters(file: &SourceFile) -> Vec<Finding> {
    let lines = file.lines();
    let mut findings = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let Some(function) = method_name(line.text()) else {
            continue;
        };

        let signature = signature_at(lines, index);
        let Some(list) = parameter_list(&signature, function) else {
            continue;
        };
        for param in split_params(list) {
            let Some((name, kind)) = param.split_once(':') else {
                continue;
            };
            let name = name.trim().trim_start_matches("mut ").trim();
            let Some(expected) = expected_route_name(kind) else {
                continue;
            };
            if name != expected {
                let key = format!("{function}({name})");
                findings.push(Finding::new(
                    Rule::RouteParameterName,
                    file.path(),
                    &key,
                    line.number(),
                ));
            }
        }
    }

    findings
}

/// The text after the `(` that opens the parameters of `fn function` in
/// `signature`, past its generic parameters, e.g. `a: u8) {` for
/// `fn pick<F: Fn(u8) -> u8>(a: u8) {`.
fn parameter_list<'a>(signature: &'a str, function: &str) -> Option<&'a str> {
    let start = find_word(signature, "fn")?;
    let after_fn = &signature[start + "fn".len()..];
    let after_name = after_fn.trim_start().strip_prefix(function)?;

    let mut depth = 0;
    let mut previous = ' ';
    for (at, ch) in after_name.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' if previous != '-' => depth -= 1,
            '(' if depth == 0 => return Some(&after_name[at + 1..]),
            _ => {},
        }
        previous = ch;
    }

    None
}

/// `intent` or `effect` when the type `kind` of a parameter is a plain
/// path that ends in `Intent` or `Effect`, after `&`, a lifetime, `mut` and
/// generic arguments are left out; `None` for any other type, such as
/// `Vec<XIntent>` or `fn() -> XIntent`.
fn expected_route_name(kind: &str) -> Option<&'static str> {
    let mut kind = kind.trim().trim_start_matches('&').trim_start();
    if kind.starts_with('\'') {
        kind = kind.split_once(' ').map_or("", |(_, rest)| rest);
    }
    let kind = kind.trim_start_matches("mut ").trim();
    let path = kind.split('<').next().unwrap_or(kind).trim();
    let is_plain_path = path.chars().all(|ch| is_ident_char(ch) || ch == ':');
    if !is_plain_path {
        return None;
    }

    let type_name = path.rsplit("::").next().unwrap_or(path);
    ROUTE_PARAMETER_NAMES
        .iter()
        .find(|(ending, _)| type_name.ends_with(ending))
        .map(|(_, name)| *name)
}

/// View model accessors (`vm` and `<part>_vm`) whose body does not call
/// `from_state`.
fn view_models_not_from_state(file: &SourceFile) -> Vec<Finding> {
    let lines = file.lines();
    let mut findings = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let Some(name) = method_name(line.text()) else {
            continue;
        };
        if name != "vm" && !name.ends_with("_vm") {
            continue;
        }

        let Some(open) =
            (index..lines.len()).find(|at| lines[*at].text().contains('{'))
        else {
            continue;
        };
        let end = block_end(lines, open);
        let builds_from_state = lines[index..=end]
            .iter()
            .any(|body| body.text().contains("from_state("));
        if !builds_from_state {
            findings.push(Finding::new(
                Rule::ViewModelNotFromState,
                file.path(),
                name,
                line.number(),
            ));
        }
    }

    findings
}

/// Props structs in the file of the view `view` that are not named
/// `<View>Props`.
fn props_not_named_after_view(file: &SourceFile, view: &str) -> Vec<Finding> {
    let expected = format!("{view}{PROPS_ENDING}");

    file.lines()
        .iter()
        .filter_map(|line| {
            let name = name_after(line.text(), "struct")?;
            let is_other_props =
                name.ends_with(PROPS_ENDING) && name != expected;

            is_other_props.then(|| {
                Finding::new(
                    Rule::ViewPropsName,
                    file.path(),
                    name,
                    line.number(),
                )
            })
        })
        .collect()
}

/// The name of the view that `file` holds, in CamelCase, e.g. `RunPanel` for
/// `widgets/containers/run/view/run_panel.rs`; `None` for a file that is not
/// one view of `widget` or of a part, such as `view/mod.rs`.
fn view_name(
    tree: &SourceTree,
    file: &SourceFile,
    widget: &str,
) -> Option<String> {
    let name = file.name();
    if !is_view_file(tree, file.path(), widget)
        || name == "mod.rs"
        || name == "view.rs"
    {
        return None;
    }

    name.strip_suffix(".rs").map(camel_case)
}

/// `run_panel` as `RunPanel`.
fn camel_case(snake: &str) -> String {
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

/// The signature that starts at `lines[start]`, joined up to its `{`.
fn signature_at(lines: &[Line], start: usize) -> String {
    let mut joined = String::new();

    for line in &lines[start..] {
        joined.push_str(line.text().trim());
        joined.push(' ');
        if line.text().contains('{') {
            break;
        }
    }

    joined
}

/// The parameters of a parameter list, split at the commas that are not
/// inside `<>`, `()` or `[]`; the list ends at its closing `)`. The `>` of
/// a `->` closes nothing.
fn split_params(list: &str) -> Vec<&str> {
    let mut params = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    let mut previous = ' ';

    for (at, ch) in list.char_indices() {
        let after = previous;
        previous = ch;
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' if after == '-' => {},
            '>' | ']' => depth -= 1,
            ')' if depth == 0 => {
                params.push(&list[start..at]);
                return params;
            },
            ')' => depth -= 1,
            ',' if depth == 0 => {
                params.push(&list[start..at]);
                start = at + 1;
            },
            _ => {},
        }
    }

    params.push(&list[start..]);
    params
}

/// `tree_vm` for `pub(crate) fn tree_vm(&self) -> X {`.
fn method_name(text: &str) -> Option<&str> {
    let start = find_word(text, "fn")? + "fn".len();
    let rest = text[start..].trim_start();
    let length = rest
        .find(|ch: char| !is_ident_char(ch))
        .unwrap_or(rest.len());

    (length > 0).then(|| &rest[..length])
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    #[test]
    fn the_reducer_parameter_is_named_intent() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/notes/mod.rs",
                "impl NotesWidget {\n    pub(crate) fn reduce(&mut self, intent: NotesIntent) -> Task<NotesEvent> {\n        reducer::reduce(&mut self.state, intent)\n    }\n}\n",
            ),
            (
                "widgets/tabs/reducer.rs",
                "pub(crate) fn reduce(\n    state: &mut TabsState,\n    event: TabsIntent,\n    ctx: &TabsCtx<'_>,\n) -> Task<TabsEvent> {\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ReduceParameterName),
            [pair("widgets/tabs/reducer.rs", "event")]
        );
    }

    #[test]
    fn route_parameters_are_named_intent_and_effect() {
        let tree = SourceTree::from_sources(&[
            (
                "router/notes.rs",
                "pub(crate) fn route(app: &mut App, event: NotesEvent) -> Task<AppEvent> {\n}\n\
                 fn handle_intent(app: &mut App, intent: NotesIntent) -> Task<AppEvent> {\n}\n\
                 fn handle_effect(event: NotesEffect) -> Task<AppEvent> {\n}\n\
                 fn wait(\n    delay: Duration,\n    next: &crate::widgets::notes::event::NotesIntent,\n) -> Task<AppEvent> {\n}\n\
                 fn sync(mut effect: NotesEffect, queued: Vec<NotesIntent>) {\n}\n\
                 fn pick<'a, F: Fn(u8) -> u8>(on: F, cause: &'a TabsIntent<'a>) {\n}\n\
                 fn on_load(loaded: fn(&[u8]) -> NotesIntent) {\n}\n\
                 fn spawn(make: impl Fn(u8) -> u8, event: NotesIntent) {\n}\n",
            ),
            (
                "widgets/notes/reducer.rs",
                "fn apply(state: &mut NotesState, change: NotesIntent) {\n}\n",
            ),
            ("update.rs", "fn forward(event: NotesIntent) {\n}\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::RouteParameterName),
            [
                pair("router/notes.rs", "handle_effect(event)"),
                pair("router/notes.rs", "pick(cause)"),
                pair("router/notes.rs", "spawn(event)"),
                pair("router/notes.rs", "wait(next)"),
            ]
        );
    }

    #[test]
    fn props_in_a_view_file_are_named_after_the_file() {
        let tree = SourceTree::from_sources(&[
            ("widgets/notes/mod.rs", ""),
            (
                "widgets/notes/view/notes_panel.rs",
                "pub(crate) struct NotesPanelProps<'a> {\n}\n",
            ),
            (
                "widgets/notes/view/status_bar.rs",
                "pub(crate) fn view() {}\n",
            ),
            (
                "widgets/notes/model.rs",
                "pub(crate) struct NotesProps {\n}\n",
            ),
            (
                "widgets/palette/view/command_palette.rs",
                "pub(crate) struct PaletteProps<'a> {\n}\n",
            ),
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            (
                "widgets/containers/run/view/run_panel.rs",
                "pub(crate) struct RunPanelProps<'a> {\n}\nstruct ImagePanelProps {\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ViewPropsName),
            [
                pair(
                    "widgets/containers/run/view/run_panel.rs",
                    "ImagePanelProps"
                ),
                pair("widgets/palette/view/command_palette.rs", "PaletteProps"),
            ]
        );
    }

    #[test]
    fn the_mod_file_of_a_model_folder_holds_no_view_model_accessors() {
        let tree = SourceTree::from_sources(&[
            ("widgets/notes/mod.rs", ""),
            (
                "widgets/notes/model/mod.rs",
                "fn row_vm(row: &Row) -> RowViewModel {\n    RowViewModel {\n        label: row.label(),\n    }\n}\n",
            ),
        ]);

        assert!(found(&check(&tree), Rule::ViewModelNotFromState).is_empty());
    }

    #[test]
    fn view_model_accessors_build_through_from_state() {
        let tree = SourceTree::from_sources(&[(
            "widgets/notes/mod.rs",
            "impl NotesWidget {\n    pub(crate) fn vm(&self) -> NotesViewModel<'_> {\n        NotesViewModel::from_state(&self.state)\n    }\n\n    \
             pub(crate) fn tree_vm(&self) -> TreeViewModel {\n        TreeViewModel {\n            rows: self.state.rows(),\n        }\n    }\n}\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::ViewModelNotFromState),
            [pair("widgets/notes/mod.rs", "tree_vm")]
        );
    }
}
