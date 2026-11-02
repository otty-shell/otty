//! The engine's inspect JSON as readable rows (spec Decision 20): top-level
//! values that read in one line go to a General group, the rest become
//! sections; nested objects are indented rows and lists are numbered rows.

use otty_ui_kit::components::data_tree::DataRow;
use serde_json::{Map, Value};

/// General values that read first.
const GENERAL_FIRST: [&str; 3] = ["Id", "Name", "Created"];
/// Sections that read first; the others follow in the order of the JSON
/// object.
const SECTIONS_FIRST: [&str; 5] =
    ["State", "Config", "HostConfig", "NetworkSettings", "Mounts"];
/// The longest list of scalars, in characters, that reads as one joined
/// value.
const JOINED_LIST_MAX: usize = 60;

/// A top-level key with rows under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectSection {
    pub(crate) title: String,
    pub(crate) rows: Vec<DataRow>,
}

/// The rows of one inspect answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectTree {
    /// The top-level values that read in one line.
    pub(crate) general: Vec<DataRow>,
    /// The other top-level keys, in reading order.
    pub(crate) sections: Vec<InspectSection>,
}

/// The rows of `value`; `None` when it is not a JSON object. Empty values
/// (null, "", [], {} and objects or lists holding only those) are left out
/// unless `show_empty`.
pub(crate) fn inspect_tree(
    value: &Value,
    show_empty: bool,
) -> Option<InspectTree> {
    let object = value.as_object()?;

    let mut general = Vec::new();
    for (key, value) in in_order(object, &GENERAL_FIRST) {
        if is_hidden(value, show_empty) {
            continue;
        }
        if let Some(text) = one_line(value, show_empty) {
            general.push(DataRow::new(0, key, text));
        }
    }

    let mut sections = Vec::new();
    for (key, value) in in_order(object, &SECTIONS_FIRST) {
        if is_hidden(value, show_empty) || one_line(value, show_empty).is_some()
        {
            continue;
        }

        let mut rows = Vec::new();
        push_children(&mut rows, value, 0, show_empty);
        sections.push(InspectSection {
            title: key.to_string(),
            rows,
        });
    }

    Some(InspectTree { general, sections })
}

/// The entries of `object`: the keys of `first` that it has, in that order,
/// then the others in the order of the object.
fn in_order<'a>(
    object: &'a Map<String, Value>,
    first: &[&str],
) -> Vec<(&'a String, &'a Value)> {
    let listed = first.iter().filter_map(|key| object.get_key_value(*key));
    let others = object
        .iter()
        .filter(|(key, _)| !first.contains(&key.as_str()));

    listed.chain(others).collect()
}

/// Add the rows of the items of `value`, an object or a list, at `depth`.
/// List items are numbered from 1; a hidden item keeps its number free.
fn push_children(
    rows: &mut Vec<DataRow>,
    value: &Value,
    depth: u16,
    show_empty: bool,
) {
    match value {
        Value::Object(object) => {
            for (key, item) in object {
                push_entry(rows, key, item, depth, show_empty);
            }
        },
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let number = (index + 1).to_string();
                push_entry(rows, &number, item, depth, show_empty);
            }
        },
        _ => {},
    }
}

/// Add the rows of `key`: one key and value row when `value` reads in one
/// line, or else a heading with the rows of its items one level deeper.
fn push_entry(
    rows: &mut Vec<DataRow>,
    key: &str,
    value: &Value,
    depth: u16,
    show_empty: bool,
) {
    if is_hidden(value, show_empty) {
        return;
    }

    if let Some(text) = one_line(value, show_empty) {
        rows.push(DataRow::new(depth, key, text));
        return;
    }

    rows.push(DataRow::heading(depth, key));
    push_children(rows, value, depth + 1, show_empty);
}

/// Whether `value` is left out of the tree.
fn is_hidden(value: &Value, show_empty: bool) -> bool {
    !show_empty && holds_nothing(value)
}

/// Whether `value` is null, "", [] or {}, or a list or an object holding
/// only those. 0 and false are values.
fn holds_nothing(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.is_empty(),
        Value::Array(items) => items.iter().all(holds_nothing),
        Value::Object(object) => object.values().all(holds_nothing),
        _ => false,
    }
}

/// `value` as one line of text: a scalar, an empty list or object, or a
/// short list of scalars. `None` when it needs rows of its own.
fn one_line(value: &Value, show_empty: bool) -> Option<String> {
    match value {
        Value::Array(items) if !items.is_empty() => {
            joined_list(items, show_empty)
        },
        Value::Object(object) if !object.is_empty() => None,
        _ => Some(plain_text(value)),
    }
}

/// `items` joined with commas, when all shown ones are scalars and the text
/// is short.
fn joined_list(items: &[Value], show_empty: bool) -> Option<String> {
    let mut shown = Vec::new();
    for item in items {
        if is_hidden(item, show_empty) {
            continue;
        }
        if item.is_array() || item.is_object() {
            return None;
        }

        shown.push(plain_text(item));
    }

    let joined = shown.join(", ");
    (joined.chars().count() <= JOINED_LIST_MAX).then_some(joined)
}

