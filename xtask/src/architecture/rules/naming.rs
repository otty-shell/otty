//! Rules about names and shapes the widget template fixes: the reducer
//! parameter, the intent and effect parameters of routes, how view models
//! are built and the props of a view.

use syn::visit::{self, Visit};
use syn::{
    Block, ExprCall, ExprMethodCall, FnArg, ImplItemFn, ItemFn, Macro, Pat,
    Signature, Type,
};

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    Layer, camel_case, is_model_file, is_view_file, layer_of,
};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{TypeKind, declared_types, line_of, macro_exprs};

/// Name ending of the props of a view.
const PROPS_ENDING: &str = "Props";

/// The name a route parameter must have for each type name ending.
const ROUTE_PARAMETER_NAMES: &[(&str, &str)] =
    &[("Intent", "intent"), ("Effect", "effect")];

/// Collects the signatures of a file's functions, at any depth.
#[derive(Default)]
struct Signatures<'ast> {
    found: Vec<&'ast Signature>,
}

impl<'ast> Visit<'ast> for Signatures<'ast> {
    fn visit_signature(&mut self, signature: &'ast Signature) {
        self.found.push(signature);

        visit::visit_signature(self, signature);
    }
}

/// Collects the functions with a body, at any depth, as
/// `(signature, body)`.
#[derive(Default)]
struct Bodies<'ast> {
    found: Vec<(&'ast Signature, &'ast Block)>,
}

impl<'ast> Visit<'ast> for Bodies<'ast> {
    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        self.found.push((&function.sig, &function.block));

        visit::visit_item_fn(self, function);
    }

    fn visit_impl_item_fn(&mut self, function: &'ast ImplItemFn) {
        self.found.push((&function.sig, &function.block));

        visit::visit_impl_item_fn(self, function);
    }
}

/// Finds a call of `from_state`, as a function or as a method.
#[derive(Default)]
struct FromStateCall {
    is_found: bool,
}

impl<'ast> Visit<'ast> for FromStateCall {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let syn::Expr::Path(callee) = call.func.as_ref()
            && callee
                .path
                .segments
                .last()
                .is_some_and(|last| last.ident == "from_state")
        {
            self.is_found = true;
        }

        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if call.method == "from_state" {
            self.is_found = true;
        }

