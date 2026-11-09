//! Rules about the `XWidget` type: only its `reduce` changes the state, and
//! a view never names it.

use syn::{ImplItem, Item, Signature, Type};

use super::super::finding::{Finding, Rule};
use super::super::layout::{Layer, is_view_file, layer_of, widget_type_name};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{file_idents, line_of};

/// The one method of `XWidget` that changes the state.
const REDUCE: &str = "reduce";

/// Report `XWidget` methods other than `reduce` that take `&mut self`, and
/// views that name their `XWidget`.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        let Some(Layer::Widget(widget)) = layer_of(file.path()) else {
            continue;
        };
        let type_name = widget_type_name(&widget);

        if file.path() == format!("widgets/{widget}/mod.rs") {
            findings.extend(mutating_methods(file, &type_name));
        }
        if is_view_file(tree, file.path(), &widget) {
            findings.extend(widget_named_in_view(file, &type_name));
        }
    }

    findings
}

/// The methods of `impl <type_name>` blocks in `file`, other than
/// `reduce`, that take `&mut self`.
fn mutating_methods(file: &SourceFile, type_name: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for item in &file.syntax().items {
        let Item::Impl(block) = item else {
            continue;
        };
        if !is_named(&block.self_ty, type_name) {
            continue;
        }

        for inner in &block.items {
            let ImplItem::Fn(method) = inner else {
                continue;
            };
            if method.sig.ident == REDUCE || !takes_self_mutably(&method.sig) {
                continue;
            }

            findings.push(Finding::new(
                Rule::WidgetMutatesOutsideReduce,
                file.path(),
                &format!("{type_name}::{}", method.sig.ident),
                line_of(method.sig.ident.span()),
            ));
        }
    }

    findings
}

/// The first place where a view file names `type_name`.
fn widget_named_in_view(file: &SourceFile, type_name: &str) -> Option<Finding> {
    let ident = file_idents(file.syntax())
        .into_iter()
        .find(|ident| ident == type_name)?;

    Some(Finding::new(
        Rule::ViewNamesWidget,
        file.path(),
        type_name,
        line_of(ident.span()),
    ))
}

/// Whether `kind` is the path type `name`, written with or without its
/// module path.
fn is_named(kind: &Type, name: &str) -> bool {
    match kind {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|last| last.ident == name),
        _ => false,
    }
}

/// Whether the receiver of `signature` is `&mut self` or `self: &mut Self`.
fn takes_self_mutably(signature: &Signature) -> bool {
    signature.receiver().is_some_and(|receiver| {
        matches!(
            receiver.ty.as_ref(),
            Type::Reference(reference) if reference.mutability.is_some()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    #[test]
    fn only_reduce_takes_the_widget_mutably() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/sidebar/mod.rs",
                "impl SidebarWidget {\n    \
                 pub(crate) fn reduce(&mut self, intent: SidebarIntent) -> Task<SidebarEvent> {\n        \
                 reducer::reduce(&mut self.state, intent)\n    }\n\n    \
                 pub(crate) fn reset(&mut self) {\n        self.state = SidebarState::default();\n    }\n\n    \
                 pub(crate) fn resize(self: &mut Self, ratio: f32) {\n    }\n\n    \
                 pub(crate) fn is_open(&self) -> bool {\n        self.state.is_open()\n    }\n\n    \
                 pub(crate) fn load() -> Self {\n        Self::default()\n    }\n}\n\
                 impl Other {\n    fn touch(&mut self) {}\n}\n",
            ),
            (
                "widgets/containers/run/mod.rs",
                "impl RunPanel {\n    fn clear(&mut self) {}\n}\n",
            ),
            (
                "widgets/sidebar/state.rs",
                "impl SidebarState {\n    pub(super) fn reset(&mut self) {}\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::WidgetMutatesOutsideReduce),
            [
                pair("widgets/sidebar/mod.rs", "SidebarWidget::reset"),
                pair("widgets/sidebar/mod.rs", "SidebarWidget::resize"),
            ]
        );
    }

    #[test]
    fn a_view_never_names_its_widget() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            (
                "widgets/modes/view/mode_switcher.rs",
                "use super::super::ModesWidget;\n\
                 fn view(widget: &ModesWidget) -> Element<'_, ModesIntent> {\n}\n",
            ),
            (
                "widgets/containers/run/view/run_form.rs",
                "fn view(widget: &crate::widgets::containers::ContainersWidget) {\n}\n",
            ),
            (
                "widgets/quick_launch/view.rs",
                "fn view(props: SidebarPanelProps<'_>) {\n    let _ = column![QuickLaunchWidget::new()];\n}\n",
            ),
            (
                "widgets/modes/model.rs",
                "fn build(widget: &ModesWidget) {\n}\n",
            ),
            (
                "widgets/modes/view/mode_menu.rs",
                "fn view(props: ModeMenuProps<'_>, w: &TabsWidget) {\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ViewNamesWidget),
            [
                pair(
                    "widgets/containers/run/view/run_form.rs",
                    "ContainersWidget"
                ),
                pair("widgets/modes/view/mode_switcher.rs", "ModesWidget"),
                pair("widgets/quick_launch/view.rs", "QuickLaunchWidget"),
            ]
        );
    }
}
