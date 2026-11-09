//! Rules about who calls routes and reducers.

use syn::visit::{self, Visit};
use syn::{
    Expr, ExprCall, ExprMethodCall, FnArg, ImplItemFn, ItemFn, Local, Macro,
    Member, Pat, Signature, Type, UnOp,
};

use super::super::finding::{Finding, Rule};
use super::super::layout::{child_names, widget_names, widget_type_name};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{
    ImportAlias, expanded, import_aliases, line_of, macro_exprs, path_names,
};

/// Files that may call routes directly: the router hands each event to its
/// route, and `update.rs` routes input.
const ROUTE_CALLERS: &[&str] = &["router/mod.rs", "update.rs"];

/// The field of `App` that holds every widget.
const WIDGETS_FIELD: &str = "widgets";

/// The type that holds every widget.
const WIDGETS_TYPE: &str = "Widgets";

/// Name ending of the type that holds one widget.
const WIDGET_ENDING: &str = "Widget";

/// A widget, by its field in `Widgets` or by its `XWidget` type.
#[derive(Debug, Clone)]
enum WidgetRef {
    Field(String),
    Type(String),
}

impl WidgetRef {
    /// The name of the widget among `widgets`, e.g. `tabs` for the field
    /// `tabs` or the type `TabsWidget`; `None` for a type of no widget.
    fn widget(&self, widgets: &[String]) -> Option<String> {
        match self {
            WidgetRef::Field(name) => Some(name.clone()),
            WidgetRef::Type(name) => widgets
                .iter()
                .find(|widget| widget_type_name(widget) == *name)
                .cloned(),
        }
    }
}

/// What a local name or a parameter holds, for finding the `reduce` calls
/// made through it.
#[derive(Debug, Clone)]
enum Held {
    /// Every widget: `app.widgets` or a `Widgets`.
    Widgets,
    /// One widget.
    Widget(WidgetRef),
}

/// The calls of a file that the rules look at, each with its line.
struct Calls {
    /// The names that the imports of the file bring in.
    aliases: Vec<ImportAlias>,
    /// The names that hold widgets in the function the walk is in.
    held: Vec<(String, Held)>,
    /// Calls of `route`, keyed by the path written before it, e.g.
    /// `command::route`; a method call is keyed `route`.
    routes: Vec<(String, usize)>,
    /// The widgets of `reduce` calls on a widget: `widgets.<widget>`, a
    /// name that holds it, or `XWidget::reduce(..)`.
    widget_reduces: Vec<(WidgetRef, usize)>,
    /// The `a::b::reduce(..)` calls, as written and with the first segment
    /// resolved through the imports, e.g. `run_reduce` and
    /// `["run", "reduce"]`.
    path_reduces: Vec<(String, Vec<String>, usize)>,
}

impl Calls {
    /// Walk `file`, reading its imports first.
    fn of(file: &SourceFile) -> Self {
        let mut calls = Self {
            aliases: import_aliases(file.syntax()),
            held: Vec::new(),
            routes: Vec::new(),
            widget_reduces: Vec::new(),
            path_reduces: Vec::new(),
        };
        calls.visit_file(file.syntax());

        calls
    }

    /// What `expr` holds: every widget or one widget.
    fn held_by(&self, expr: &Expr) -> Option<Held> {
        if let Some(widget) = self.widget_in(expr) {
            return Some(Held::Widget(widget));
        }

        self.is_widgets(expr).then_some(Held::Widgets)
    }

