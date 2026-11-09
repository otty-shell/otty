//! Small readers of the syntax tree that `syn` builds from a source file:
//! the line of a place, the code inside macro calls, the paths and names a
//! file writes and the types it declares.

use proc_macro2::{Ident, Span};
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{
    AngleBracketedGenericArguments, Attribute, Expr, File, Item, ItemEnum,
    ItemStruct, ItemType, ItemUse, Lifetime, Macro, Pat, Path, Token, UseTree,
};

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

    fn visit_macro(&mut self, mac: &'ast Macro) {
        visit::visit_macro(self, mac);

        for expr in macro_exprs(mac) {
            self.visit_expr(&expr);
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

/// The line where `span` starts, counting from 1.
pub(crate) fn line_of(span: Span) -> usize {
    span.start().line
}

/// The arguments of a macro call such as `vec![a, b]` or
/// `column![title, body]`, when they read as expressions separated by
/// commas; nothing for any other macro body.
pub(crate) fn macro_exprs(mac: &Macro) -> Vec<Expr> {
    mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
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
    fn a_macro_body_that_is_no_list_of_expressions_is_skipped() {
        let file = parse("fn f() {\n    let _ = vec![crate::a::b(); 3];\n}\n");

        let paths = written_paths(&file);

        assert_eq!(joined(&paths), ["vec"]);
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