/// A value without rows of its own as text: a string as it is (an empty one
/// as `""`), and anything else as JSON writes it.
fn plain_text(value: &Value) -> String {
    match value {
        Value::String(text) if !text.is_empty() => text.clone(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn tree(value: &Value) -> InspectTree {
        inspect_tree(value, false).expect("an object")
    }

    fn tree_with_empty(value: &Value) -> InspectTree {
        inspect_tree(value, true).expect("an object")
    }

    fn titles(tree: &InspectTree) -> Vec<&str> {
        tree.sections
            .iter()
            .map(|section| section.title.as_str())
            .collect()
    }

    fn section<'a>(tree: &'a InspectTree, title: &str) -> &'a [DataRow] {
        &tree
            .sections
            .iter()
            .find(|section| section.title == title)
            .expect("the section")
            .rows
    }

    #[test]
    fn top_level_scalars_go_to_general_with_id_name_and_created_first() {
        let value = json!({
            "Driver": "overlay2",
            "RestartCount": 0,
            "Name": "/web",
            "Created": "2026-11-01T18:30:00Z",
            "AppArmorProfile": "default",
            "Id": "850b",
            "Privileged": false,
        });

        let tree = tree(&value);

        assert_eq!(
            tree.general,
            vec![
                DataRow::new(0, "Id", "850b"),
                DataRow::new(0, "Name", "/web"),
                DataRow::new(0, "Created", "2026-11-01T18:30:00Z"),
                DataRow::new(0, "AppArmorProfile", "default"),
                DataRow::new(0, "Driver", "overlay2"),
                DataRow::new(0, "Privileged", "false"),
                DataRow::new(0, "RestartCount", "0"),
            ]
        );
        assert!(tree.sections.is_empty());
    }

    #[test]
    fn the_known_sections_come_first_in_their_order_then_the_rest() {
        let value = json!({
            "Zebra": { "a": 1 },
            "Mounts": [{ "Type": "bind" }],
            "NetworkSettings": { "Bridge": "docker0" },
            "HostConfig": { "Memory": 5 },
            "GraphDriver": { "Name": "overlay2" },
            "Config": { "Image": "busybox" },
            "State": { "Status": "running" },
        });

        let tree = tree(&value);

        assert_eq!(
            titles(&tree),
            vec![
                "State",
                "Config",
                "HostConfig",
                "NetworkSettings",
                "Mounts",
                "GraphDriver",
                "Zebra",
            ]
        );
        assert!(tree.general.is_empty());
    }

    #[test]
    fn nested_objects_become_indented_rows_under_a_heading() {
        let value = json!({
            "State": {
                "Status": "running",
                "Health": { "Status": "healthy", "Probe": { "Retries": 3 } },
            },
        });

        let tree = tree(&value);

        assert_eq!(
            section(&tree, "State"),
            [
                DataRow::heading(0, "Health"),
                DataRow::heading(1, "Probe"),
                DataRow::new(2, "Retries", "3"),
                DataRow::new(1, "Status", "healthy"),
                DataRow::new(0, "Status", "running"),
            ]
        );
    }

    #[test]
    fn a_short_list_of_scalars_reads_as_one_joined_value() {
        let value = json!({
            "Config": { "Cmd": ["sleep", 3600, true] },
        });

        let tree = tree(&value);

        assert_eq!(
            section(&tree, "Config"),
            [DataRow::new(0, "Cmd", "sleep, 3600, true")]
        );
    }

    #[test]
    fn a_joined_list_stops_at_sixty_characters() {
        // Two items and the ", " between them: 60 and 61 characters.
        let fits =
            json!({ "Config": { "Cmd": ["a".repeat(29), "b".repeat(29)] } });
        let over =
            json!({ "Config": { "Cmd": ["a".repeat(30), "b".repeat(29)] } });

        let fits = tree(&fits);
        let over = tree(&over);

        assert_eq!(section(&fits, "Config").len(), 1);
        assert_eq!(section(&over, "Config").len(), 3);
    }

    #[test]
    fn a_long_list_of_scalars_becomes_numbered_rows() {
        let first = "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin";
        let value = json!({
            "Config": { "Env": [first, "LANG=C.UTF-8"] },
        });

        let tree = tree(&value);

        assert_eq!(
            section(&tree, "Config"),
            [
                DataRow::heading(0, "Env"),
                DataRow::new(1, "1", first),
                DataRow::new(1, "2", "LANG=C.UTF-8"),
            ]
        );
    }

    #[test]
    fn a_list_of_objects_becomes_numbered_headings_with_their_rows() {
        let value = json!({
            "Mounts": [
                { "Type": "volume", "RW": true },
                { "Type": "bind", "RW": false },
            ],
        });

        let tree = tree(&value);

        assert_eq!(
            section(&tree, "Mounts"),
            [
                DataRow::heading(0, "1"),
                DataRow::new(1, "RW", "true"),
                DataRow::new(1, "Type", "volume"),
                DataRow::heading(0, "2"),
                DataRow::new(1, "RW", "false"),
                DataRow::new(1, "Type", "bind"),
            ]
        );
    }

    #[test]
    fn a_top_level_list_of_scalars_goes_to_general_when_short_and_to_a_section_when_long()
     {
        let long = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef";
        let value = json!({
            "Args": ["-c", "sleep 1"],
            "RepoDigests": [long, "nginx:1"],
        });

        let tree = tree(&value);

        assert_eq!(tree.general, vec![DataRow::new(0, "Args", "-c, sleep 1")]);
        assert_eq!(
            section(&tree, "RepoDigests"),
            [DataRow::new(0, "1", long), DataRow::new(0, "2", "nginx:1")]
        );
    }

    #[test]
    fn empty_values_are_hidden_but_zero_and_false_are_not() {
        let value = json!({
            "Null": null,
            "Text": "",
            "List": [],
            "Object": {},
            "Zero": 0,
            "No": false,
            "Config": {
                "Null": null,
                "Text": "",
                "List": [],
                "Object": {},
                "Zero": 0,
                "No": false,
            },
        });

        let tree = tree(&value);

        assert_eq!(
            tree.general,
            vec![DataRow::new(0, "No", "false"), DataRow::new(0, "Zero", "0")]
        );
        assert_eq!(
            section(&tree, "Config"),
            [DataRow::new(0, "No", "false"), DataRow::new(0, "Zero", "0")]
        );
    }

    #[test]
    fn a_key_holding_only_empty_values_is_hidden_with_its_heading() {
        let value = json!({
            "GraphDriver": { "Data": null, "Options": { "a": "" } },
            "Config": {
                "Labels": { "a": null },
                "Volumes": [{}, []],
                "Image": "busybox",
            },
        });

        let tree = tree(&value);

        assert_eq!(titles(&tree), vec!["Config"]);
        assert_eq!(
            section(&tree, "Config"),
            [DataRow::new(0, "Image", "busybox")]
        );
    }

    #[test]
    fn empty_items_of_a_list_are_hidden_and_the_others_keep_their_number() {
        let long =
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123";
        let value = json!({
            "Config": {
                "Short": ["a", "", null, "b"],
                "Long": ["", long, null, "c"],
            },
        });

        let tree = tree(&value);

        assert_eq!(
            section(&tree, "Config"),
            [
                DataRow::heading(0, "Long"),
                DataRow::new(1, "2", long),
                DataRow::new(1, "4", "c"),
                DataRow::new(0, "Short", "a, b"),
            ]
        );
    }

    #[test]
    fn show_empty_shows_each_empty_value_as_it_is_written() {
        let value = json!({
            "Null": null,
            "Text": "",
            "List": [],
            "Object": {},
            "Config": {
                "Null": null,
                "Text": "",
                "List": [],
                "Object": {},
                "Mixed": ["", null, "a"],
            },
        });

        let tree = tree_with_empty(&value);

        assert_eq!(
            tree.general,
            vec![
                DataRow::new(0, "List", "[]"),
                DataRow::new(0, "Null", "null"),
                DataRow::new(0, "Object", "{}"),
                DataRow::new(0, "Text", "\"\""),
            ]
        );
        assert_eq!(
            section(&tree, "Config"),
            [
                DataRow::new(0, "List", "[]"),
                DataRow::new(0, "Mixed", "\"\", null, a"),
                DataRow::new(0, "Null", "null"),
                DataRow::new(0, "Object", "{}"),
                DataRow::new(0, "Text", "\"\""),
            ]
        );
    }

    #[test]
    fn show_empty_brings_back_a_key_holding_only_empty_values() {
        let value = json!({
            "GraphDriver": { "Data": null, "Options": { "a": "" } },
        });

        let tree = tree_with_empty(&value);

        assert_eq!(
            section(&tree, "GraphDriver"),
            [
                DataRow::new(0, "Data", "null"),
                DataRow::heading(0, "Options"),
                DataRow::new(1, "a", "\"\""),
            ]
        );
    }

    #[test]
    fn a_nested_list_item_reads_in_one_line_when_it_can() {
        let value = json!({
            "Config": { "Matrix": [[1, 2], [{ "a": 3 }]] },
        });

        let tree = tree(&value);

        assert_eq!(
            section(&tree, "Config"),
            [
                DataRow::heading(0, "Matrix"),
                DataRow::new(1, "1", "1, 2"),
                DataRow::heading(1, "2"),
                DataRow::heading(2, "1"),
                DataRow::new(3, "a", "3"),
            ]
        );
    }

    #[test]
    fn a_value_that_is_not_an_object_has_no_tree() {
        assert_eq!(inspect_tree(&json!([1, 2]), false), None);
        assert_eq!(inspect_tree(&json!("text"), true), None);
        assert_eq!(inspect_tree(&Value::Null, false), None);
    }
}
