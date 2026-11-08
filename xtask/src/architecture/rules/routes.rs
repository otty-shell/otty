//! The route rule: a route chooses what to do only by the variant of its
//! intent or effect, never by widget state. When the data an effect needs is
//! missing, sending nothing is allowed: that is no decision.

use super::super::finding::{Finding, Rule};
use super::super::source::{Line, SourceTree};
use super::super::text::{
    block_end, brace_balance, contains_word, find_word, is_ident_char,
    is_in_string, join_statement,
};

/// The parameter through which a route reads widget state.
const APP: &str = "app";

/// Expressions that send nothing.
const NOTHING: &[&str] = &[
    "",
    "Task::none()",
    "None",
    "()",
    "continue",
    "break",
    "return",
];

/// Report route functions that read the app and branch on what they read.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        if !file.path().starts_with("router/") {
            continue;
        }
        for function in functions(file.lines()) {
            if !contains_word(&function.signature, APP) {
                continue;
            }
            if let Some(line) = first_state_branch(&function.body) {
                findings.push(Finding::new(
                    Rule::RouteBranchesOnState,
                    file.path(),
                    &function.name,
                    line,
                ));
            }
        }
    }

    findings
}

/// A top-level function of a route file.
struct Function {
    name: String,
    signature: String,
    body: Vec<Line>,
}

/// The top-level functions of `lines`, with their bodies.
fn functions(lines: &[Line]) -> Vec<Function> {
    let mut functions = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let Some(name) = function_name(lines[index].text()) else {
            index += 1;
            continue;
        };

        let mut open = index;
        let mut signature = String::new();
        while open < lines.len() {
            signature.push_str(lines[open].text().trim());
            signature.push(' ');
            if lines[open].text().trim_end().ends_with('{') {
                break;
            }
            open += 1;
        }

        let mut close = open + 1;
        while close < lines.len() && lines[close].text() != "}" {
            close += 1;
        }

        let body_end = close.min(lines.len());
        functions.push(Function {
            name: name.to_string(),
            signature,
            body: lines.get(open + 1..body_end).unwrap_or(&[]).to_vec(),
        });
        index = close + 1;
    }

    functions
}

/// The name of a function declared at the start of `text`, at column 0.
fn function_name(text: &str) -> Option<&str> {
    if text.starts_with(' ') {
        return None;
    }
    let without_vis = text
        .strip_prefix("pub(crate) ")
        .or_else(|| text.strip_prefix("pub(super) "))
        .or_else(|| text.strip_prefix("pub "))
        .unwrap_or(text);
    let rest = without_vis.strip_prefix("fn ")?;
    let length = rest
        .find(|ch: char| !is_ident_char(ch))
        .unwrap_or(rest.len());

    (length > 0).then(|| &rest[..length])
}

/// The line of the first branch on widget state in `body`, if any.
///
/// Names bound by `let` from an expression that mentions `app`, or another
/// such name, carry widget state. A branch whose only job is to skip work
/// when such data is missing is allowed: `let … else { return Task::none() }`,
/// `if let` without a working `else`, and a `match` with an empty `None` or
/// `Err` arm.
fn first_state_branch(body: &[Line]) -> Option<usize> {
    let mut state_names = vec![APP.to_string()];

    for index in 0..body.len() {
        let text = body[index].text().trim();

        if text.starts_with("let ") {
            let (statement, _) = join_statement(body, index);
            if decides_in_let(&statement, &mut state_names) {
                return Some(body[index].number());
            }
        }

        for keyword in ["if", "match"] {
            for at in word_positions(text, keyword) {
                let (header, header_end) =
                    branch_header(body, index, &text[at + keyword.len()..]);
                let decides = if keyword == "if" {
                    decides_in_if(body, header_end, &header, &mut state_names)
                } else {
                    decides_in_match(body, header_end, &header, &state_names)
                };
                if decides {
                    return Some(body[index].number());
                }
            }
        }
    }

    None
}

/// Track the names a `let` binds from state; a `let … else` whose `else`
/// does more than send nothing is a decision.
fn decides_in_let(statement: &str, state_names: &mut Vec<String>) -> bool {
    let Some((pattern, rest)) = statement
        .strip_prefix("let ")
        .and_then(|after| after.split_once(" = "))
    else {
        return false;
    };
    let rest = rest.trim_end_matches(';').trim();

    let (expr, else_body) = split_let_else(rest);
    let from_state = mentions_any(expr, state_names);
    if from_state {
        state_names.extend(bound_names(pattern));
    }

    from_state && else_body.is_some_and(|body| !is_nothing(body))
}

