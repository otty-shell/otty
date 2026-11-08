//! Rules about who calls routes and reducers.

use super::super::finding::{Finding, Rule};
use super::super::layout::{part_names, widget_names};
use super::super::source::{SourceFile, SourceTree};
use super::super::text::is_ident_char;

/// Files that may call routes directly: the router hands each event to its
/// route, and `update.rs` routes input.
const ROUTE_CALLERS: &[&str] = &["router/mod.rs", "update.rs"];

/// Report direct route calls, `reduce` calls outside a widget's route and
/// `reduce` calls of a part outside its parent's `reducer.rs`.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let parts = all_parts(tree);
    let mut findings = Vec::new();

    for file in tree.files() {
        if !ROUTE_CALLERS.contains(&file.path()) {
            findings.extend(route_calls(file));
        }
        findings.extend(reduce_calls(file));
        findings.extend(part_reduce_calls(file, &parts));
    }

    findings
}

/// The parts of every widget of `tree`, as `(widget, part)` pairs.
fn all_parts(tree: &SourceTree) -> Vec<(String, String)> {
    widget_names(tree)
        .into_iter()
        .flat_map(|widget| {
            part_names(tree, &widget)
                .into_iter()
                .map(move |part| (widget.clone(), part))
        })
        .collect()
}

fn route_calls(file: &SourceFile) -> Vec<Finding> {
    let mut findings = Vec::new();

    for line in file.lines() {
        let text = line.text();
        let mut from = 0;
        while let Some(found) = text[from..].find("route(") {
            let at = from + found;
            from = at + "route(".len();

            let before = &text[..at];
            if before.chars().next_back().is_some_and(is_ident_char)
                || before.trim_end().ends_with("fn")
            {
                continue;
            }

            let callee = format!("{}route", path_before(before));
            findings.push(Finding::new(
                Rule::RouteCalledDirectly,
                file.path(),
                &callee,
                line.number(),
            ));
        }
    }

    findings
}

/// The `a::b::` path that ends right before a call, e.g. `command::` in
/// `command::route(`.
fn path_before(before: &str) -> &str {
    let start = before
        .char_indices()
        .rev()
        .find(|(_, ch)| !is_ident_char(*ch) && *ch != ':')
        .map_or(0, |(at, ch)| at + ch.len_utf8());

    &before[start..]
}

/// Calls `app.widgets.<widget>.reduce(` outside `router/<widget>.rs`, also
/// when the chain spans several lines.
fn reduce_calls(file: &SourceFile) -> Vec<Finding> {
    let mut compact = String::new();
    let mut starts = Vec::new();
    for line in file.lines() {
        starts.push((compact.len(), line.number()));
        compact.push_str(line.text().trim());
    }

    let mut findings = Vec::new();
    let mut from = 0;
    while let Some(found) = compact[from..].find(".reduce(") {
        let at = from + found;
        from = at + ".reduce(".len();

        let Some(widget) = widget_before(&compact[..at]) else {
            continue;
        };
        if file.path() == format!("router/{widget}.rs") {
            continue;
        }

        let line = starts
            .iter()
            .take_while(|(start, _)| *start <= at)
            .last()
            .map_or(0, |(_, number)| *number);
        findings.push(Finding::new(
            Rule::ReduceCalledOutsideRoute,
            file.path(),
            widget,
            line,
        ));
    }

    findings
}

/// Calls `<part>::reduce(` of one of the `(widget, part)` pairs in `parts`
/// anywhere but in the `reducer.rs` of the part's widget.
fn part_reduce_calls(
    file: &SourceFile,
    parts: &[(String, String)],
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for line in file.lines() {
        let text = line.text();
        let mut from = 0;
        while let Some(found) = text[from..].find("::reduce(") {
            let at = from + found;
            from = at + "::reduce(".len();

            let callee = path_before(&text[..at]);
            let name = callee.rsplit("::").next().unwrap_or(callee);
            let is_part = parts.iter().any(|(_, part)| part == name);
            let in_parent_reducer = parts.iter().any(|(widget, part)| {
                part == name
                    && file.path() == format!("widgets/{widget}/reducer.rs")
            });
            if !is_part || in_parent_reducer {
                continue;
            }

            findings.push(Finding::new(
                Rule::PartReduceOutsideParent,
                file.path(),
                &format!("{callee}::reduce"),
                line.number(),
            ));
        }
    }

    findings
}

/// `tabs` when `before` ends with `widgets.tabs`.
fn widget_before(before: &str) -> Option<&str> {
    let start = before
        .char_indices()
        .rev()
        .find(|(_, ch)| !is_ident_char(*ch))
        .map_or(0, |(at, ch)| at + ch.len_utf8());
    let widget = &before[start..];

    let owner = before[..start].strip_suffix('.')?;
    (owner.ends_with("widgets") && !widget.is_empty()).then_some(widget)
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    #[test]
    fn only_the_router_and_update_call_routes() {
        let tree = SourceTree::from_sources(&[
            (
                "router/mod.rs",
                "AppEvent::Tabs(event) => tabs::route(app, event),\n",
            ),
            ("update.rs", "let task = router::route(app, event);\n"),
            (
                "router/palette.rs",
                "pub(crate) fn route(app: &mut App, e: E) -> T {\n    command::route(app, c)\n}\nfn reroute() {}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::RouteCalledDirectly),
            [pair("router/palette.rs", "command::route")]
        );
    }

    #[test]
    fn only_a_widget_route_calls_its_reduce() {
        let tree = SourceTree::from_sources(&[
            ("router/tabs.rs", "app.widgets.tabs.reduce(intent, &ctx)\n"),
            (
                "router/palette.rs",
                "app.widgets.tabs.reduce(intent, &ctx)\n",
            ),
            (
                "update.rs",
                "let t = app\n    .widgets\n    .tabs\n    .reduce(intent, &ctx);\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ReduceCalledOutsideRoute),
            [pair("router/palette.rs", "tabs"), pair("update.rs", "tabs")]
        );
    }

    #[test]
    fn only_the_parent_reducer_calls_the_reduce_of_a_part() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/mod.rs",
                "reducer::reduce(&mut self.state, intent)\n",
            ),
            ("widgets/containers/run/mod.rs", ""),
            (
                "widgets/containers/reducer.rs",
                "let (effects, outcome) = run::reduce(run, intent, &ctx);\n",
            ),
            (
                "widgets/containers/view/main_area.rs",
                "let _ = run::reduce(run, intent, &ctx);\n",
            ),
            (
                "router/containers.rs",
                "let _ = containers::run::reduce(run, intent, &ctx);\n",
            ),
            (
                "widgets/notes/reducer.rs",
                "let _ = run::reduce(run, intent, &ctx);\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::PartReduceOutsideParent),
            [
                pair("router/containers.rs", "containers::run::reduce"),
                pair("widgets/containers/view/main_area.rs", "run::reduce"),
                pair("widgets/notes/reducer.rs", "run::reduce"),
            ]
        );
    }
}
