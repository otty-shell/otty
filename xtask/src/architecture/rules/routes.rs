//! The route rule: a route chooses what to do only by the variant of its
//! intent or effect, never by widget state. When the data an effect needs is
//! missing, sending nothing is allowed: that is no decision. A helper in
//! `router/` that takes a widget, every widget or a widget state instead of
//! the app is checked the same way.

use syn::visit::{self, Visit};
use syn::{
    Arm, BinOp, Block, Expr, ExprIf, ExprMatch, ExprMethodCall, FnArg, Item,
    ItemFn, Local, Macro, Pat, Stmt, Type,
};

use super::super::finding::{Finding, Rule};
use super::super::source::SourceTree;
use super::super::syntax::{
    expr_idents, line_of, macro_exprs, pat_idents, path_names,
};

/// The parameter through which a route reads widget state.
const APP: &str = "app";

/// Types of parameters that carry widget state, by their full name.
const STATE_TYPES: &[&str] = &["App", "Widgets"];

/// Name endings of the types of parameters that carry widget state: one
/// widget or one widget state. `State` alone is the window state of the
/// app, not a widget's.
const STATE_TYPE_ENDINGS: &[&str] = &["Widget", "State"];

/// The method of `bool` that chooses a value from a closure, `then(|| ..)`.
const BOOL_THEN: &str = "then";

/// The method of `bool` that chooses a value, `then_some(..)`.
const BOOL_THEN_SOME: &str = "then_some";

/// What a condition of an `if` reads.
enum Condition {
    /// A part other than a `let` reads widget state: a decision.
    ReadsState,
    /// Only `let` parts read widget state, to bind it.
    BindsState,
    /// Nothing reads widget state.
    Free,
}

/// Walks the body of a route function in the order of the code and stops
/// at the first branch on widget state.
///
/// The parameters that carry widget state, such as `app`, and the names
/// bound by `let` from an expression that names one of them, or another
/// such name, carry widget state. A branch whose only job is to skip work
/// when such data is missing is allowed: `let … else { return Task::none() }`,
/// `if let` without a working `else`, and a `match` with an empty `None` or
/// `Err` arm.
struct StateBranch {
    /// `app` and the names bound from widget state so far.
    state_names: Vec<String>,
    /// The line of the first branch on widget state.
    line: Option<usize>,
}

impl StateBranch {
    /// Whether `expr` names one of the state names.
    fn reads_state(&self, expr: &Expr) -> bool {
        expr_idents(expr)
            .iter()
            .any(|ident| self.state_names.iter().any(|name| ident == name))
    }

    /// Track the names a `let` binds from state; a `let … else` whose
    /// `else` does more than send nothing is a decision.
    fn decides_in_let(&mut self, local: &Local) -> bool {
        let Some(init) = &local.init else {
            return false;
        };
        if !self.reads_state(&init.expr) {
            return false;
        }

        self.state_names.extend(bound_names(&local.pat));
        init.diverge
            .as_ref()
            .is_some_and(|(_, otherwise)| !is_nothing(otherwise))
    }

    /// An `if` decides when a condition reads state. `if let` that only
    /// binds state data is fine unless its `else` does work.
    fn decides_in_if(&mut self, branch: &ExprIf) -> bool {
        match self.condition(&branch.cond) {
            Condition::ReadsState => true,
            Condition::BindsState => branch
                .else_branch
                .as_ref()
                .is_some_and(|(_, otherwise)| is_working_else(otherwise)),
            Condition::Free => false,
        }
    }

    /// A `match` decides when its scrutinee reads state, unless an empty
    /// `None` or `Err` arm shows it only unpacks data that may be missing.
    fn decides_in_match(&self, branch: &ExprMatch) -> bool {
        self.reads_state(&branch.expr)
            && !branch.arms.iter().any(is_empty_missing_arm)
    }

