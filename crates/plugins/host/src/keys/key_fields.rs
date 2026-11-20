//! JSON documents with their key fields taken out: what a plugin gets of
//! a key file, a keyring entry or a JSON column, `{"$key": n}` in place of
//! each key.

use otty_plugin_manifest::JsonPointer;
use serde_json::{Map, Value};

/// Replaces each text value of `document` at one of `fields` by `{"$key":
/// n}` and gives the JSON pointer of each, `n` being its index. A value at
/// a key field that is not text, such as an object, becomes `null`, so no
/// key can hide inside it.
pub(crate) fn take_key_fields(
    document: &mut Value,
    fields: &[JsonPointer],
) -> Vec<String> {
    let mut pointers = Vec::new();

    take_from(document, "", fields, &mut pointers);

    pointers
}

/// A value that stands for the key at index `index`.
pub(crate) fn key_value(index: usize) -> Value {
    let mut handle = Map::new();
    handle.insert("$key".to_string(), Value::from(index));

    Value::Object(handle)
}

fn take_from(
    value: &mut Value,
    pointer: &str,
    fields: &[JsonPointer],
    pointers: &mut Vec<String>,
) {
    if !pointer.is_empty() && fields.iter().any(|field| field.matches(pointer))
    {
        *value = match value {
            Value::String(_) => {
                pointers.push(pointer.to_string());
                key_value(pointers.len() - 1)
            },
            _ => Value::Null,
        };

        return;
    }

    match value {
        Value::Object(members) => {
            for (name, member) in members.iter_mut() {
                let escaped = name.replace('~', "~0").replace('/', "~1");
                take_from(
                    member,
                    &format!("{pointer}/{escaped}"),
                    fields,
                    pointers,
                );
            }
        },
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                take_from(
                    item,
                    &format!("{pointer}/{index}"),
                    fields,
                    pointers,
                );
            }
        },
        _ => {},
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::{KeyRule, Manifest};
    use serde_json::json;

    use super::*;

    /// The JSON pointers `texts`, as a manifest declares them.
    fn fields(texts: &[&str]) -> Vec<JsonPointer> {
        let quoted: Vec<String> =
            texts.iter().map(|text| format!("\"{text}\"")).collect();
        let manifest = Manifest::parse(&format!(
            r#"
[plugin]
id = "made-up"
name = "Made up"
version = "0.1.0"
wasm = "made-up.wasm"

[api]
agent = "0.1"

[[agent]]
id = "made-up-agent"
name = "Made-up agent"
icon = "agent.svg"

[[agent.permissions.keys]]
files = ["~/.made-up/auth.json"]
fields = [{}]
"#,
            quoted.join(", ")
        ))
        .expect("a valid manifest");

        match &manifest.agents()[0].permissions().keys()[0] {
            KeyRule::JsonFile { fields, .. } => fields.clone(),
            _ => panic!("a key file"),
        }
    }

    #[test]
    fn key_fields_become_handles_in_the_order_they_are_found() {
        let mut document = json!({
            "token": "x.y.z",
            "work": {"key": "ex-made-up-key", "name": "Work"},
            "home": {"key": "ex-other-key"},
        });

        let pointers =
            take_key_fields(&mut document, &fields(&["/*/key", "/token"]));

        assert_eq!(pointers, ["/home/key", "/token", "/work/key"]);
        assert_eq!(
            document,
            json!({
                "token": {"$key": 1},
                "work": {"key": {"$key": 2}, "name": "Work"},
                "home": {"key": {"$key": 0}},
            })
        );
    }

    #[test]
    fn a_key_field_that_is_not_text_is_dropped() {
        let mut document = json!({
            "tokens": {"access_token": "x.y.z", "id_token": "a.b.c"},
            "count": 3,
        });

        let pointers =
            take_key_fields(&mut document, &fields(&["/tokens", "/count"]));

        assert!(pointers.is_empty());
        assert_eq!(document, json!({"tokens": null, "count": null}));
    }

    #[test]
    fn names_with_a_slash_or_a_tilde_are_escaped_and_arrays_are_walked() {
        let mut document = json!({
            "a/b": "ex-made-up-key",
            "list": ["ex-first", "ex-second"],
        });

        let pointers =
            take_key_fields(&mut document, &fields(&["/a~1b", "/list/1"]));

        assert_eq!(pointers, ["/a~1b", "/list/1"]);
        assert_eq!(
            document,
            json!({"a/b": {"$key": 0}, "list": ["ex-first", {"$key": 1}]})
        );
    }
}
