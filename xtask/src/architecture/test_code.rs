//! Leaves test code out of a parsed source file: every item under
//! `#[cfg(test)]`, also when `test` is one of the conditions of
//! `#[cfg(all(..))]`.

use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{
    Attribute, Block, File, ImplItem, ItemImpl, ItemMod, Meta, Stmt, Token,
};

use super::syntax::item_attrs;

/// Removes the test-only items of a file, at any depth.
struct TestCodeRemover;

impl VisitMut for TestCodeRemover {
    fn visit_file_mut(&mut self, file: &mut File) {
        file.items.retain(|item| !is_test_only(item_attrs(item)));

        visit_mut::visit_file_mut(self, file);
    }

    fn visit_item_mod_mut(&mut self, module: &mut ItemMod) {
        if let Some((_, items)) = &mut module.content {
            items.retain(|item| !is_test_only(item_attrs(item)));
        }

        visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_item_impl_mut(&mut self, item: &mut ItemImpl) {
        item.items
            .retain(|inner| !is_test_only(impl_item_attrs(inner)));

        visit_mut::visit_item_impl_mut(self, item);
    }

    fn visit_block_mut(&mut self, block: &mut Block) {
        block.stmts.retain(|stmt| match stmt {
            Stmt::Item(item) => !is_test_only(item_attrs(item)),
            _ => true,
        });

        visit_mut::visit_block_mut(self, block);
    }
}

/// Remove every test-only item from `file`.
pub(crate) fn remove_test_code(file: &mut File) {
    TestCodeRemover.visit_file_mut(file);
}

/// Whether `attrs` hold a `cfg` that is true only in tests.
fn is_test_only(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .filter_map(|attr| attr.parse_args::<Meta>().ok())
        .any(|condition| is_test_condition(&condition))
}

/// `test`, or `all(..)` with `test` among its conditions.
fn is_test_condition(condition: &Meta) -> bool {
    match condition {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) if list.path.is_ident("all") => list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .is_ok_and(|inner| inner.iter().any(is_test_condition)),
        _ => false,
    }
}

fn impl_item_attrs(item: &ImplItem) -> &[Attribute] {
    match item {
        ImplItem::Const(item) => &item.attrs,
        ImplItem::Fn(item) => &item.attrs,
        ImplItem::Type(item) => &item.attrs,
        ImplItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collects the names of the functions it visits.
    #[derive(Default)]
    struct FunctionNames(Vec<String>);

    impl<'ast> syn::visit::Visit<'ast> for FunctionNames {
        fn visit_signature(&mut self, signature: &'ast syn::Signature) {
            self.0.push(signature.ident.to_string());
        }
    }

    /// The names of the functions left in `code` once test code is out.
    fn kept_functions(code: &str) -> Vec<String> {
        let mut file = syn::parse_file(code).expect("the test code parses");
        remove_test_code(&mut file);

        let mut names = FunctionNames::default();
        syn::visit::Visit::visit_file(&mut names, &file);
        names.0
    }

    #[test]
    fn a_test_module_and_its_other_attributes_are_left_out() {
        let kept = kept_functions(
            "fn reduce() {}\n\
             #[cfg(test)]\n\
             #[expect(clippy::disallowed_methods, reason = \"x\")]\n\
             mod tests {\n    fn helper() {}\n}\n\
             fn after() {}\n",
        );

        assert_eq!(kept, ["reduce", "after"]);
    }

    #[test]
    fn a_test_only_method_inside_an_impl_is_left_out() {
        let kept = kept_functions(
            "impl NotesState {\n    #[cfg(test)]\n    fn text(&self) {}\n    fn kept(&self) {}\n}\n",
        );

        assert_eq!(kept, ["kept"]);
    }

    #[test]
    fn test_items_inside_modules_and_bodies_are_left_out() {
        let kept = kept_functions(
            "mod inner {\n    #[cfg(test)]\n    fn a() {}\n    fn b() {}\n}\n\
             fn outer() {\n    #[cfg(test)]\n    fn c() {}\n    fn d() {}\n}\n",
        );

        assert_eq!(kept, ["b", "outer", "d"]);
    }

    #[test]
    fn every_kind_of_item_and_impl_item_under_cfg_test_is_left_out() {
        let mut file = syn::parse_file(
            "#[cfg(test)]\nconst A: u8 = 1;\n#[cfg(test)]\nenum B {}\n\
             #[cfg(test)]\nextern crate c;\n#[cfg(test)]\nextern \"C\" {}\n\
             #[cfg(test)]\nimpl B {}\n#[cfg(test)]\nm!();\n#[cfg(test)]\nstatic D: u8 = 1;\n\
             #[cfg(test)]\nstruct E;\n#[cfg(test)]\ntrait F {}\n#[cfg(test)]\ntrait G = F;\n\
             #[cfg(test)]\ntype H = u8;\n#[cfg(test)]\nunion I { a: u8 }\n#[cfg(test)]\nuse j::K;\n\
             impl L {\n    #[cfg(test)]\n    const M: u8 = 1;\n    #[cfg(test)]\n    type N = u8;\n    \
             #[cfg(test)]\n    m!();\n    const KEPT: u8 = 1;\n}\n",
        )
        .expect("the test code parses");

        remove_test_code(&mut file);

        let [syn::Item::Impl(kept)] = file.items.as_slice() else {
            panic!("only `impl L` is left: {}", file.items.len());
        };
        assert_eq!(kept.items.len(), 1);
    }

    #[test]
    fn test_inside_all_is_test_only_but_not_inside_not_or_any() {
        let kept = kept_functions(
            "#[cfg(all(test, unix))]\nfn a() {}\n\
             #[cfg(not(test))]\nfn b() {}\n\
             #[cfg(any(test, unix))]\nfn c() {}\n\
             #[cfg(unix)]\nfn d() {}\n",
        );

        assert_eq!(kept, ["b", "c", "d"]);
    }
}
