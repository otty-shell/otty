//! Small readers of the syntax tree that `syn` builds from a source file:
//! the line of a place, the code inside macro calls, the paths and names a
//! file writes, the names its imports bring in and the types it declares.

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{
    AngleBracketedGenericArguments, Attribute, Expr, File, Item, ItemEnum,
    ItemStruct, ItemType, ItemUse, Lifetime, Macro, Pat, Path, Token, UseTree,
};

/// Path starts that point into this crate.
pub(crate) const CRATE_PREFIXES: &[&str] = &["crate", "super", "self"];

/// A path written in a source file, as `use` imports it or as code names
/// it inline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WrittenPath {
    segments: Vec<String>,
    line: usize,
    is_import: bool,
}

impl WrittenPath {
    /// The segments as written, e.g. `["super", "state", "NotesState"]`.
    pub(crate) fn segments(&self) -> &[String] {
        &self.segments
    }

    /// The line of the path; for an import, the line of its `use`.
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    /// Whether a `use` item imports the path.
    pub(crate) fn is_import(&self) -> bool {
        self.is_import
    }
}

/// What kind of type a [`DeclaredType`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeKind {
    Struct,
    Enum,
    Alias,
}

/// A struct, an enum or a type alias that a source file declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclaredType {
    kind: TypeKind,
    name: String,
    line: usize,
}

impl DeclaredType {
    /// Whether it is a struct, an enum or a type alias.
    pub(crate) fn kind(&self) -> TypeKind {
        self.kind
    }

    /// The type name, e.g. `NotesCtx`.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The line of the name.
    pub(crate) fn line(&self) -> usize {
        self.line
    }
}

/// A name that a `use` item brings into scope, with the path it stands
/// for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportAlias {
    name: String,
    path: Vec<String>,
}

impl ImportAlias {
    /// The name in scope, e.g. `run_reduce` for
    /// `use run::reduce as run_reduce;`.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The path the name stands for, e.g. `["run", "reduce"]`.
    pub(crate) fn path(&self) -> &[String] {
        &self.path
    }
}

/// Collects the paths of a file, also inside macro calls.
#[derive(Default)]
struct PathCollector {
    paths: Vec<WrittenPath>,
}

impl<'ast> Visit<'ast> for PathCollector {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let line = line_of(item.use_token.span);

        for segments in use_paths(&item.tree) {
            self.paths.push(WrittenPath {
                segments,
                line,
                is_import: true,
            });
        }
    }

    fn visit_path(&mut self, path: &'ast Path) {
        if let Some(first) = path.segments.first() {
            self.paths.push(WrittenPath {
                segments: path_names(path),
                line: line_of(first.ident.span()),
                is_import: false,
            });
        }

        visit::visit_path(self, path);
    }

    /// A macro body that is no list of expressions, such as
    /// `vec![x; 2]` or `macro_rules!`, is read token by token.
    fn visit_macro(&mut self, mac: &'ast Macro) {
        visit::visit_macro(self, mac);

        match macro_body(mac) {
            Ok(exprs) => {
                for expr in exprs {
                    self.visit_expr(&expr);
                }
            },
            Err(_) => self.paths.extend(token_paths(mac.tokens.clone())),
        }
    }
}

/// Collects every identifier, also inside macro calls.
#[derive(Default)]
struct IdentCollector {
    idents: Vec<Ident>,
}

impl<'ast> Visit<'ast> for IdentCollector {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        self.idents.push(ident.clone());
    }

    /// A lifetime such as `'a` names no value.
    fn visit_lifetime(&mut self, _lifetime: &'ast Lifetime) {}

    fn visit_macro(&mut self, mac: &'ast Macro) {
        visit::visit_macro(self, mac);

        for expr in macro_exprs(mac) {
            self.visit_expr(&expr);
        }
    }
}

/// Collects the structs, enums and type aliases a file declares.
#[derive(Default)]
struct TypeCollector {
    types: Vec<DeclaredType>,
}

impl TypeCollector {
    fn push(&mut self, kind: TypeKind, ident: &Ident) {
        self.types.push(DeclaredType {
            kind,
            name: ident.to_string(),
            line: line_of(ident.span()),
        });
    }
}

impl<'ast> Visit<'ast> for TypeCollector {
    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        self.push(TypeKind::Struct, &item.ident);

        visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        self.push(TypeKind::Enum, &item.ident);

