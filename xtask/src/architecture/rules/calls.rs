//! Rules about who calls routes and reducers.

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprMethodCall, Macro, Member};

use super::super::finding::{Finding, Rule};
use super::super::layout::{part_names, widget_names};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{line_of, macro_exprs, path_names};

/// Files that may call routes directly: the router hands each event to its
/// route, and `update.rs` routes input.
const ROUTE_CALLERS: &[&str] = &["router/mod.rs", "update.rs"];

/// The calls of a file that the rules look at, each with its line.
#[derive(Default)]
struct Calls {
    /// Calls of `route`, keyed by the path written before it, e.g.
    /// `command::route`; a method call is keyed `route`.
    routes: Vec<(String, usize)>,
    /// The widgets of `widgets.<widget>.reduce(..)` calls.
    widget_reduces: Vec<(String, usize)>,
    /// The paths of `a::b::reduce(..)` calls, e.g. `["run", "reduce"]`.
    path_reduces: Vec<(Vec<String>, usize)>,
}

impl<'ast> Visit<'ast> for Calls {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(callee) = call.func.as_ref()
            && let Some(last) = callee.path.segments.last()
        {
            let names = path_names(&callee.path);
            let line = line_of(last.ident.span());
            if last.ident == "route" {
                self.routes.push((names.join("::"), line));
            }
            if last.ident == "reduce" && names.len() > 1 {
                self.path_reduces.push((names, line));
            }
        }

        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let line = line_of(call.method.span());
        if call.method == "route" {
            self.routes.push(("route".to_string(), line));
        }
        if call.method == "reduce"
            && let Some(widget) = widget_of(&call.receiver)
        {
            self.widget_reduces.push((widget, line));
        }

        visit::visit_expr_method_call(self, call);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        for expr in macro_exprs(mac) {
            self.visit_expr(&expr);
        }
    }
}

/// Report direct route calls, `reduce` calls outside a widget's route and
/// `reduce` calls of a part outside its parent's `reducer.rs`.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let parts = all_parts(tree);
    let mut findings = Vec::new();

    for file in tree.files() {
        let mut calls = Calls::default();
        calls.visit_file(file.syntax());

        if !ROUTE_CALLERS.contains(&file.path()) {
            findings.extend(route_calls(file, &calls));
        }
        findings.extend(reduce_calls(file, &calls));
        findings.extend(part_reduce_calls(file, &calls, &parts));
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

fn route_calls(file: &SourceFile, calls: &Calls) -> Vec<Finding> {
    calls
        .routes
        .iter()
        .map(|(callee, line)| {
            Finding::new(Rule::RouteCalledDirectly, file.path(), callee, *line)
        })
        .collect()
}

/// Calls `app.widgets.<widget>.reduce(` outside `router/<widget>.rs`.
fn reduce_calls(file: &SourceFile, calls: &Calls) -> Vec<Finding> {
    calls
        .widget_reduces
        .iter()
        .filter(|(widget, _)| file.path() != format!("router/{widget}.rs"))
        .map(|(widget, line)| {
            Finding::new(
                Rule::ReduceCalledOutsideRoute,
                file.path(),
                widget,
                *line,
            )
        })
        .collect()
}

/// Calls `<part>::reduce(` of one of the `(widget, part)` pairs in `parts`
/// anywhere but in the `reducer.rs` of the part's widget.
fn part_reduce_calls(
    file: &SourceFile,
    calls: &Calls,
    parts: &[(String, String)],
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for (callee, line) in &calls.path_reduces {
        let name = &callee[callee.len() - 2];
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
            &callee.join("::"),
            *line,
        ));
    }

    findings
}

/// `tabs` for the receiver `app.widgets.tabs` or `widgets.tabs`.
fn widget_of(receiver: &Expr) -> Option<String> {
    let Expr::Field(widget) = receiver else {
        return None;
    };
    let Member::Named(name) = &widget.member else {
        return None;
    };

    let owner_is_widgets = match widget.base.as_ref() {
        Expr::Field(owner) => {
            matches!(&owner.member, Member::Named(field) if field == "widgets")
        },
        Expr::Path(owner) => owner.path.is_ident("widgets"),
        _ => false,
    };
    owner_is_widgets.then(|| name.to_string())
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
                "fn route(app: &mut App, event: AppEvent) -> Task<AppEvent> {\n    \
                 match event {\n        AppEvent::Tabs(event) => tabs::route(app, event),\n    }\n}\n",
            ),
            (
                "update.rs",
                "fn update(app: &mut App) {\n    let task = router::route(app, event);\n}\n",
            ),
            (
                "router/palette.rs",
                "pub(crate) fn route(app: &mut App, e: E) -> T {\n    command::route(app, c)\n}\n\
                 fn reroute() {\n    reroute();\n}\n",
            ),
            (
                "router/tabs.rs",
                "fn batch(app: &mut App) -> T {\n    Task::batch(vec![sidebar::route(app, e)])\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::RouteCalledDirectly),
            [
                pair("router/palette.rs", "command::route"),
                pair("router/tabs.rs", "sidebar::route"),
            ]
        );
    }

    #[test]
    fn only_a_widget_route_calls_its_reduce() {
        let tree = SourceTree::from_sources(&[
            (
                "router/tabs.rs",
                "fn route() {\n    app.widgets.tabs.reduce(intent, &ctx)\n}\n",
            ),
            (
                "router/palette.rs",
                "fn route() {\n    app.widgets.tabs.reduce(intent, &ctx)\n}\n",
            ),
            (
                "update.rs",
                "fn update() {\n    let t = app\n        .widgets\n        .tabs\n        .reduce(intent, &ctx);\n}\n",
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
                "fn reduce() {\n    reducer::reduce(&mut self.state, intent)\n}\n",
            ),
            ("widgets/containers/run/mod.rs", ""),
            (
                "widgets/containers/reducer.rs",
                "fn reduce() {\n    let (effects, outcome) = run::reduce(run, intent, &ctx);\n}\n",
            ),
            (
                "widgets/containers/view/main_area.rs",
                "fn view() {\n    let _ = run::reduce(run, intent, &ctx);\n}\n",
            ),
            (
                "router/containers.rs",
                "fn route() {\n    let _ = containers::run::reduce(run, intent, &ctx);\n}\n",
            ),
            (
                "widgets/notes/reducer.rs",
                "fn reduce() {\n    let _ = run::reduce(run, intent, &ctx);\n}\n",
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

    #[test]
    fn method_calls_and_a_local_widgets_list_are_read_too() {
        let tree = SourceTree::from_sources(&[(
            "router/palette.rs",
            "fn route(app: &mut App, widgets: &mut Widgets) {\n    \
             let _ = self.route(event);\n    \
             let _ = widgets.tabs.reduce(intent);\n    \
             let _ = app.other.tabs.reduce(intent);\n    \
             let _ = tabs.reduce(intent);\n}\n",
        )]);

        let findings = check(&tree);

        assert_eq!(
            found(&findings, Rule::RouteCalledDirectly),
            [pair("router/palette.rs", "route")]
        );
        assert_eq!(
            found(&findings, Rule::ReduceCalledOutsideRoute),
            [pair("router/palette.rs", "tabs")]
        );
    }
}
