//! Rules about where the clippy ban on files, processes and tasks may be
//! lifted.

use super::super::finding::{Finding, Rule};
use super::super::source::{Line, SourceFile, SourceTree, declared_module};
use super::super::text::is_ident_char;

/// The lint whose ban only some modules may lift.
const LINT: &str = "clippy::disallowed_methods";

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

/// Report places where `clippy::disallowed_methods` is lifted outside the
/// allowed module declarations.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    tree.files().iter().flat_map(lifted_bans).collect()
}

fn lifted_bans(file: &SourceFile) -> Vec<Finding> {
    let lines = file.lines();
    let mut findings = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let text = lines[index].text().trim();
        if !text.starts_with("#[") && !text.starts_with("#![") {
            index += 1;
            continue;
        }

        let (attribute, end) = attribute_at(lines, index);
        let next = end + 1;
        if !attribute.contains(LINT) {
            index = next;
            continue;
        }

        let key = if attribute.starts_with("#![forbid(")
            || attribute.starts_with("#![deny(")
        {
            None
        } else if attribute.starts_with("#![") {
            Some("inner attribute".to_string())
        } else if attribute.starts_with("#[allow(") {
            Some("allow".to_string())
        } else if attribute.starts_with("#[expect(") {
            not_allowed_item(lines, next)
        } else {
            None
        };
        if let Some(key) = key {
            findings.push(Finding::new(
                Rule::DisallowedMethodsLifted,
                file.path(),
                &key,
                lines[index].number(),
            ));
        }
        index = next;
    }

    findings
}

/// The attribute starting at `lines[start]`, joined, and the index of its
/// last line.
fn attribute_at(lines: &[Line], start: usize) -> (String, usize) {
    let mut joined = String::new();
    let mut index = start;

    loop {
        joined.push_str(lines[index].text().trim());
        if joined.ends_with(']') || index + 1 >= lines.len() {
            return (joined, index);
        }
        index += 1;
    }
}

/// `None` when the item at or after `lines[start]` is an allowed module
/// declaration; otherwise a key naming the item.
fn not_allowed_item(lines: &[Line], start: usize) -> Option<String> {
    let mut index = start;
    while index < lines.len() && lines[index].text().trim().starts_with("#[") {
        index = attribute_at(lines, index).1 + 1;
    }

    let Some(item) = lines.get(index) else {
        return Some("item".to_string());
    };
    if let Some(name) = declared_module(item.text()) {
        if MODULES_THAT_MAY_LIFT.contains(&name) {
            return None;
        }
        return Some(format!("mod {name}"));
    }

    Some(item_name(item.text()))
}

/// `fn load` for `pub(crate) fn load() -> Self {`.
fn item_name(text: &str) -> String {
    let trimmed = text.trim();
    let without_vis = trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .unwrap_or(trimmed);

    let mut words = without_vis.split_whitespace();
    let kind = words.next().unwrap_or("item");
    let name: String = words
        .next()
        .unwrap_or("")
        .chars()
        .take_while(|ch| is_ident_char(*ch))
        .collect();

    format!("{kind} {name}").trim().to_string()
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
}