    /// Read the `&&` parts of `cond` from left to right: a `let` part that
    /// reads state binds its names, any other part that reads state decides.
    fn condition(&mut self, cond: &Expr) -> Condition {
        let mut condition = Condition::Free;

        for part in and_parts(cond) {
            if let Expr::Let(binding) = part {
                if self.reads_state(&binding.expr) {
                    condition = Condition::BindsState;
                    self.state_names.extend(bound_names(&binding.pat));
                }
                continue;
            }
            if self.reads_state(part) {
                return Condition::ReadsState;
            }
        }

        condition
    }
}

impl<'ast> Visit<'ast> for StateBranch {
    fn visit_local(&mut self, local: &'ast Local) {
        if self.line.is_some() {
            return;
        }
        if self.decides_in_let(local) {
            self.line = Some(line_of(local.let_token.span));
            return;
        }

        visit::visit_local(self, local);
    }

    fn visit_expr_if(&mut self, branch: &'ast ExprIf) {
        if self.line.is_some() {
            return;
        }
        if self.decides_in_if(branch) {
            self.line = Some(line_of(branch.if_token.span));
            return;
        }

        visit::visit_expr_if(self, branch);
    }

    fn visit_expr_match(&mut self, branch: &'ast ExprMatch) {
        if self.line.is_some() {
            return;
        }
        if self.decides_in_match(branch) {
            self.line = Some(line_of(branch.match_token.span));
            return;
        }

        visit::visit_expr_match(self, branch);
    }

    /// `bool::then(|| ..)` or `then_some(..)` on a value read from state
    /// chooses what to send, as an `if` does. `then` with a closure that
    /// takes a value, or with a named function, is `Task::then`.
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if self.line.is_some() {
            return;
        }
        let chooses = call.method == BOOL_THEN_SOME
            || (call.method == BOOL_THEN
                && matches!(
                    call.args.first(),
                    Some(Expr::Closure(closure)) if closure.inputs.is_empty()
                ));
        if chooses && self.reads_state(&call.receiver) {
            self.line = Some(line_of(call.method.span()));
            return;
        }

        visit::visit_expr_method_call(self, call);
    }

    /// A match guard that reads state decides, as an `if` does.
    fn visit_arm(&mut self, arm: &'ast Arm) {
        if self.line.is_some() {
            return;
        }
        if let Some((if_token, guard)) = &arm.guard
            && matches!(self.condition(guard), Condition::ReadsState)
        {
            self.line = Some(line_of(if_token.span));
            return;
        }

        visit::visit_arm(self, arm);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        for expr in macro_exprs(mac) {
            self.visit_expr(&expr);
        }
    }
}

/// Report functions in `router/` that read widget state and branch on what
/// they read.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        if !file.path().starts_with("router/") {
            continue;
        }
        for item in &file.syntax().items {
            let Item::Fn(function) = item else {
                continue;
            };
            let sources = state_parameters(function);
            if sources.is_empty() {
                continue;
            }

            if let Some(line) = first_state_branch(&function.block, sources) {
                findings.push(Finding::new(
                    Rule::RouteBranchesOnState,
                    file.path(),
                    &function.sig.ident.to_string(),
                    line,
                ));
            }
        }
    }

    findings
}

/// The names of the parameters of `function` that carry widget state: one
/// named `app`, or one whose type is the app, every widget, one widget or
/// a widget state.
fn state_parameters(function: &ItemFn) -> Vec<String> {
    function
        .sig
        .inputs
        .iter()
        .filter_map(|input| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let Pat::Ident(binding) = typed.pat.as_ref() else {
                return None;
            };

            (binding.ident == APP || carries_state(&typed.ty))
                .then(|| binding.ident.to_string())
        })
        .collect()
}

/// Whether a value of type `kind`, behind references, carries widget
/// state.
fn carries_state(kind: &Type) -> bool {
    match kind {
        Type::Reference(reference) => carries_state(&reference.elem),
        Type::Path(path) => path.path.segments.last().is_some_and(|last| {
            let name = last.ident.to_string();
            STATE_TYPES.contains(&name.as_str())
                || STATE_TYPE_ENDINGS.iter().any(|ending| {
                    name.len() > ending.len() && name.ends_with(ending)
                })
        }),
        _ => false,
    }
}