        visit::visit_item_enum(self, item);
    }

    fn visit_item_type(&mut self, item: &'ast ItemType) {
        self.push(TypeKind::Alias, &item.ident);

        visit::visit_item_type(self, item);
    }
}

/// Collects the names that the `use` items of a file bring into scope, at
/// any depth.
#[derive(Default)]
struct AliasCollector {
    aliases: Vec<ImportAlias>,
}

impl<'ast> Visit<'ast> for AliasCollector {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        use_aliases(&item.tree, &[], &mut self.aliases);
    }
}

/// The line where `span` starts, counting from 1.
pub(crate) fn line_of(span: Span) -> usize {
    span.start().line
}

/// The arguments of a macro call such as `vec![a, b]` or
/// `column![title, body]`, when they read as expressions separated by
/// commas; nothing for any other macro body.
pub(crate) fn macro_exprs(mac: &Macro) -> Vec<Expr> {
    macro_body(mac)
        .map(|exprs| exprs.into_iter().collect())
        .unwrap_or_default()
}

/// The segment names of `path`, e.g. `["crate", "theme", "ThemeProps"]`.
pub(crate) fn path_names(path: &Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect()
}

/// The paths of a `use` tree, one per imported name, with groups and `as`
/// renames resolved: `a::{b, c as d, self}` gives `a::b`, `a::c` and `a`;
/// `a::*` gives `a::*`.
pub(crate) fn use_paths(tree: &UseTree) -> Vec<Vec<String>> {
    match tree {
        UseTree::Path(path) => use_paths(&path.tree)
            .into_iter()
            .map(|rest| {
                let mut segments = vec![path.ident.to_string()];
                if rest.first().is_none_or(|first| first != "self") {
                    segments.extend(rest);
                }
                segments
            })
            .collect(),
        UseTree::Name(name) => vec![vec![name.ident.to_string()]],
        UseTree::Rename(rename) => vec![vec![rename.ident.to_string()]],
        UseTree::Glob(_) => vec![vec!["*".to_string()]],
        UseTree::Group(group) => {
            group.items.iter().flat_map(use_paths).collect()
        },
    }
}

/// Every path that `file` writes, in `use` items and inline, also inside
/// macro calls, in the order of the file.
pub(crate) fn written_paths(file: &File) -> Vec<WrittenPath> {
    let mut collector = PathCollector::default();
    collector.visit_file(file);

    collector.paths
}

/// The names that the `use` items of `file` bring into scope, at any depth,
/// each with the path it stands for: `use a::{b as c, self};` gives `c` for
/// `a::b` and `a` for `a`. A glob import brings no name.
pub(crate) fn import_aliases(file: &File) -> Vec<ImportAlias> {
    let mut collector = AliasCollector::default();
    collector.visit_file(file);

    collector.aliases
}

/// `segments` with its first segment replaced by the path that an import
/// of `aliases` gives it, e.g. `iced::time::every` for `time::every` after
/// `use iced::time;`; unchanged when no import brings in the first segment.
pub(crate) fn expanded(
    segments: &[String],
    aliases: &[ImportAlias],
) -> Vec<String> {
    let Some((first, rest)) = segments.split_first() else {
        return Vec::new();
    };

    match aliases.iter().find(|alias| alias.name() == first) {
        Some(alias) => [alias.path(), rest].concat(),
        None => segments.to_vec(),
    }
}

/// Every identifier of `file`, also inside macro calls, in the order of the
/// file.
pub(crate) fn file_idents(file: &File) -> Vec<Ident> {
    let mut collector = IdentCollector::default();
    collector.visit_file(file);

    collector.idents
}

/// Every identifier of `expr`, also inside macro calls.
pub(crate) fn expr_idents(expr: &Expr) -> Vec<Ident> {
    let mut collector = IdentCollector::default();
    collector.visit_expr(expr);

    collector.idents
}

/// Every identifier inside the generic arguments `<...>` of a path
/// segment.
pub(crate) fn generic_idents(
    arguments: &AngleBracketedGenericArguments,
) -> Vec<Ident> {
    let mut collector = IdentCollector::default();
    collector.visit_angle_bracketed_generic_arguments(arguments);

    collector.idents
}