    /// The widget that `expr` is, behind references: `<widgets>.<widget>`
    /// or a name that holds one widget.
    fn widget_in(&self, expr: &Expr) -> Option<WidgetRef> {
        match unwrapped(expr) {
            Expr::Field(field) => {
                let Member::Named(name) = &field.member else {
                    return None;
                };
                self.is_widgets(&field.base)
                    .then(|| WidgetRef::Field(name.to_string()))
            },
            Expr::Path(path) => match self.held_name(path.path.get_ident()?) {
                Some(Held::Widget(widget)) => Some(widget.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether `expr` is every widget, behind references: `<x>.widgets`,
    /// `widgets` or a name that holds them.
    fn is_widgets(&self, expr: &Expr) -> bool {
        match unwrapped(expr) {
            Expr::Field(owner) => matches!(
                &owner.member,
                Member::Named(field) if field == WIDGETS_FIELD
            ),
            Expr::Path(owner) => owner.path.get_ident().is_some_and(|name| {
                name == WIDGETS_FIELD
                    || matches!(self.held_name(name), Some(Held::Widgets))
            }),
            _ => false,
        }
    }

    /// What the name `name` holds in the function the walk is in.
    fn held_name(&self, name: &syn::Ident) -> Option<&Held> {
        self.held
            .iter()
            .rev()
            .find(|(held, _)| name == held)
            .map(|(_, held)| held)
    }

    /// The widget of a `<path>::reduce(first, ..)` call: the widget that
    /// `first` is, or the `XWidget` type before `reduce`.
    fn widget_of_path_call(
        &self,
        path: &[String],
        first: Option<&Expr>,
    ) -> Option<WidgetRef> {
        if let Some(widget) = first.and_then(|first| self.widget_in(first)) {
            return Some(widget);
        }

        let owner = path.get(path.len().checked_sub(2)?)?;
        owner
            .ends_with(WIDGET_ENDING)
            .then(|| WidgetRef::Type(owner.clone()))
    }

    /// Walk a function body with the names its parameters hold.
    fn in_function(
        &mut self,
        signature: &Signature,
        walk: impl FnOnce(&mut Self),
    ) {
        let held = parameters_held(signature);
        let outer = std::mem::replace(&mut self.held, held);

        walk(self);

        self.held = outer;
    }
}

impl<'ast> Visit<'ast> for Calls {
    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        self.in_function(&function.sig, |calls| {
            visit::visit_item_fn(calls, function);
        });
    }

    fn visit_impl_item_fn(&mut self, function: &'ast ImplItemFn) {
        self.in_function(&function.sig, |calls| {
            visit::visit_impl_item_fn(calls, function);
        });
    }

    /// A `let` that binds a name to a widget, or to every widget, makes the
    /// name hold it; any other `let` of the name ends that.
    fn visit_local(&mut self, local: &'ast Local) {
        visit::visit_local(self, local);

        let Some((name, kind)) = binding(&local.pat) else {
            return;
        };
        let held = local
            .init
            .as_ref()
            .and_then(|init| self.held_by(&init.expr))
            .or_else(|| kind.and_then(held_by_type));

        self.held.retain(|(held, _)| *held != name);
        if let Some(held) = held {
            self.held.push((name, held));
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(callee) = call.func.as_ref()
            && let Some(last) = callee.path.segments.last()
        {
            let written = path_names(&callee.path);
            let resolved = expanded(&written, &self.aliases);
            let line = line_of(last.ident.span());

            match resolved.last().map(String::as_str) {
                Some("route") => self.routes.push((written.join("::"), line)),
                Some("reduce") if resolved.len() > 1 => {
                    if let Some(widget) =
                        self.widget_of_path_call(&resolved, call.args.first())
                    {
                        self.widget_reduces.push((widget, line));
                    }
                    self.path_reduces.push((
                        written.join("::"),
                        resolved,
                        line,
                    ));
                },
                _ => {},
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
            && let Some(widget) = self.widget_in(&call.receiver)
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
/// `reduce` calls of a child widget outside its parent's `reducer.rs`.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let widgets = widget_names(tree);
    let children = all_children(tree, &widgets);
    let mut findings = Vec::new();

    for file in tree.files() {
        let calls = Calls::of(file);

        if !ROUTE_CALLERS.contains(&file.path()) {
            findings.extend(route_calls(file, &calls));
        }
        findings.extend(reduce_calls(file, &calls, &widgets));
        findings.extend(child_reduce_calls(file, &calls, &children));
    }

    findings
}

/// The child widgets of every widget of `widgets`, as `(widget, child)`
/// pairs.
fn all_children(
    tree: &SourceTree,
    widgets: &[String],
) -> Vec<(String, String)> {
    widgets
        .iter()
        .flat_map(|widget| {
            child_names(tree, widget)
                .into_iter()
                .map(move |child| (widget.clone(), child))
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

/// Calls of a widget's `reduce` outside `router/<widget>.rs`.
fn reduce_calls(
    file: &SourceFile,
    calls: &Calls,
    widgets: &[String],
) -> Vec<Finding> {
    calls
        .widget_reduces
        .iter()
        .filter_map(|(widget, line)| Some((widget.widget(widgets)?, line)))
        .filter(|(widget, _)| file.path() != format!("router/{widget}.rs"))
        .map(|(widget, line)| {
            Finding::new(
                Rule::ReduceCalledOutsideRoute,
                file.path(),
                &widget,
                *line,
            )
        })
        .collect()
}

/// Calls `<child>::reduce(` of one of the `(widget, child)` pairs in
/// `children` anywhere but in the `reducer.rs` of the child's parent, also
/// when an import renames the child widget or its `reduce`.
fn child_reduce_calls(
    file: &SourceFile,
    calls: &Calls,
    children: &[(String, String)],
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for (written, resolved, line) in &calls.path_reduces {
        let name = &resolved[resolved.len() - 2];
        let is_child = children.iter().any(|(_, child)| child == name);
        let in_parent_reducer = children.iter().any(|(widget, child)| {
            child == name
                && file.path() == format!("widgets/{widget}/reducer.rs")
        });
        if !is_child || in_parent_reducer {
            continue;
        }

        findings.push(Finding::new(
            Rule::ChildReduceOutsideParent,
            file.path(),
            written,
            *line,
        ));
    }

    findings
}

/// What the parameters of `signature` hold, by their types.
fn parameters_held(signature: &Signature) -> Vec<(String, Held)> {
    signature
        .inputs
        .iter()
        .filter_map(|input| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let (name, _) = binding(&typed.pat)?;

            Some((name, held_by_type(&typed.ty)?))
        })
        .collect()
}

/// The name a pattern binds, with its type when it is written, for a
/// plain `name` or `name: Type`.
fn binding(pat: &Pat) -> Option<(String, Option<&Type>)> {
    match pat {
        Pat::Ident(binding) => Some((binding.ident.to_string(), None)),
        Pat::Type(typed) => match typed.pat.as_ref() {
            Pat::Ident(binding) => {
                Some((binding.ident.to_string(), Some(typed.ty.as_ref())))
            },
            _ => None,
        },
        _ => None,
    }
}

/// What a value of type `kind` holds, behind references: `Widgets` holds
/// every widget, an `XWidget` one widget.
fn held_by_type(kind: &Type) -> Option<Held> {
    match kind {
        Type::Reference(reference) => held_by_type(&reference.elem),
        Type::Paren(inner) => held_by_type(&inner.elem),
        Type::Path(path) => {
            let name = path.path.segments.last()?.ident.to_string();
            if name == WIDGETS_TYPE {
                return Some(Held::Widgets);
            }

            (name.len() > WIDGET_ENDING.len() && name.ends_with(WIDGET_ENDING))
                .then_some(Held::Widget(WidgetRef::Type(name)))
        },
        _ => None,
    }
}

/// `expr` without the references, dereferences and brackets around it.
fn unwrapped(expr: &Expr) -> &Expr {
    match expr {
        Expr::Reference(reference) => unwrapped(&reference.expr),
        Expr::Paren(inner) => unwrapped(&inner.expr),
        Expr::Unary(unary) if matches!(unary.op, UnOp::Deref(_)) => {
            unwrapped(&unary.expr)
        },
        _ => expr,
    }
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
    fn only_the_parent_reducer_calls_the_reduce_of_a_child_widget() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/mod.rs",
                "fn reduce() {\n    reducer::reduce(&mut self.state, intent)\n}\n",
            ),
            ("widgets/containers/child/mod.rs", "pub(crate) mod run;\n"),
            ("widgets/containers/child/run/mod.rs", ""),
            (
                "widgets/containers/reducer.rs",
                "use super::child::run;\n\
                 fn reduce() {\n    let (effects, outcome) = run::reduce(run, intent, &ctx);\n    \
                 let _ = child::run::reduce(run, intent, &ctx);\n}\n",
            ),
            (
                "widgets/containers/view/main_area.rs",
                "fn view() {\n    let _ = run::reduce(run, intent, &ctx);\n}\n",
            ),
            (
                "router/containers.rs",
                "fn route() {\n    let _ = containers::child::run::reduce(run, intent, &ctx);\n}\n",
            ),
            (
                "widgets/notes/reducer.rs",
                "fn reduce() {\n    let _ = run::reduce(run, intent, &ctx);\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ChildReduceOutsideParent),
            [
                pair("router/containers.rs", "containers::child::run::reduce"),
                pair("widgets/containers/view/main_area.rs", "run::reduce"),
                pair("widgets/notes/reducer.rs", "run::reduce"),
            ]
        );
    }

    #[test]
    fn a_folder_outside_child_has_no_reduce_of_a_child_widget() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            (
                "widgets/containers/view/main_area.rs",
                "fn view() {\n    let _ = run::reduce(run, intent, &ctx);\n}\n",
            ),
        ]);

        assert!(
            found(&check(&tree), Rule::ChildReduceOutsideParent).is_empty()
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

    #[test]
    fn a_reduce_through_a_local_a_parameter_or_the_widget_type_is_found() {
        let tree = SourceTree::from_sources(&[
            ("widgets/tabs/mod.rs", ""),
            ("widgets/quick_launch/mod.rs", ""),
            (
                "router/modes.rs",
                "fn route(app: &mut App) {\n    let tabs = &mut app.widgets.tabs;\n    tabs.reduce(intent);\n}\n",
            ),
            (
                "router/sidebar.rs",
                "fn route(app: &mut App) {\n    TabsWidget::reduce(&mut app.widgets.tabs, intent);\n}\n",
            ),
            (
                "router/chrome.rs",
                "fn helper(launches: &mut QuickLaunchWidget, all: &mut Widgets) {\n    \
                 launches.reduce(intent, &ctx);\n    let w = &mut *all;\n    w.tabs.reduce(intent);\n}\n",
            ),
            (
                "router/palette.rs",
                "fn route(app: &mut App) {\n    let all = &mut app.widgets;\n    \
                 let tabs = other();\n    tabs.reduce(intent);\n    \
                 let _ = OtherWidget::reduce(x);\n}\n\
                 fn later(tabs: Tabs) {\n    tabs.reduce(intent);\n    all.tabs.reduce(intent);\n}\n",
            ),
            (
                "router/tabs.rs",
                "fn route(app: &mut App) {\n    let tabs = &mut app.widgets.tabs;\n    \
                 tabs.reduce(intent);\n    TabsWidget::reduce(tabs, intent);\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ReduceCalledOutsideRoute),
            [
                pair("router/chrome.rs", "quick_launch"),
                pair("router/chrome.rs", "tabs"),
                pair("router/modes.rs", "tabs"),
                pair("router/sidebar.rs", "tabs"),
            ]
        );
    }

    #[test]
    fn a_child_reduce_or_route_under_another_name_is_found() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/mod.rs",
                "use child::run::reduce as run_reduce;\nfn f() {\n    run_reduce(run, intent, &ctx);\n}\n",
            ),
            ("widgets/containers/child/run/mod.rs", ""),
            (
                "widgets/containers/reducer.rs",
                "use super::child::run::reduce as reduce_child;\nuse super::child::run as form;\n\
                 fn r() {\n    reduce_child(run, intent, &ctx);\n    form::reduce(run, intent, &ctx);\n}\n",
            ),
            (
                "widgets/containers/view/main_area.rs",
                "use super::super::child::run as form;\nfn v() {\n    form::reduce(run, intent, &ctx);\n}\n",
            ),
            (
                "router/palette.rs",
                "use super::command::route as run_command;\nfn f() {\n    run_command(app, c);\n}\n",
            ),
        ]);

        let findings = check(&tree);

        assert_eq!(
            found(&findings, Rule::ChildReduceOutsideParent),
            [
                pair("widgets/containers/mod.rs", "run_reduce"),
                pair("widgets/containers/view/main_area.rs", "form::reduce"),
            ]
        );
        assert_eq!(
            found(&findings, Rule::RouteCalledDirectly),
            [pair("router/palette.rs", "run_command")]
        );
    }
}