        visit::visit_expr_method_call(self, call);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        for expr in macro_exprs(mac) {
            self.visit_expr(&expr);
        }
    }
}

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
        if let Some(view) = view_name(file, &widget) {
            findings.extend(props_not_named_after_view(file, &view));
        }
        match file.name() {
            "reducer.rs" => findings.extend(reduce_parameter(file)),
            // The `mod.rs` of a `model/` folder is a model file, not the
            // `mod.rs` that holds `XWidget`.
            "mod.rs" if !is_model_file(file.path(), &widget) => {
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
    let mut findings = Vec::new();

    for signature in signatures(file) {
        if signature.ident != "reduce" {
            continue;
        }

        for (name, kind) in parameters(signature) {
            let is_intent = plain_type_name(kind)
                .is_some_and(|type_name| type_name.ends_with("Intent"));
            if is_intent && name != "intent" {
                findings.push(Finding::new(
                    Rule::ReduceParameterName,
                    file.path(),
                    &name,
                    line_of(signature.ident.span()),
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
    let mut findings = Vec::new();

    for signature in signatures(file) {
        let function = signature.ident.to_string();

        for (name, kind) in parameters(signature) {
            let Some(expected) = expected_route_name(kind) else {
                continue;
            };
            if name != expected {
                let key = format!("{function}({name})");
                findings.push(Finding::new(
                    Rule::RouteParameterName,
                    file.path(),
                    &key,
                    line_of(signature.ident.span()),
                ));
            }
        }
    }

    findings
}

/// `intent` or `effect` when `kind` is a path type, behind any references,
/// whose name ends in `Intent` or `Effect`; `None` for any other type, such
/// as `Vec<XIntent>` or `fn() -> XIntent`.
fn expected_route_name(kind: &Type) -> Option<&'static str> {
    let type_name = path_type_segment(kind)?.ident.to_string();

    ROUTE_PARAMETER_NAMES
        .iter()
        .find(|(ending, _)| type_name.ends_with(ending))
        .map(|(_, name)| *name)
}

/// View model accessors (`vm` and `<child>_vm`) whose body does not call
/// `from_state`.
fn view_models_not_from_state(file: &SourceFile) -> Vec<Finding> {
    let mut bodies = Bodies::default();
    bodies.visit_file(file.syntax());

    let mut findings = Vec::new();
    for (signature, body) in bodies.found {
        let name = signature.ident.to_string();
        if name != "vm" && !name.ends_with("_vm") {
            continue;
        }

        let mut call = FromStateCall::default();
        call.visit_block(body);
        if !call.is_found {
            findings.push(Finding::new(
                Rule::ViewModelNotFromState,
                file.path(),
                &name,
                line_of(signature.ident.span()),
            ));
        }
    }

    findings
}

/// Props structs in the file of the view `view` that are not named
/// `<View>Props`.
fn props_not_named_after_view(file: &SourceFile, view: &str) -> Vec<Finding> {
    let expected = format!("{view}{PROPS_ENDING}");

    declared_types(file.syntax())
        .iter()
        .filter(|declared| {
            declared.kind() == TypeKind::Struct
                && declared.name().ends_with(PROPS_ENDING)
                && declared.name() != expected
        })
        .map(|declared| {
            Finding::new(
                Rule::ViewPropsName,
                file.path(),
                declared.name(),
                declared.line(),
            )
        })
        .collect()
}

/// The name of the view that `file` holds, in CamelCase, e.g. `RunPanel` for
/// `widgets/containers/child/run/view/run_panel.rs`; `None` for a file that
/// is not one view of `widget` or of a child widget, such as `view/mod.rs`.
fn view_name(file: &SourceFile, widget: &str) -> Option<String> {
    let name = file.name();
    if !is_view_file(file.path(), widget)
        || name == "mod.rs"
        || name == "view.rs"
    {
        return None;
    }

    name.strip_suffix(".rs").map(camel_case)
}

/// The signatures of every function of `file`, at any depth.
fn signatures(file: &SourceFile) -> Vec<&Signature> {
    let mut signatures = Signatures::default();
    signatures.visit_file(file.syntax());

    signatures.found
}

/// The typed parameters of `signature` whose pattern is a name or `_`, as
/// `(name, type)`.
fn parameters(signature: &Signature) -> Vec<(String, &Type)> {
    signature
        .inputs
        .iter()
        .filter_map(|input| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let name = match typed.pat.as_ref() {
                Pat::Ident(binding) => binding.ident.to_string(),
                Pat::Wild(_) => "_".to_string(),
                _ => return None,
            };

            Some((name, typed.ty.as_ref()))
        })
        .collect()
}

/// The name of a path type without generic arguments, behind any
/// references, e.g. `TabsIntent` for `&super::event::TabsIntent`.
fn plain_type_name(kind: &Type) -> Option<String> {
    let last = path_type_segment(kind)?;

    last.arguments.is_empty().then(|| last.ident.to_string())
}

/// The last segment of a path type, behind any references.
fn path_type_segment(kind: &Type) -> Option<&syn::PathSegment> {
    match kind {
        Type::Reference(reference) => path_type_segment(&reference.elem),
        Type::Path(path) if path.qself.is_none() => path.path.segments.last(),
        _ => None,
    }
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
            ("widgets/containers/child/run/mod.rs", ""),
            (
                "widgets/containers/child/run/view/run_panel.rs",
                "pub(crate) struct RunPanelProps<'a> {\n}\nstruct ImagePanelProps {\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ViewPropsName),
            [
                pair(
                    "widgets/containers/child/run/view/run_panel.rs",
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

    #[test]
    fn from_state_may_be_called_as_a_method_or_inside_a_macro() {
        let tree = SourceTree::from_sources(&[(
            "widgets/notes/mod.rs",
            "fn list_vm(state: &NotesState) -> Vec<RowViewModel> {\n    \
             vec![RowViewModel::from_state(state)]\n}\n\
             fn panel_vm(state: &NotesState) -> PanelViewModel {\n    state.from_state()\n}\n\
             fn tree_vm(state: &NotesState) -> TreeViewModel {\n    TreeViewModel::new(state)\n}\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::ViewModelNotFromState),
            [pair("widgets/notes/mod.rs", "tree_vm")]
        );
    }

    #[test]
    fn an_unnamed_route_parameter_is_reported_as_an_underscore() {
        let tree = SourceTree::from_sources(&[(
            "router/notes.rs",
            "fn skip(_: NotesIntent, (a, b): (NotesEffect, u8)) {}\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::RouteParameterName),
            [pair("router/notes.rs", "skip(_)")]
        );
    }
}
