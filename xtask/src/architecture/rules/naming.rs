//! Rules about names and shapes the widget template fixes: the reducer
//! parameter and how view models are built.

use super::super::finding::{Finding, Rule};
use super::super::layout::{Layer, layer_of};
use super::super::source::{Line, SourceFile, SourceTree};
use super::super::text::{block_end, find_word, is_ident_char};

/// Report a reducer parameter not named `intent` and view model accessors
/// that do not build through `from_state`.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        if !matches!(layer_of(file.path()), Some(Layer::Widget(_))) {
            continue;
        }
        match file.name() {
            "reducer.rs" => findings.extend(reduce_parameter(file)),
            "mod.rs" => {
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
/// inside `<>`, `()` or `[]`; the list ends at its closing `)`.
fn split_params(list: &str) -> Vec<&str> {
    let mut params = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (at, ch) in list.char_indices() {
        match ch {
            '<' | '(' | '[' => depth += 1,
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