/// An `if` decides when a condition reads state. `if let` that only binds
/// state data is fine unless its `else` does work.
fn decides_in_if(
    body: &[Line],
    header_end: usize,
    header: &str,
    state_names: &mut Vec<String>,
) -> bool {
    let mut binds_state = false;

    for part in header.split(" && ") {
        let part = part.trim();
        if let Some(binding) = part.strip_prefix("let ") {
            let Some((pattern, expr)) = binding.split_once(" = ") else {
                continue;
            };
            if mentions_any(expr, state_names) {
                binds_state = true;
                state_names.extend(bound_names(pattern));
            }
            continue;
        }
        if mentions_any(part, state_names) {
            return true;
        }
    }

    binds_state && has_working_else(body, header_end)
}

/// A `match` decides when its scrutinee reads state, unless an empty `None`
/// or `Err` arm shows it only unpacks data that may be missing.
fn decides_in_match(
    body: &[Line],
    header_end: usize,
    scrutinee: &str,
    state_names: &[String],
) -> bool {
    if !mentions_any(scrutinee, state_names) {
        return false;
    }

    let end = block_end(body, header_end);
    let has_empty_arm = body[header_end..=end].iter().any(|line| {
        let text = line.text().trim();
        let is_missing_arm =
            text.starts_with("None =>") || text.starts_with("Err(");

        is_missing_arm
            && text
                .split_once("=>")
                .is_some_and(|(_, value)| is_nothing(value))
    });

    !has_empty_arm
}

/// Whether the `if` block opened on `body[header_end]` has an `else` that
/// does more than send nothing.
fn has_working_else(body: &[Line], header_end: usize) -> bool {
    let close = block_end(body, header_end);
    let Some(text) = body.get(close).map(|line| line.text().trim()) else {
        return false;
    };
    if text.starts_with("} else if") {
        return true;
    }
    if !text.starts_with("} else") {
        return false;
    }

    let else_end = block_end(body, close);
    let else_body: Vec<&str> = body[close + 1..else_end]
        .iter()
        .map(|line| line.text().trim())
        .collect();

    !is_nothing(&else_body.join(" "))
}

/// The condition or scrutinee after a keyword, joined over lines up to its
/// opening `{` (or `=>` for a match guard), with the index of the line that
/// ends it.
fn branch_header(
    body: &[Line],
    start: usize,
    after_keyword: &str,
) -> (String, usize) {
    let mut header = after_keyword.trim().to_string();
    let mut end = start;

    while !header.contains('{')
        && !header.contains("=>")
        && end + 1 < body.len()
    {
        end += 1;
        header.push(' ');
        header.push_str(body[end].text().trim());
    }

    let cut = [header.find('{'), header.find("=>")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(header.len());

    (header[..cut].trim().to_string(), end)
}

/// Split `expr else { … }` of a `let … else` into the expression and the
/// else block; an `if` or `match` expression has no `else` of its own here.
fn split_let_else(rest: &str) -> (&str, Option<&str>) {
    if rest.starts_with("if ") || rest.starts_with("match ") {
        return (rest, None);
    }

    match rest.rfind(" else {") {
        Some(at) if brace_balance(&rest[..at]) == 0 => {
            let block =
                rest[at + " else {".len()..].trim().trim_end_matches('}');
            (&rest[..at], Some(block))
        },
        _ => (rest, None),
    }
}

/// The variable names a pattern binds, e.g. `cwd` in `Some(cwd)`.
fn bound_names(pattern: &str) -> Vec<String> {
    let pattern = match pattern.find(':') {
        Some(at)
            if !pattern.contains('{') && !pattern[at..].starts_with("::") =>
        {
            &pattern[..at]
        },
        _ => pattern,
    };

    pattern
        .split(|ch: char| !is_ident_char(ch))
        .filter(|word| {
            word.chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_lowercase())
        })
        .filter(|word| !matches!(*word, "mut" | "ref"))
        .map(str::to_string)
        .collect()
}

/// Whether `text` mentions one of `names` as a whole word.
fn mentions_any(text: &str, names: &[String]) -> bool {
    names.iter().any(|name| contains_word(text, name))
}

/// Whether `text` sends nothing: `Task::none()`, `None`, an empty block or
/// a bare `return`.
fn is_nothing(text: &str) -> bool {
    let mut value = text.trim();
    loop {
        let trimmed = value
            .trim()
            .trim_end_matches([',', ';'])
            .trim()
            .trim_start_matches("return")
            .trim();
        let unwrapped = trimmed
            .strip_prefix('{')
            .and_then(|inner| inner.strip_suffix('}'))
            .unwrap_or(trimmed)
            .trim();
        if unwrapped == value {
            break;
        }
        value = unwrapped;
    }

    NOTHING.contains(&value)
}

