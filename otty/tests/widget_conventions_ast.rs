use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::{Item, UseTree, Visibility};

#[test]
fn given_ui_widgets_when_validating_conventions_then_all_modules_comply() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let widgets_dir = manifest_dir.join("src/ui/widgets");
    let mod_rs = widgets_dir.join("mod.rs");

    let mut violations: Vec<String> = Vec::new();

    let mod_source = fs::read_to_string(&mod_rs).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", mod_rs.display())
    });
    let mod_file = syn::parse_file(&mod_source).unwrap_or_else(|err| {
        panic!("failed to parse {}: {err}", mod_rs.display())
    });

    let mut declared_modules = BTreeSet::new();
    for item in &mod_file.items {
        if let Item::Mod(item_mod) = item {
            if is_pub_crate(&item_mod.vis) && item_mod.content.is_none() {
                declared_modules.insert(item_mod.ident.to_string());
            } else {
                violations.push(format!(
                    "{}: module declaration '{}' must be pub(crate) mod <name>;",
                    mod_rs.display(),
                    item_mod.ident
                ));
            }
        }

        if let Item::Use(item_use) = item {
            if use_tree_has_glob(&item_use.tree) {
                violations.push(format!(
                    "{}: wildcard use/import is forbidden",
                    mod_rs.display()
                ));
            }
        }
    }

    let mut fs_modules = BTreeSet::new();
    let entries = fs::read_dir(&widgets_dir).unwrap_or_else(|err| {
        panic!("failed to read dir {}: {err}", widgets_dir.display())
    });
    for entry in entries {
        let entry = entry
            .unwrap_or_else(|err| panic!("failed to read dir entry: {err}"));
        let path = entry.path();
        let file_type = entry.file_type().unwrap_or_else(|err| {
            panic!("failed to read file type for {}: {err}", path.display())
        });

        if file_type.is_dir() {
            violations.push(format!(
                "{}: nested widget directories are forbidden in strict flat layout",
                path.display()
            ));
            continue;
        }

        let Some(ext) = path.extension() else {
            continue;
        };
        if ext != "rs" {
            continue;
        }

        let file_name = path
            .file_name()
            .unwrap_or_else(|| {
                panic!("missing file name for {}", path.display())
            })
            .to_string_lossy()
            .to_string();
        if file_name == "mod.rs" {
            continue;
        }

        let stem = path
            .file_stem()
            .unwrap_or_else(|| panic!("missing stem for {}", path.display()))
            .to_string_lossy()
            .to_string();
        fs_modules.insert(stem);
    }

    if declared_modules != fs_modules {
        violations.push(format!(
            "{}: declared modules {:?} do not match file modules {:?}",
            mod_rs.display(),
            declared_modules,
            fs_modules
        ));
    }

    for module in &declared_modules {
        let file_path = widgets_dir.join(format!("{module}.rs"));
        validate_widget_file(&file_path, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "widget convention violations:\n{}",
        violations.join("\n")
    );
}

