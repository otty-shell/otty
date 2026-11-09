//! Rules about where the clippy ban on files, processes and tasks may be
//! lifted.

use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{AttrStyle, Attribute, ImplItemFn, Item, ItemMod, Meta, Token};

use super::super::finding::{Finding, Rule};
use super::super::layout::{SHARED_IO_MODULES, part_names};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{item_attrs, path_names};

/// The lint whose ban only some modules may lift.
const LINT: &[&str] = &["clippy", "disallowed_methods"];

/// Lint groups that hold [`LINT`], so allowing one of them lifts the ban
/// too: clippy puts `disallowed_methods` in `clippy::style`, `clippy::all`
/// holds `clippy::style`, and `warnings` holds every lint that warns.
const LINT_GROUPS: &[&[&str]] =
    &[&["clippy", "all"], &["clippy", "style"], &["warnings"]];

/// The lint levels that lift a ban.
const LIFTING_LEVELS: &[&str] = &["allow", "expect"];

/// The module of all routes, which may lift the ban anywhere inside.
const ROUTER: &str = "router";

/// Modules of a widget that exist to talk to the outside world.
const WIDGET_MODULES_THAT_MAY_LIFT: &[&str] =
    &["services", "storage", "subscription"];

/// The module of a part that may lift the ban: a part has no route, so no
/// `services` or `storage`.
const PART_MODULE_THAT_MAY_LIFT: &str = "subscription";

/// One attribute of a file that lifts the ban on [`LINT`], with the item it
/// belongs to when it belongs to an item.
struct LintAttribute {
    is_inner: bool,
    level: String,
    owner: Option<Owner>,
    line: usize,
    column: usize,
}

/// The item an attribute belongs to: a module declaration, by its path from
/// the crate root, may lift the ban when it lies in an allowed place; any
/// other item never.
#[derive(Clone)]
enum Owner {
    Module(Vec<String>),
    Other(String),
}

/// Collects the attributes that lift the ban, at any depth. An attribute
/// is found first; once the walk has left an item or a method, it claims
/// its own attributes as their owner. Attributes are told apart by where
/// they start.
struct LintAttributes {
    /// The path of the module the walk is in, from the crate root.
    module: Vec<String>,
    found: Vec<LintAttribute>,
}

impl LintAttributes {
    /// Start a walk of the file whose module is `module`.
    fn new(module: &[String]) -> Self {
        Self {
            module: module.to_vec(),
            found: Vec::new(),
        }
    }

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

    /// `mod name` by its path for a module, `fn load` for a function and
    /// `item` for any other item.
    fn owner_of(&self, item: &Item) -> Owner {
        match item {
            Item::Mod(module) => {
                let mut path = self.module.clone();
                path.push(module.ident.to_string());
                Owner::Module(path)
            },
            Item::Fn(function) => {
                Owner::Other(format!("fn {}", function.sig.ident))
            },
            _ => Owner::Other("item".to_string()),
        }
    }
}

impl<'ast> Visit<'ast> for LintAttributes {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if let Some(level) = lifting_level(&attr.meta) {
            let start = attr.pound_token.span.start();
            self.found.push(LintAttribute {
                is_inner: matches!(attr.style, AttrStyle::Inner(_)),
                level,
                owner: None,
                line: start.line,
                column: start.column,
            });
        }

        visit::visit_attribute(self, attr);
    }

    fn visit_item(&mut self, item: &'ast Item) {
        visit::visit_item(self, item);

        let owner = self.owner_of(item);
        self.claim(item_attrs(item), &owner);
    }

    /// The items of an inline module lie one module deeper.
    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        let is_inline = module.content.is_some();
        if is_inline {
            self.module.push(module.ident.to_string());
        }

        visit::visit_item_mod(self, module);

        if is_inline {
            self.module.pop();
        }
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
    tree.files()
        .iter()
        .flat_map(|file| lifted_bans(tree, file))
        .collect()
}

fn lifted_bans(tree: &SourceTree, file: &SourceFile) -> Vec<Finding> {
    let mut attributes = LintAttributes::new(file.module());
    attributes.visit_file(file.syntax());

    attributes
        .found
        .iter()
        .filter_map(|attr| {
            let key = lifted_ban_key(tree, attr)?;

            Some(Finding::new(
                Rule::DisallowedMethodsLifted,
                file.path(),
                &key,
                attr.line,
            ))
        })
        .collect()
}

/// The key of an attribute that lifts the ban where it may not, or `None`
/// when it lifts it with `expect` on a module in an allowed place.
fn lifted_ban_key(tree: &SourceTree, attr: &LintAttribute) -> Option<String> {
    if attr.is_inner {
        return Some("inner attribute".to_string());
    }
    if attr.level != "expect" {
        return Some(attr.level.clone());
    }

    match &attr.owner {
        Some(Owner::Module(path)) => {
            let name = path.last().map_or("", String::as_str);
            (!may_lift(tree, path)).then(|| format!("mod {name}"))
        },
        Some(Owner::Other(key)) => Some(key.clone()),
        None => Some("item".to_string()),
    }
}