/// Every identifier of `pat`, e.g. `Some`, `cwd` for `Some(cwd)`.
pub(crate) fn pat_idents(pat: &Pat) -> Vec<Ident> {
    let mut collector = IdentCollector::default();
    collector.visit_pat(pat);

    collector.idents
}

/// The structs, enums and type aliases that `file` declares, at any depth.
pub(crate) fn declared_types(file: &File) -> Vec<DeclaredType> {
    let mut collector = TypeCollector::default();
    collector.visit_file(file);

    collector.types
}

/// The attributes of `item`.
pub(crate) fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

/// The body of a macro call read as expressions separated by commas.
fn macro_body(mac: &Macro) -> syn::Result<Punctuated<Expr, Token![,]>> {
    mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
}

/// The paths in `tokens` that start with `crate`, `super` or `self`, inside
/// brackets too, for a macro body that `syn` cannot read as expressions.
fn token_paths(tokens: TokenStream) -> Vec<WrittenPath> {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let mut paths = Vec::new();

    let mut index = 0;
    while index < trees.len() {
        match &trees[index] {
            TokenTree::Group(group) => {
                paths.extend(token_paths(group.stream()));
            },
            TokenTree::Ident(ident)
                if CRATE_PREFIXES.contains(&ident.to_string().as_str()) =>
            {
                let segments = leading_path(&trees[index..]);
                // Every segment after the first one follows two `:`.
                index += segments.len() * 3 - 2;
                if segments.len() > 1 {
                    paths.push(WrittenPath {
                        segments,
                        line: line_of(ident.span()),
                        is_import: false,
                    });
                }
                continue;
            },
            _ => {},
        }
        index += 1;
    }

    paths
}

/// The segments of the path that `trees` start with: an identifier, and
/// one more for every `::` followed by an identifier.
fn leading_path(trees: &[TokenTree]) -> Vec<String> {
    let mut segments = Vec::new();

    let mut rest = trees;
    while let [TokenTree::Ident(ident), tail @ ..] = rest {
        segments.push(ident.to_string());
        match tail {
            [TokenTree::Punct(first), TokenTree::Punct(second), next @ ..]
                if first.as_char() == ':' && second.as_char() == ':' =>
            {
                rest = next;
            },
            _ => break,
        }
    }

    segments
}

/// Add the names that `tree`, under the path `prefix`, brings into scope.
fn use_aliases(
    tree: &UseTree,
    prefix: &[String],
    aliases: &mut Vec<ImportAlias>,
) {
    match tree {
        UseTree::Path(path) => {
            let longer = [prefix, &[path.ident.to_string()]].concat();
            use_aliases(&path.tree, &longer, aliases);
        },
        UseTree::Name(name) => {
            aliases.push(import_alias(prefix, &name.ident, &name.ident));
        },
        UseTree::Rename(rename) => {
            aliases.push(import_alias(prefix, &rename.ident, &rename.rename));
        },
        UseTree::Glob(_) => {},
        UseTree::Group(group) => {
            for item in &group.items {
                use_aliases(item, prefix, aliases);
            }
        },
    }
}

