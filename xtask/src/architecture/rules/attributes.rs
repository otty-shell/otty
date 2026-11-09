//! Rules about where the clippy ban on files, processes and tasks may be
//! lifted.

use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{AttrStyle, Attribute, ImplItemFn, Item, Meta, Token};

use super::super::finding::{Finding, Rule};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{item_attrs, path_names};

/// The lint whose ban only some modules may lift.
const LINT: &[&str] = &["clippy", "disallowed_methods"];

/// Modules that exist to talk to the outside world, and test modules.
const MODULES_THAT_MAY_LIFT: &[&str] = &[
    "router",
    "services",
    "storage",
    "subscription",
    "shell_integration",
    "testing",
    "tests",
];

/// One attribute of a file that names [`LINT`], with the item it belongs
/// to when it belongs to an item.
struct LintAttribute {
    is_inner: bool,
    name: String,
    owner: Option<Owner>,
    line: usize,
    column: usize,
}

/// The item an attribute belongs to: a module declaration may lift the ban
/// when its name is in [`MODULES_THAT_MAY_LIFT`], any other item never.
#[derive(Clone)]
enum Owner {
    Module(String),
    Other(String),
}

/// Collects the attributes that name [`LINT`], at any depth. An attribute
/// is found first; once the walk has left an item or a method, it claims
/// its own attributes as their owner. Attributes are told apart by where
/// they start.
#[derive(Default)]
struct LintAttributes {
    found: Vec<LintAttribute>,
}

impl LintAttributes {
    /// Record `owner` as the owner of the lint attributes among `attrs`.
    fn claim(&mut self, attrs: &[Attribute], owner: &Owner) {
        for attr in attrs {
            let start = attr.pound_token.span.start();
            if let Some(found) = self.found.iter_mut().find(|found| {
                found.line == start.line && found.column == start.column
            }) {
                found.owner = Some(owner.clone());
            }
        }
    }
}

impl<'ast> Visit<'ast> for LintAttributes {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if names_lint(attr) {
            let start = attr.pound_token.span.start();
            self.found.push(LintAttribute {
                is_inner: matches!(attr.style, AttrStyle::Inner(_)),
                name: attr
                    .path()
                    .get_ident()
                    .map_or_else(String::new, |name| name.to_string()),
                owner: None,
                line: start.line,
                column: start.column,
            });
        }

        visit::visit_attribute(self, attr);
    }

    fn visit_item(&mut self, item: &'ast Item) {
        visit::visit_item(self, item);

        self.claim(item_attrs(item), &item_owner(item));
    }

    fn visit_impl_item_fn(&mut self, function: &'ast ImplItemFn) {
        visit::visit_impl_item_fn(self, function);

        let owner = Owner::Other(format!("fn {}", function.sig.ident));
        self.claim(&function.attrs, &owner);
    }
}

/// Report places where `clippy::disallowed_methods` is lifted outside the
/// allowed module declarations.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    tree.files().iter().flat_map(lifted_bans).collect()
}

fn lifted_bans(file: &SourceFile) -> Vec<Finding> {
    let mut attributes = LintAttributes::default();
    attributes.visit_file(file.syntax());

    attributes
        .found
        .iter()
        .filter_map(|attr| {
            let key = lifted_ban_key(attr)?;

            Some(Finding::new(
                Rule::DisallowedMethodsLifted,
                file.path(),
                &key,
                attr.line,
            ))
        })
        .collect()
}

/// The key of a lint attribute that lifts the ban where it may not, or
/// `None` when it keeps the ban or lifts it on an allowed module.
fn lifted_ban_key(attr: &LintAttribute) -> Option<String> {
    let name = attr.name.as_str();

    if attr.is_inner {
        return (name != "forbid" && name != "deny")
            .then(|| "inner attribute".to_string());
    }
    match name {
        "allow" => Some("allow".to_string()),
        "expect" => match &attr.owner {
            Some(Owner::Module(module)) => (!MODULES_THAT_MAY_LIFT
                .contains(&module.as_str()))
            .then(|| format!("mod {module}")),
            Some(Owner::Other(key)) => Some(key.clone()),
            None => Some("item".to_string()),
        },
        _ => None,
    }
}

/// Whether `attr` lists [`LINT`] among its arguments, as in
/// `#[expect(clippy::disallowed_methods, reason = "...")]`.
fn names_lint(attr: &Attribute) -> bool {
    let Ok(arguments) =
        attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
    else {
        return false;
    };

    arguments
        .iter()
        .any(|argument| path_names(argument.path()) == LINT)
}

/// `mod name` for a module, `fn load` for a function and `item` for any
/// other item.
fn item_owner(item: &Item) -> Owner {
    match item {
        Item::Mod(module) => Owner::Module(module.ident.to_string()),
        Item::Fn(function) => {
            Owner::Other(format!("fn {}", function.sig.ident))
        },
        _ => Owner::Other("item".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    #[test]
    fn the_ban_may_be_lifted_on_the_allowed_modules() {
        let tree = SourceTree::from_sources(&[
            (
                "main.rs",
                "#[expect(\n    clippy::disallowed_methods,\n    reason = \"routes\"\n)]\nmod router;\n",
            ),
            (
                "widgets/notes/mod.rs",
                "#[expect(clippy::disallowed_methods, reason = \"x\")]\npub(crate) mod storage;\n",
            ),
        ]);

        assert!(found(&check(&tree), Rule::DisallowedMethodsLifted).is_empty());
    }

    #[test]
    fn lifting_the_ban_anywhere_else_is_reported() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/notes/mod.rs",
                "#[expect(clippy::disallowed_methods, reason = \"x\")]\nmod state;\n",
            ),
            (
                "widgets/tabs/model.rs",
                "#[allow(clippy::disallowed_methods)]\nfn read() {}\n",
            ),
            (
                "widgets/sidebar/state.rs",
                "#![expect(clippy::disallowed_methods, reason = \"x\")]\n",
            ),
            (
                "widgets/modes/state.rs",
                "#[expect(clippy::disallowed_methods, reason = \"x\")]\nfn load() {}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::DisallowedMethodsLifted),
            [
                pair("widgets/modes/state.rs", "fn load"),
                pair("widgets/notes/mod.rs", "mod state"),
                pair("widgets/sidebar/state.rs", "inner attribute"),
                pair("widgets/tabs/model.rs", "allow"),
            ]
        );
    }

    #[test]
    fn a_method_a_statement_and_a_forbid_are_told_apart() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/notes/state.rs",
                "impl NotesState {\n    \
                 #[expect(clippy::disallowed_methods, reason = \"x\")]\n    fn load() {}\n}\n",
            ),
            (
                "widgets/tabs/state.rs",
                "fn read() {\n    \
                 #[expect(clippy::disallowed_methods, reason = \"x\")]\n    \
                 let text = std::fs::read_to_string(path);\n}\n",
            ),
            (
                "widgets/tabs/reducer.rs",
                "#![forbid(clippy::disallowed_methods)]\n\
                 #[cfg_attr(test, allow(clippy::disallowed_methods))]\nfn reduce() {}\n",
            ),
            (
                "widgets/tabs/types.rs",
                "#[expect(clippy::disallowed_methods, reason = \"x\")]\nstruct Paths;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::DisallowedMethodsLifted),
            [
                pair("widgets/notes/state.rs", "fn load"),
                pair("widgets/tabs/state.rs", "item"),
                pair("widgets/tabs/types.rs", "item"),
            ]
        );
    }
}