/// The level, `allow` or `expect`, with which `meta` lifts the ban: it
/// names [`LINT`] or one of [`LINT_GROUPS`], directly or inside
/// `cfg_attr(<condition>, ..)` whatever the condition.
fn lifting_level(meta: &Meta) -> Option<String> {
    let Meta::List(list) = meta else {
        return None;
    };
    let level = list.path.get_ident()?.to_string();
    let arguments = list
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .ok()?;

    if level == "cfg_attr" {
        return arguments.iter().skip(1).find_map(lifting_level);
    }
    let names_ban = arguments.iter().any(|argument| {
        let path = path_names(argument.path());
        path == LINT || LINT_GROUPS.iter().any(|group| path == *group)
    });

    (LIFTING_LEVELS.contains(&level.as_str()) && names_ban).then_some(level)
}

/// Whether a module declared at `module` may lift the ban: it is, or lies
/// inside, `router`, a shared module that talks to the outside world, a
/// widget's `services`, `storage` or `subscription`, or a part's
/// `subscription`.
fn may_lift(tree: &SourceTree, module: &[String]) -> bool {
    let names: Vec<&str> = module.iter().map(String::as_str).collect();

    match names.as_slice() {
        [first, ..] if *first == ROUTER => true,
        [first, ..] if SHARED_IO_MODULES.contains(first) => true,
        ["widgets", _, inner, ..]
            if WIDGET_MODULES_THAT_MAY_LIFT.contains(inner) =>
        {
            true
        },
        ["widgets", widget, part, inner, ..]
            if *inner == PART_MODULE_THAT_MAY_LIFT =>
        {
            part_names(tree, widget).iter().any(|name| name == part)
        },
        _ => false,
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
                pair("widgets/tabs/reducer.rs", "allow"),
                pair("widgets/tabs/state.rs", "item"),
                pair("widgets/tabs/types.rs", "item"),
            ]
        );
    }

    #[test]
    fn a_lint_group_that_holds_the_ban_lifts_it_too() {
        let tree = SourceTree::from_sources(&[
            (
                "main.rs",
                "#[expect(clippy::style, reason = \"routes\")]\nmod router;\n",
            ),
            (
                "widgets/explorer/mod.rs",
                "#[expect(clippy::style, reason = \"x\")]\nmod helpers;\n",
            ),
            (
                "widgets/tabs/model.rs",
                "#[allow(clippy::all)]\nfn read() {}\n",
            ),
            ("widgets/notes/state.rs", "#![allow(warnings)]\n"),
            (
                "widgets/modes/types.rs",
                "#[allow(clippy::pedantic)]\nfn a() {}\n#[allow(dead_code)]\nfn b() {}\n\
                 #[deny(warnings)]\nfn c() {}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::DisallowedMethodsLifted),
            [
                pair("widgets/explorer/mod.rs", "mod helpers"),
                pair("widgets/notes/state.rs", "inner attribute"),
                pair("widgets/tabs/model.rs", "allow"),
            ]
        );
    }

    #[test]
    fn a_lift_inside_cfg_attr_counts_whatever_the_condition() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/settings/mod.rs",
                "#[cfg_attr(all(), allow(clippy::disallowed_methods))]\nmod io;\n",
            ),
            (
                "widgets/tabs/mod.rs",
                "#[cfg_attr(unix, expect(clippy::disallowed_methods, reason = \"x\"))]\nmod state;\n",
            ),
            (
                "widgets/notes/mod.rs",
                "#[cfg_attr(unix, cfg_attr(not(test), expect(clippy::all)))]\npub(crate) mod storage;\n",
            ),
            (
                "widgets/palette/mod.rs",
                "#[cfg_attr(test, derive(Debug))]\nstruct Palette;\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::DisallowedMethodsLifted),
            [
                pair("widgets/settings/mod.rs", "allow"),
                pair("widgets/tabs/mod.rs", "mod state"),
            ]
        );
    }

    #[test]
    fn a_lift_is_allowed_by_the_place_of_the_module_not_by_its_name() {
        let lift = "#[expect(clippy::disallowed_methods, reason = \"x\")]";
        let main = format!(
            "{lift}\nmod services;\n{lift}\nmod shell_integration;\n{lift}\nmod theme;\n\
             {lift}\nmod tests;\n#[cfg(test)]\n{lift}\nmod testing;\n"
        );
        let modes_state = format!("{lift}\nmod storage;\n");
        let explorer_view = format!("{lift}\nmod services;\n");
        let containers = format!("{lift}\nmod subscription;\n");
        let run = format!("{lift}\nmod subscription;\n{lift}\nmod storage;\n");
        let router = format!("{lift}\nmod tabs;\n");
        let notes_storage = format!("{lift}\nmod paths;\n");
        let tabs_state =
            format!("mod inner {{\n    {lift}\n    mod storage;\n}}\n");
        let tree = SourceTree::from_sources(&[
            ("main.rs", main.as_str()),
            ("widgets/modes/state.rs", modes_state.as_str()),
            ("widgets/explorer/view/mod.rs", explorer_view.as_str()),
            ("widgets/containers/mod.rs", containers.as_str()),
            ("widgets/containers/run/mod.rs", run.as_str()),
            ("router/mod.rs", router.as_str()),
            ("widgets/notes/storage.rs", notes_storage.as_str()),
            ("widgets/tabs/state.rs", tabs_state.as_str()),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::DisallowedMethodsLifted),
            [
                pair("main.rs", "mod tests"),
                pair("main.rs", "mod theme"),
                pair("widgets/containers/run/mod.rs", "mod storage"),
                pair("widgets/explorer/view/mod.rs", "mod services"),
                pair("widgets/modes/state.rs", "mod storage"),
                pair("widgets/tabs/state.rs", "mod storage"),
            ]
        );
    }
}