/// The offsets of `word` in `text` as a whole word, outside strings.
fn word_positions(text: &str, word: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut from = 0;

    while let Some(found) = find_word(&text[from..], word) {
        let at = from + found;
        if !is_in_string(text, at) {
            positions.push(at);
        }
        from = at + word.len();
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    fn route_findings(code: &str) -> Vec<(String, String)> {
        let tree = SourceTree::from_sources(&[("router/notes.rs", code)]);

        found(&check(&tree), Rule::RouteBranchesOnState)
    }

    #[test]
    fn branching_on_the_event_is_allowed() {
        let code = "pub(crate) fn route(app: &mut App, event: NotesEvent) -> Task<AppEvent> {\n    \
            match event {\n        NotesEvent::Intent(intent) => {\n            \
            app.widgets.notes.reduce(intent).map(AppEvent::Notes)\n        },\n        \
            NotesEvent::Effect(effect) => handle_effect(effect),\n    }\n}\n";

        assert!(route_findings(code).is_empty());
    }

    #[test]
    fn an_if_on_widget_state_is_reported() {
        let code = "fn close(app: &App) -> Task<AppEvent> {\n    \
            if app.widgets.modes.active() != Mode::Terminal {\n        return Task::none();\n    }\n\n    \
            Task::none()\n}\n";

        assert_eq!(route_findings(code), [pair("router/notes.rs", "close")]);
    }

    #[test]
    fn a_local_read_from_state_carries_the_state() {
        let code = "fn focus(app: &App, was_open: bool) -> Task<AppEvent> {\n    \
            let is_open = app.widgets.modes.is_menu_open();\n\n    \
            if is_open && !was_open {\n        return focus_menu();\n    }\n\n    Task::none()\n}\n";

        assert_eq!(route_findings(code), [pair("router/notes.rs", "focus")]);
    }

    #[test]
    fn a_local_read_from_the_intent_does_not() {
        let code = "fn handle(app: &mut App, event: NotesIntent) -> Task<AppEvent> {\n    \
            let should_sync = should_sync(&event);\n    \
            let task = app\n        .widgets\n        .notes\n        .reduce(event)\n        .map(AppEvent::Notes);\n\n    \
            if !should_sync {\n        return task;\n    }\n\n    Task::batch([task, sync()])\n}\n";

        assert!(route_findings(code).is_empty());
    }

    #[test]
    fn no_data_for_the_effect_means_nothing_is_sent() {
        let code = "fn sync(app: &mut App) -> Task<AppEvent> {\n    \
            let Some(cwd) = app\n        .widgets\n        .tabs\n        .cwd()\n    else {\n        return Task::none();\n    };\n\n    \
            if let Some(path) = app.widgets.tabs.path() {\n        tasks.push(open(path));\n    }\n\n    \
            if let Some(id) = new_id\n        && let Some(dir) = app.widgets.tabs.dir(id)\n    {\n        tasks.push(open(dir));\n    }\n\n    \
            match app.widgets.tabs.folder() {\n        Some(folder) => show(folder),\n        None => Task::none(),\n    }\n}\n";

        assert!(
            route_findings(code).is_empty(),
            "{:?}",
            route_findings(code)
        );
    }

    #[test]
    fn a_state_condition_in_a_let_chain_is_reported() {
        let code = "fn open(app: &App, new_id: Option<u64>) -> Task<AppEvent> {\n    \
            if let Some(id) = new_id\n        && app.widgets.tabs.is_open(id)\n    {\n        return show(id);\n    }\n\n    Task::none()\n}\n";

        assert_eq!(route_findings(code), [pair("router/notes.rs", "open")]);
    }

    #[test]
    fn an_if_let_on_state_with_a_working_else_is_reported() {
        let code = "fn pick(app: &App) -> Task<AppEvent> {\n    \
            if let Some(x) = app.widgets.tabs.active() {\n        one(x)\n    } else {\n        other()\n    }\n}\n";

        assert_eq!(route_findings(code), [pair("router/notes.rs", "pick")]);
    }

    #[test]
    fn a_match_on_state_without_an_empty_arm_is_reported() {
        let code = "fn pick(app: &App) -> Task<AppEvent> {\n    \
            match app.widgets.modes.active() {\n        Mode::Terminal => one(),\n        _ => other(),\n    }\n}\n";

        assert_eq!(route_findings(code), [pair("router/notes.rs", "pick")]);
    }

    #[test]
    fn functions_without_the_app_and_other_files_are_not_checked() {
        let tree = SourceTree::from_sources(&[
            (
                "router/notes.rs",
                "fn save(pinned: Vec<Mode>) -> Task<AppEvent> {\n    if pinned.is_empty() {\n        return Task::none();\n    }\n    write(pinned)\n}\n",
            ),
            (
                "update.rs",
                "fn filter_input(app: &mut App, event: AppEvent) -> Task<AppEvent> {\n    if app.widgets.modes.is_menu_open() {\n        return Task::none();\n    }\n    Task::none()\n}\n",
            ),
        ]);

        assert!(found(&check(&tree), Rule::RouteBranchesOnState).is_empty());
    }
}