/// The line of the first branch on widget state in `body`, which reads
/// state through the parameters `sources`, if any.
fn first_state_branch(body: &Block, sources: Vec<String>) -> Option<usize> {
    let mut walker = StateBranch {
        state_names: sources,
        line: None,
    };
    walker.visit_block(body);

    walker.line
}

/// The parts of `cond` joined by `&&`, from left to right.
fn and_parts(cond: &Expr) -> Vec<&Expr> {
    match cond {
        Expr::Binary(binary) if matches!(binary.op, BinOp::And(_)) => {
            let mut parts = and_parts(&binary.left);
            parts.extend(and_parts(&binary.right));
            parts
        },
        _ => vec![cond],
    }
}

/// The variable names a pattern binds, e.g. `cwd` in `Some(cwd)`: its
/// names that start with a lowercase letter, without a type ascription.
fn bound_names(pat: &Pat) -> Vec<String> {
    let pat = match pat {
        Pat::Type(typed) => typed.pat.as_ref(),
        _ => pat,
    };

    pat_idents(pat)
        .iter()
        .map(ToString::to_string)
        .filter(|name| name.starts_with(|ch: char| ch.is_ascii_lowercase()))
        .collect()
}

/// Whether the `else` of an `if` does more than send nothing; an
/// `else if` always does.
fn is_working_else(otherwise: &Expr) -> bool {
    match otherwise {
        Expr::If(_) => true,
        _ => !is_nothing(otherwise),
    }
}

/// A `None` or `Err(..)` arm without a guard that sends nothing.
fn is_empty_missing_arm(arm: &Arm) -> bool {
    let is_missing = match &arm.pat {
        Pat::Ident(binding) => binding.ident == "None",
        Pat::TupleStruct(variant) => variant.path.is_ident("Err"),
        _ => false,
    };

    is_missing && arm.guard.is_none() && is_nothing(&arm.body)
}

/// Whether `expr` sends nothing: `Task::none()`, `None`, `()`, an empty
/// block, a bare `return`, `continue` or `break`, or a block or `return`
/// of one of these.
fn is_nothing(expr: &Expr) -> bool {
    match expr {
        Expr::Block(block) => is_nothing_block(&block.block),
        Expr::Return(value) => value.expr.as_deref().is_none_or(is_nothing),
        Expr::Path(path) => path.path.is_ident("None"),
        Expr::Tuple(tuple) => tuple.elems.is_empty(),
        Expr::Continue(next) => next.label.is_none(),
        Expr::Break(stop) => stop.label.is_none() && stop.expr.is_none(),
        Expr::Call(call) => {
            call.args.is_empty()
                && matches!(call.func.as_ref(), Expr::Path(callee)
                    if path_names(&callee.path) == ["Task", "none"])
        },
        _ => false,
    }
}