fn validate_widget_file(file_path: &Path, violations: &mut Vec<String>) {
    let source = fs::read_to_string(file_path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", file_path.display())
    });
    let file = syn::parse_file(&source).unwrap_or_else(|err| {
        panic!("failed to parse {}: {err}", file_path.display())
    });
    let expected_prefix = file_stem_pascal_case(file_path);

    if source.contains("crate::app::Event") {
        violations.push(format!(
            "{}: direct coupling to crate::app::Event is forbidden",
            file_path.display()
        ));
    }
    if source.contains("crate::state::") {
        violations.push(format!(
            "{}: dependency on crate::state is forbidden in widgets",
            file_path.display()
        ));
    }

    for forbidden in [
        "log::",
        "std::fs::",
        "std::process::Command",
        "tokio::spawn",
        "Task::",
        "iced::Task",
    ] {
        if source.contains(forbidden) {
            violations.push(format!(
                "{}: forbidden side-effect pattern detected: {forbidden}",
                file_path.display()
            ));
        }
    }
    for forbidden in ["Instant::now", ".elapsed("] {
        if source.contains(forbidden) {
            violations.push(format!(
                "{}: forbidden runtime-time pattern detected: {forbidden}",
                file_path.display()
            ));
        }
    }

    for line in source.lines() {
        if line.contains("crate::features::")
            && (line.contains("::event::")
                || line.contains("::state::")
                || line.contains("::model::")
                || line.contains("::storage::")
                || line.contains("::errors::"))
        {
            violations.push(format!(
                "{}: forbidden feature internal import: {line}",
                file_path.display()
            ));
        }
    }

    let mut view_count = 0usize;
    let mut props_count = 0usize;
    let mut event_count = 0usize;
    let mut props_names: Vec<String> = Vec::new();
    let mut event_names: Vec<String> = Vec::new();

    for item in &file.items {
        match item {
            Item::Fn(item_fn) => {
                if item_fn.sig.ident == "view" {
                    if is_pub_crate(&item_fn.vis) {
                        view_count += 1;
                    } else {
                        violations.push(format!(
                            "{}: view must be pub(crate)",
                            file_path.display()
                        ));
                    }
                }
            },
            Item::Struct(item_struct) => {
                if item_struct.ident.to_string().ends_with("WidgetProps") {
                    violations.push(format!(
                        "{}: *WidgetProps suffix is forbidden; use <Widget>Props",
                        file_path.display()
                    ));
                }
                let name = item_struct.ident.to_string();
                if name.ends_with("Props") {
                    props_count += 1;
                    props_names.push(name);
                }
            },
            Item::Enum(item_enum) => {
                if item_enum.ident.to_string().ends_with("WidgetEvent") {
                    violations.push(format!(
                        "{}: *WidgetEvent suffix is forbidden; use <Widget>Event",
                        file_path.display()
                    ));
                }
                let name = item_enum.ident.to_string();
                if name.ends_with("Event") {
                    event_count += 1;
                    event_names.push(name);
                }
            },
            Item::Type(item_type) => {
                if item_type.ident.to_string().ends_with("WidgetProps") {
                    violations.push(format!(
                        "{}: *WidgetProps suffix is forbidden; use <Widget>Props",
                        file_path.display()
                    ));
                }
                if item_type.ident.to_string().ends_with("WidgetEvent") {
                    violations.push(format!(
                        "{}: *WidgetEvent suffix is forbidden; use <Widget>Event",
                        file_path.display()
                    ));
                }
                let name = item_type.ident.to_string();
                if name.ends_with("Event") {
                    event_count += 1;
                    event_names.push(name);
                }
            },
            Item::Use(item_use) => {
                if use_tree_has_glob(&item_use.tree) {
                    violations.push(format!(
                        "{}: wildcard use/import is forbidden",
                        file_path.display()
                    ));
                }
            },
            _ => {},
        }
    }

    if view_count != 1 {
        violations.push(format!(
            "{}: expected exactly one pub(crate) fn view, found {view_count}",
            file_path.display()
        ));
    }

    if props_count != 1 {
        violations.push(format!(
            "{}: expected exactly one *Props type, found {props_count}",
            file_path.display()
        ));
    }

    if event_count != 1 {
        violations.push(format!(
            "{}: expected exactly one *Event contract, found {event_count}",
            file_path.display()
        ));
    }

    for name in props_names {
        if !name.starts_with(&expected_prefix) {
            violations.push(format!(
                "{}: props type '{name}' must start with file prefix '{expected_prefix}'",
                file_path.display()
            ));
        }
    }
    for name in event_names {
        if !name.starts_with(&expected_prefix) {
            violations.push(format!(
                "{}: event contract '{name}' must start with file prefix '{expected_prefix}'",
                file_path.display()
            ));
        }
    }
}

fn file_stem_pascal_case(file_path: &Path) -> String {
    let stem = file_path
        .file_stem()
        .unwrap_or_else(|| panic!("missing stem for {}", file_path.display()))
        .to_string_lossy()
        .to_string();
    snake_to_pascal_case(&stem)
}

fn snake_to_pascal_case(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut pascal = String::new();
            pascal.extend(first.to_uppercase());
            pascal.push_str(chars.as_str());
            pascal
        })
        .collect::<String>()
}

fn use_tree_has_glob(tree: &UseTree) -> bool {
    match tree {
        UseTree::Glob(_) => true,
        UseTree::Group(group) => group.items.iter().any(use_tree_has_glob),
        UseTree::Path(path) => use_tree_has_glob(&path.tree),
        UseTree::Name(_) | UseTree::Rename(_) => false,
    }
}

fn is_pub_crate(vis: &Visibility) -> bool {
    match vis {
        Visibility::Restricted(restricted) => {
            restricted.in_token.is_none() && restricted.path.is_ident("crate")
        },
        _ => false,
    }
}
