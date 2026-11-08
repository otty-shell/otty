//! Rules about who calls routes and reducers.

use super::super::finding::{Finding, Rule};
use super::super::source::{SourceFile, SourceTree};
use super::super::text::is_ident_char;

/// Files that may call routes directly: the router hands each event to its
/// route, and `update.rs` routes input.
const ROUTE_CALLERS: &[&str] = &["router/mod.rs", "update.rs"];

/// Report direct route calls and `reduce` calls outside a widget's route.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        if !ROUTE_CALLERS.contains(&file.path()) {
            findings.extend(route_calls(file));
        }
        findings.extend(reduce_calls(file));
    }

    findings
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
}