/// The import of `item` under `prefix` as `name`; `self` stands for the
/// prefix itself.
fn import_alias(prefix: &[String], item: &Ident, name: &Ident) -> ImportAlias {
    let mut path = prefix.to_vec();
    if item != "self" {
        path.push(item.to_string());
    }
    let name = if name == "self" {
        prefix.last().cloned().unwrap_or_default()
    } else {
        name.to_string()
    };

    ImportAlias { name, path }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(code: &str) -> File {
        syn::parse_file(code).expect("the test code parses")
    }

    fn joined(paths: &[WrittenPath]) -> Vec<String> {
        paths
            .iter()
            .map(|path| path.segments().join("::"))
            .collect()
    }

    #[test]
    fn a_use_tree_gives_one_path_per_name() {
        let file = parse(
            "use crate::i18n::{self, Key as K, nested::{a, b}};\nuse super::*;\n",
        );

        let paths = written_paths(&file);

        assert_eq!(
            joined(&paths),
            [
                "crate::i18n",
                "crate::i18n::Key",
                "crate::i18n::nested::a",
                "crate::i18n::nested::b",
                "super::*",
            ]
        );
        assert!(paths.iter().all(WrittenPath::is_import));
    }

    #[test]
    fn a_multi_line_import_keeps_the_line_of_its_use() {
        let file = parse(
            "\nuse crate::widgets::tabs::{\n    TabsEvent,\n    TabsIntent,\n};\n",
        );

        let lines: Vec<usize> =
            written_paths(&file).iter().map(WrittenPath::line).collect();

        assert_eq!(lines, [2, 2]);
    }

    #[test]
    fn inline_paths_are_found_inside_generics_and_macro_calls() {
        let file =
            parse("fn f() -> Vec<crate::a::B> {\n    vec![crate::c::d()]\n}\n");

        let paths = written_paths(&file);
        let inline: Vec<(String, usize)> = paths
            .iter()
            .filter(|path| !path.is_import())
            .map(|path| (path.segments().join("::"), path.line()))
            .collect();

        assert_eq!(
            inline,
            [
                ("Vec".to_string(), 1),
                ("crate::a::B".to_string(), 1),
                ("vec".to_string(), 2),
                ("crate::c::d".to_string(), 2),
            ]
        );
    }

    #[test]
    fn crate_paths_in_a_macro_body_that_is_no_list_of_expressions_are_found() {
        let file = parse(
            "fn f() {\n    let _ = vec![crate::a::b(); 3];\n}\n\
             m! {\n    use super::super::x::{Y, Z};\n    self.z;\n    (self::w::V)\n}\n",
        );

        let paths = written_paths(&file);
        let found: Vec<(String, usize)> = paths
            .iter()
            .map(|path| (path.segments().join("::"), path.line()))
            .collect();

        assert_eq!(
            found,
            [
                ("vec".to_string(), 2),
                ("crate::a::b".to_string(), 2),
                ("m".to_string(), 4),
                ("super::super::x".to_string(), 5),
                ("self::w::V".to_string(), 7),
            ]
        );
        assert!(paths.iter().all(|path| !path.is_import()));
    }

    #[test]
    fn imports_give_the_local_name_and_the_full_path() {
        let file = parse(
            "use iced::{self, time, keyboard::listen as keys};\n\
             use run::reduce as run_reduce;\nuse a::b::{self as c};\nuse d::*;\n\
             fn f() {\n    use e::F;\n}\n",
        );

        let aliases = import_aliases(&file);
        let found: Vec<(&str, String)> = aliases
            .iter()
            .map(|alias| (alias.name(), alias.path().join("::")))
            .collect();

        assert_eq!(
            found,
            [
                ("iced", "iced".to_string()),
                ("time", "iced::time".to_string()),
                ("keys", "iced::keyboard::listen".to_string()),
                ("run_reduce", "run::reduce".to_string()),
                ("c", "a::b".to_string()),
                ("F", "e::F".to_string()),
            ]
        );
    }

    #[test]
    fn a_path_is_expanded_through_the_import_of_its_first_segment() {
        let file = parse("use iced::time;\nuse run::reduce as run_reduce;\n");
        let aliases = import_aliases(&file);
        let segments = |path: &str| -> Vec<String> {
            path.split("::").map(str::to_string).collect()
        };

        let expand =
            |path: &str| expanded(&segments(path), &aliases).join("::");

        assert_eq!(expand("time::every"), "iced::time::every");
        assert_eq!(expand("run_reduce"), "run::reduce");
        assert_eq!(expand("other::every"), "other::every");
    }

    #[test]
    fn identifiers_skip_comments_and_strings() {
        let file = parse(
            "// Task in a comment\n/// Task in a doc\nfn f() -> &'static str {\n    \"Task\"\n}\n",
        );

        let names: Vec<String> =
            file_idents(&file).iter().map(Ident::to_string).collect();

        assert_eq!(names, ["doc", "f", "str"]);
    }

    #[test]
    fn declared_types_are_found_at_any_depth() {
        let file = parse(
            "struct A;\nenum B {}\ntype C = u8;\nfn f() {\n    struct D;\n}\n",
        );

        let types: Vec<(TypeKind, String, usize)> = declared_types(&file)
            .iter()
            .map(|declared| {
                (
                    declared.kind(),
                    declared.name().to_string(),
                    declared.line(),
                )
            })
            .collect();

        assert_eq!(
            types,
            [
                (TypeKind::Struct, "A".to_string(), 1),
                (TypeKind::Enum, "B".to_string(), 2),
                (TypeKind::Alias, "C".to_string(), 3),
                (TypeKind::Struct, "D".to_string(), 5),
            ]
        );
    }
}