/// Whether `block` is empty or holds one statement that sends nothing.
fn is_nothing_block(block: &Block) -> bool {
    match block.stmts.as_slice() {
        [] => true,
        [Stmt::Expr(expr, _)] => is_nothing(expr),
        _ => false,
    }
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

    #[test]
    fn a_match_guard_or_a_macro_that_reads_state_is_reported() {
        let tree = SourceTree::from_sources(&[(
            "router/notes.rs",
            "fn guard(app: &App, event: NotesEvent) -> Task<AppEvent> {\n    \
             match event {\n        NotesEvent::Intent(intent) if app.widgets.notes.is_open() => one(intent),\n        \
             _ => Task::none(),\n    }\n}\n\
             fn wrapped(app: &App) -> Task<AppEvent> {\n    \
             Task::batch(vec![if app.widgets.notes.is_open() { one() } else { two() }])\n}\n\
             fn one_line(app: &App) -> Task<AppEvent> { one() }\n\
             fn after(app: &App) -> Task<AppEvent> {\n    Task::none()\n}\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::RouteBranchesOnState),
            [
                pair("router/notes.rs", "guard"),
                pair("router/notes.rs", "wrapped"),
            ]
        );
    }

    #[test]
    fn a_working_let_else_or_else_if_on_state_is_reported() {
        let tree = SourceTree::from_sources(&[(
            "router/notes.rs",
            "fn let_else(app: &App) -> Task<AppEvent> {\n    \
             let Some(x) = app.widgets.notes.active() else {\n        log();\n        return Task::none();\n    };\n    \
             one(x)\n}\n\
             fn typed(app: &App) -> Task<AppEvent> {\n    \
             let open: bool = app.widgets.notes.is_open();\n    if open {\n        return one();\n    }\n    \
             Task::none()\n}\n\
             fn else_if(app: &App, other: bool) -> Task<AppEvent> {\n    \
             if let Some(x) = app.widgets.notes.active() {\n        one(x)\n    } else if other {\n        two()\n    } else {\n        Task::none()\n    }\n}\n\
             fn skips(app: &App) -> Option<u8> {\n    \
             let x = app.widgets.notes.count();\n    \
             match app.widgets.notes.active() {\n        Some(x) => one(x),\n        None => None,\n    }\n}\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::RouteBranchesOnState),
            [
                pair("router/notes.rs", "else_if"),
                pair("router/notes.rs", "let_else"),
                pair("router/notes.rs", "typed"),
            ]
        );
    }

    #[test]
    fn a_helper_that_takes_a_widget_or_its_state_is_checked_like_a_route() {
        let code = "fn pick(tabs: &TabsWidget) -> Task<AppEvent> {\n    \
            if tabs.is_empty() {\n        Task::done(A)\n    } else {\n        Task::done(B)\n    }\n}\n\
            fn all(widgets: &mut Widgets) -> Task<AppEvent> {\n    \
            match widgets.modes.active() {\n        Mode::Terminal => one(),\n        _ => two(),\n    }\n}\n\
            fn other(shell: &mut App) -> Task<AppEvent> {\n    \
            if shell.widgets.tabs.is_open() { one() } else { two() }\n}\n\
            fn part(run: &RunState) -> Task<AppEvent> {\n    \
            if run.is_open() { one() } else { two() }\n}\n\
            fn window(state: &State) -> Task<AppEvent> {\n    \
            if state.is_wide() { one() } else { two() }\n}\n\
            fn props(tabs: &TabsViewModel<'_>) -> Task<AppEvent> {\n    \
            if tabs.groups.is_empty() { one() } else { two() }\n}\n\
            fn missing(tabs: &TabsWidget) -> Task<AppEvent> {\n    \
            let Some(id) = tabs.active() else {\n        return Task::none();\n    };\n    show(id)\n}\n";

        assert_eq!(
            route_findings(code),
            [
                pair("router/notes.rs", "all"),
                pair("router/notes.rs", "other"),
                pair("router/notes.rs", "part"),
                pair("router/notes.rs", "pick"),
            ]
        );
    }

    #[test]
    fn then_on_a_value_read_from_state_is_a_branch() {
        let code = "fn open(app: &App) -> Task<AppEvent> {\n    \
            app.widgets\n        .palette\n        .is_open()\n        \
            .then(|| Task::done(A))\n        .unwrap_or_else(|| Task::done(B))\n}\n\
            fn some(app: &App) -> Option<Task<AppEvent>> {\n    \
            let open = app.widgets.palette.is_open();\n    open.then_some(Task::done(A))\n}\n\
            fn chain(app: &mut App, intent: PaletteIntent) -> Task<AppEvent> {\n    \
            app.widgets.palette.reduce(intent).map(AppEvent::Palette).then(|event| Task::done(event))\n}\n\
            fn named(app: &App, pinned: Vec<Mode>) -> Task<AppEvent> {\n    \
            save(app.widgets.modes.pinned()).then(pinned_saved)\n}\n\
            fn free(app: &App, flag: bool) -> Option<u8> {\n    flag.then(|| 1)\n}\n";

        assert_eq!(
            route_findings(code),
            [
                pair("router/notes.rs", "open"),
                pair("router/notes.rs", "some")
            ]
        );
    }
}
